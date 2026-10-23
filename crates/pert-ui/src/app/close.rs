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
/// synchronous `KeepOpen`.
#[derive(Debug)]
pub(crate) struct Released {
    focus: Vec<(NativeWindowId, WidgetId)>,
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
/// must not require a failing update to succeed first.
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
        let primary = runtime.primary_window();
        if affected.contains(&primary) {
            runtime.flush_windows(now)?;
        } else {
            let flushed = affected
                .iter()
                .try_for_each(|root| flush_visible(runtime, root, now));
            reselect(runtime, &previous)?;
            flushed?;
        }
    }
    Ok(Released { focus })
}

/// Focuses again, after a synchronous `KeepOpen`, each component admission
/// released, in a root where nothing holds focus after the callback, when the
/// same incarnation is still mounted in that root and the runtime admits it:
/// a `Hidden`, `Unmounted` or `ModalBlocked` refusal leaves focus cleared.
/// The selected window is selected again afterwards.
pub(crate) fn restore_focus(runtime: &mut Runtime, released: Released) -> Result<(), RuntimeError> {
    let previous = runtime.current_window();
    let restored: Result<(), RuntimeError> =
        released.focus.into_iter().try_for_each(|(root, id)| {
            if !runtime.contains_window(&root) {
                return Ok(());
            }
            runtime.select_window(&root)?;
            if runtime.focused().is_some() || runtime.window_of(&id).ok().as_ref() != Some(&root) {
                return Ok(());
            }
            match runtime.focus(&id) {
                Ok(())
                | Err(
                    RuntimeError::Hidden | RuntimeError::Unmounted | RuntimeError::ModalBlocked,
                ) => Ok(()),
                Err(error) => Err(error),
            }
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
