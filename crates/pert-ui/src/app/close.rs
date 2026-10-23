//! Close admission shared by the native runner and [`HeadlessApp`]: the steps
//! that run before and after [`App::on_close`]. The native runner keeps what is
//! a state machine (coalescing repeated requests, the pending decision, the
//! receipts and applying the decision).
//!
//! [`HeadlessApp`]: crate::testing::HeadlessApp
//! [`App::on_close`]: crate::App::on_close
use crate::{InspectionQuery, NativeWindowId, Runtime, RuntimeError, WidgetId};
use std::time::Duration;

/// The focus admission released, per root, for restoring it after a
/// synchronous `KeepOpen`, and whether admission was allowed to settle.
#[derive(Debug)]
pub(crate) struct Released {
    focus: Vec<(NativeWindowId, WidgetId)>,
    affected: Vec<NativeWindowId>,
    settled: bool,
    now: Duration,
}

/// The roots an accepted close of `window` discards: a detached window
/// discards its own content; the main window ends the application and with it
/// every window.
pub(crate) fn affected_roots(runtime: &Runtime, window: &NativeWindowId) -> Vec<NativeWindowId> {
    if *window == runtime.primary_window() {
        runtime.native_windows()
    } else {
        vec![window.clone()]
    }
}

/// Releases focus in each of `affected` through `Runtime::clear_focus`, as a
/// window deactivation does, so each editor applies its focus-loss policy
/// before the close policy runs; then, when `settle`, flushes them at `now`:
/// every root through `flush_windows` for the main window, or the detached
/// root alone when it is visible. A failed `focus_lost` of an affected root is
/// returned by that flush. The selected window is selected again afterwards.
///
/// `settle` is false while any root is in component-error backoff: settling
/// could retry a backing-off root's update before its deadline, and closing
/// must not require a failing update to succeed first. Nothing is then
/// restored after a `KeepOpen` either, since focusing can flush.
pub(crate) fn prepare_close(
    runtime: &mut Runtime,
    affected: &[NativeWindowId],
    settle: bool,
    now: Duration,
) -> Result<Released, RuntimeError> {
    let previous = runtime.current_window();
    let mut focus = Vec::new();
    let released: Result<(), RuntimeError> = affected.iter().try_for_each(|root| {
        runtime.select_window(root)?;
        if let Some(id) = runtime.focused() {
            focus.push((root.clone(), id));
        }
        runtime.clear_focus();
        Ok(())
    });
    reselect(runtime, &previous)?;
    released?;
    if settle {
        settle_roots(runtime, affected, now)?;
    }
    Ok(Released {
        focus,
        affected: affected.to_vec(),
        settled: settle,
        now,
    })
}

/// Flushes `affected` at `now`: every root through `flush_windows` when the
/// main window is among them, otherwise each visible root alone. The
/// selected window is selected again afterwards.
fn settle_roots(
    runtime: &mut Runtime,
    affected: &[NativeWindowId],
    now: Duration,
) -> Result<(), RuntimeError> {
    if affected.contains(&runtime.primary_window()) {
        runtime.flush_windows(now)?;
        return Ok(());
    }
    let previous = runtime.current_window();
    let flushed = affected
        .iter()
        .try_for_each(|root| flush_visible(runtime, root, now));
    reselect(runtime, &previous)?;
    flushed
}

/// Focuses again, after a synchronous `KeepOpen`, each component admission
/// released, in a root where nothing holds focus after the callback, when the
/// same incarnation is still in that root and [`Runtime::can_focus`] admits
/// it: a hidden, unmounted, unfocusable (such as disabled) or modal-blocked
/// component leaves focus cleared. The affected roots are settled first, so
/// what the callback changed, such as disabling the component, is applied
/// before the check. Settling and focusing flush, so nothing is restored, and
/// nothing flushed, when admission did not settle (a root was in
/// component-error backoff). The selected window is selected again
/// afterwards.
pub(crate) fn restore_focus(runtime: &mut Runtime, released: Released) -> Result<(), RuntimeError> {
    if !released.settled || released.focus.is_empty() {
        return Ok(());
    }
    // The policy may have flushed at a later time; the runtime rejects an
    // earlier one.
    let now = released.now.max(runtime.now());
    settle_roots(runtime, &released.affected, now)?;
    let previous = runtime.current_window();
    let restored: Result<(), RuntimeError> =
        released.focus.into_iter().try_for_each(|(root, id)| {
            if !runtime.contains_window(&root) {
                return Ok(());
            }
            runtime.select_window(&root)?;
            if runtime.focused().is_some()
                || runtime.window_of(&id).ok().as_ref() != Some(&root)
                || !runtime.can_focus(&id)?
            {
                return Ok(());
            }
            runtime.focus(&id)
        });
    reselect(runtime, &previous)?;
    restored
}

/// Flushes `root` when the runtime shows it; a hidden root keeps its queued
/// errors and notifications until it is shown, as any hidden root does.
fn flush_visible(
    runtime: &mut Runtime,
    root: &NativeWindowId,
    now: Duration,
) -> Result<(), RuntimeError> {
    let shown = runtime
        .query(&InspectionQuery::new().windows())
        .windows
        .is_some_and(|windows| {
            windows
                .iter()
                .any(|window| window.id == root.id() && window.presentable)
        });
    if shown {
        runtime.select_window(root)?;
        runtime.flush(now)?;
    }
    Ok(())
}

/// Selects `window` again when it is still open.
fn reselect(runtime: &mut Runtime, window: &NativeWindowId) -> Result<(), RuntimeError> {
    if runtime.contains_window(window) {
        runtime.select_window(window)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Column, Component, Constraints, Element, MeasureContext, ReadSignal, Size, TextField,
        UpdateContext, Viewport,
    };
    use std::{cell::Cell, rc::Rc};

    /// Counts its updates; reading `tick` makes a write to it pending work.
    struct Counted {
        tick: ReadSignal<u32>,
        updates: Rc<Cell<u32>>,
    }
    impl Component for Counted {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            cx.read(&self.tick)?;
            self.updates.set(self.updates.get() + 1);
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::from([20, 20]))
        }
    }

    struct Form {
        runtime: Runtime,
        field: WidgetId,
        text: crate::Signal<String>,
        tick: crate::Signal<u32>,
        updates: Rc<Cell<u32>>,
    }
    /// A drafting field holding "draft" with focus, and an update pending.
    fn form() -> Result<Form, RuntimeError> {
        let mut runtime = Runtime::new();
        runtime.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let text = runtime.signal(String::new());
        let tick = runtime.signal(0_u32);
        let updates = Rc::new(Cell::new(0));
        let root = runtime.mount_tree(Element::new(
            "form",
            Column::new([
                Element::new("field", TextField::new(text.clone()).max_length(Some(50))),
                Element::new(
                    "counted",
                    Counted {
                        tick: tick.read_only(),
                        updates: updates.clone(),
                    },
                ),
            ]),
        ))?;
        runtime.flush(Duration::ZERO)?;
        let field = runtime
            .find(&root, &["field"])?
            .ok_or(RuntimeError::Unmounted)?;
        runtime.focus(&field)?;
        runtime.text_input("draft")?;
        tick.set(1)?;
        Ok(Form {
            runtime,
            field,
            text,
            tick,
            updates,
        })
    }

    /// While a window backs off, admission releases focus, so drafts are
    /// saved, but flushes nothing, and a kept window gets nothing back:
    /// focusing could flush too.
    #[test]
    fn without_settling_admission_flushes_and_restores_nothing() -> Result<(), RuntimeError> {
        let Form {
            mut runtime,
            text,
            updates,
            ..
        } = form()?;
        let before = (updates.get(), runtime.now());
        let affected = affected_roots(&runtime, &runtime.primary_window());
        let released = prepare_close(&mut runtime, &affected, false, Duration::from_secs(1))?;
        assert_eq!(text.get()?, "draft");
        assert_eq!(runtime.focused(), None);
        restore_focus(&mut runtime, released)?;
        assert_eq!(runtime.focused(), None);
        assert_eq!((updates.get(), runtime.now()), before, "nothing flushed");
        Ok(())
    }

    /// Settled, the same admission flushes and a kept window gets its
    /// editor's focus back.
    #[test]
    fn settling_admission_flushes_and_restores_the_editor() -> Result<(), RuntimeError> {
        let Form {
            mut runtime,
            field,
            text,
            tick,
            updates,
        } = form()?;
        let before = updates.get();
        let affected = affected_roots(&runtime, &runtime.primary_window());
        let released = prepare_close(&mut runtime, &affected, true, Duration::from_secs(1))?;
        assert_eq!(text.get()?, "draft");
        assert_eq!(updates.get(), before + 1);
        assert_eq!(runtime.now(), Duration::from_secs(1));
        // The policy's own change is applied before the focus comes back.
        tick.set(2)?;
        restore_focus(&mut runtime, released)?;
        assert_eq!(updates.get(), before + 2);
        assert_eq!(runtime.focused(), Some(field));
        Ok(())
    }

    /// A policy that flushes at a later time does not make restoration flush
    /// at admission's earlier time, which the runtime would reject.
    #[test]
    fn restoration_after_a_policy_that_advanced_the_clock_still_restores()
    -> Result<(), RuntimeError> {
        let Form {
            mut runtime, field, ..
        } = form()?;
        let affected = affected_roots(&runtime, &runtime.primary_window());
        let released = prepare_close(&mut runtime, &affected, true, Duration::from_secs(1))?;
        runtime.flush(Duration::from_secs(2))?;
        restore_focus(&mut runtime, released)?;
        assert_eq!(runtime.focused(), Some(field));
        assert_eq!(runtime.now(), Duration::from_secs(2));
        Ok(())
    }
}
