//! Typed cleanup errors survive window retirement and terminal surface removal.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Debug)]
struct CleanupFailure(&'static str);
impl std::fmt::Display for CleanupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for CleanupFailure {}
fn failure(label: &'static str) -> RuntimeError {
    ApplicationError::new(CleanupFailure(label)).into()
}
struct FocusFailure(RuntimeError);
impl Component for FocusFailure {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        Err(self.0.clone())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 30.0)?)
    }
}
fn detached(ui: &mut Runtime, root: &WidgetId) -> NativeWindowId {
    let prepared = ui
        .prepare_detach(root, NativeWindowOptions::new("Detached").unwrap())
        .unwrap();
    let window = ui.commit_detach(prepared).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    ui.select_window(&window).unwrap();
    window
}
#[test]
fn retired_focus_failures_follow_the_surviving_windows_queue_once() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let old = failure("retired focus");
    let surviving = failure("surviving focus");
    let root = ui.mount("focus", FocusFailure(old.clone())).unwrap();
    let window = detached(&mut ui, &root);
    ui.focus(&root).unwrap();
    ui.clear_focus();
    ui.select_window(&ui.primary_window()).unwrap();
    let survivor = ui
        .mount("survivor", FocusFailure(surviving.clone()))
        .unwrap();
    ui.focus(&survivor).unwrap();
    ui.clear_focus();
    ui.unmount(&root).unwrap();
    assert!(!ui.contains_window(&window));
    assert_eq!(ui.select_window(&window), Err(RuntimeError::Unmounted));
    assert_eq!(ui.focus(&root), Err(RuntimeError::Unmounted));
    assert_eq!(ui.flush(ui.now()), Err(surviving));
    assert_eq!(ui.flush(ui.now()), Err(old));
    ui.flush(ui.now()).unwrap();
}
#[test]
fn retired_focus_failures_remain_available_at_shutdown_before_flush() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let error = failure("retired focus");
    let root = ui.mount("focus", FocusFailure(error.clone())).unwrap();
    let _window = detached(&mut ui, &root);
    ui.focus(&root).unwrap();
    ui.clear_focus();
    ui.unmount(&root).unwrap();
    assert!(ui.take_shutdown_errors().is_empty());
    ui.shutdown();
    assert_eq!(ui.take_shutdown_errors(), [error]);
    ui.shutdown();
    assert!(ui.take_shutdown_errors().is_empty());
}
struct SurfaceFailures {
    column: Column,
    terminal: Option<RuntimeError>,
    shown: Rc<Cell<bool>>,
    earlier: Rc<RefCell<Option<RuntimeError>>>,
    removals: Rc<Cell<usize>>,
}
impl Component for SurfaceFailures {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.column.take_children()
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "popup" || self.shown.get()
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::ContextMenu)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        if reason == SurfaceDismissal::OwnerRemoved {
            self.removals.set(self.removals.get() + 1);
            return self.terminal.clone().map_or(Ok(()), Err);
        }
        if reason == SurfaceDismissal::WindowFocusLost
            && let Some(error) = self.earlier.borrow_mut().take()
        {
            return Err(error);
        }
        self.shown.set(false);
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.column.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.column.arrange(cx, size)
    }
}
type SurfaceFixture = (
    WidgetId,
    Rc<RefCell<Option<RuntimeError>>>,
    Rc<Cell<usize>>,
    Rc<Cell<bool>>,
);
fn surface(ui: &mut Runtime, key: &str, terminal: RuntimeError) -> SurfaceFixture {
    surface_with_outcome(ui, key, Some(terminal))
}
fn surface_with_outcome(
    ui: &mut Runtime,
    key: &str,
    terminal: Option<RuntimeError>,
) -> SurfaceFixture {
    let earlier = Rc::new(RefCell::new(None));
    let removals = Rc::new(Cell::new(0));
    let shown = Rc::new(Cell::new(true));
    let root = ui
        .mount_tree(Element::new(
            key,
            SurfaceFailures {
                column: Column::new([
                    Element::new("popup", Button::new("Popup", |_| Ok(()))),
                    Element::new("anchor", Label::new("Surface owner")),
                ]),
                terminal,
                shown: shown.clone(),
                earlier: earlier.clone(),
                removals: removals.clone(),
            },
        ))
        .unwrap();
    (root, earlier, removals, shown)
}
#[test]
fn root_surface_removal_schedules_each_terminal_failure_without_retry() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let a = failure("first removal");
    let b = failure("second removal");
    let (first, _, calls_a, _shown_a) = surface(&mut ui, "first", a.clone());
    ui.flush(ui.now()).unwrap();
    let (second, _, calls_b, _shown_b) = surface(&mut ui, "second", b.clone());
    ui.unmount(&first).unwrap();
    ui.unmount(&second).unwrap();
    assert_eq!(ui.application_deadline(), Some(ui.now()));
    assert_eq!(ui.flush(ui.now()), Err(a));
    assert_eq!(ui.flush(ui.now()), Err(b));
    ui.flush(ui.now()).unwrap();
    assert_eq!(calls_a.get(), 1);
    assert_eq!(calls_b.get(), 1);
    assert_eq!(ui.application_deadline(), None);
}
#[test]
fn retiring_surface_keeps_terminal_fifo_and_earlier_surface_cause() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let terminal = failure("owner removed");
    let prior = failure("window focus lost");
    let (root, earlier, calls, shown) = surface(&mut ui, "root", terminal.clone());
    let _window = detached(&mut ui, &root);
    shown.set(true);
    ui.invalidate(&root, Invalidation::Layout).unwrap();
    ui.flush_windows(ui.now()).unwrap();
    *earlier.borrow_mut() = Some(prior.clone());
    ui.clear_focus();
    assert!(
        earlier.borrow().is_none(),
        "dismissal consumed the armed cause"
    );
    ui.unmount(&root).unwrap();
    assert_eq!(ui.flush(ui.now()), Err(terminal));
    assert_eq!(ui.flush(ui.now()), Err(prior));
    ui.flush(ui.now()).unwrap();
    assert_eq!(calls.get(), 1);
}
#[test]
fn shutdown_retains_prior_surface_cause_and_new_terminal_failure_once() {
    for detached_window in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
            .unwrap();
        let terminal = failure("owner removed");
        let prior = failure("window focus lost");
        let (root, earlier, calls, shown) = surface(&mut ui, "root", terminal.clone());
        if detached_window {
            let _window = detached(&mut ui, &root);
        } else {
            ui.flush(ui.now()).unwrap();
        }
        shown.set(true);
        ui.invalidate(&root, Invalidation::Layout).unwrap();
        ui.flush_windows(ui.now()).unwrap();
        *earlier.borrow_mut() = Some(prior.clone());
        ui.clear_focus();
        assert!(
            earlier.borrow().is_none(),
            "dismissal consumed the armed cause"
        );
        ui.shutdown();
        assert_eq!(ui.take_shutdown_errors(), [prior, terminal]);
        assert_eq!(calls.get(), 1);
        assert!(ui.take_shutdown_errors().is_empty());
    }
}

#[test]
fn root_removal_keeps_an_older_surface_error_due_for_both_terminal_outcomes() {
    for terminal_fails in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
            .unwrap();
        let prior = failure("earlier dismissal");
        let terminal = failure("terminal removal");
        let (root, earlier, calls, _shown) =
            surface_with_outcome(&mut ui, "owner", terminal_fails.then(|| terminal.clone()));
        ui.flush(ui.now()).unwrap();
        *earlier.borrow_mut() = Some(prior.clone());
        ui.clear_focus();
        assert!(earlier.borrow().is_none());
        ui.unmount(&root).unwrap();
        assert_eq!(ui.next_deadline(), Some(ui.now()));
        assert_eq!(ui.application_deadline(), Some(ui.now()));
        if terminal_fails {
            assert_eq!(ui.flush(ui.now()), Err(terminal));
            assert_eq!(ui.next_deadline(), Some(ui.now()));
            assert_eq!(ui.application_deadline(), Some(ui.now()));
        }
        assert_eq!(ui.flush(ui.now()), Err(prior));
        ui.flush(ui.now()).unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(ui.application_deadline(), None);
    }
}

#[test]
fn redock_retries_the_surviving_owner_and_forwards_its_earlier_error() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let prior = failure("earlier focus loss");
    let (root, earlier, calls, shown) = surface(&mut ui, "root", failure("removal"));
    let window = detached(&mut ui, &root);
    shown.set(true);
    ui.invalidate(&root, Invalidation::Layout).unwrap();
    ui.flush_windows(ui.now()).unwrap();
    *earlier.borrow_mut() = Some(prior.clone());
    ui.clear_focus();
    assert!(earlier.borrow().is_none());
    assert!(shown.get(), "failed dismissal leaves the surface pending");
    ui.redock(&window).unwrap();
    assert!(!shown.get(), "movement cleanup retries the surviving owner");
    assert!(!ui.contains_window(&window));
    assert_eq!(ui.window_of(&root).unwrap(), ui.primary_window());
    assert_eq!(calls.get(), 0, "redock retains the mounted owner");
    assert_eq!(ui.flush(ui.now()), Err(prior));
    ui.flush(ui.now()).unwrap();
}
