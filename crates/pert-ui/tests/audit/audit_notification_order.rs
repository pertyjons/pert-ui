//! Change delivery preserves request order across components and presentation roots.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
type Log = Rc<RefCell<Vec<String>>>;
type Endpoint = Rc<RefCell<Option<Notifier>>>;
struct Probe {
    tag: &'static str,
    endpoint: Endpoint,
    other: Endpoint,
    log: Log,
}
impl Component for Probe {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        *self.endpoint.borrow_mut() = Some(cx.notifier());
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 60.0)?)
    }
    fn focusable(&self) -> bool {
        true
    }
    fn key_input(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        self.other.borrow().as_ref().unwrap().request();
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.log.borrow_mut().push(self.tag.into());
        Ok(())
    }
}
fn request(endpoint: &Endpoint) {
    endpoint.borrow().as_ref().unwrap().request();
}
fn two_windows() -> (Runtime, WidgetId, NativeWindowId, Endpoint, Endpoint, Log) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let a = Endpoint::default();
    let b = Endpoint::default();
    let log = Log::default();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "a",
                    Probe {
                        tag: "a",
                        endpoint: a.clone(),
                        other: b.clone(),
                        log: log.clone(),
                    },
                ),
                Element::new(
                    "b",
                    Probe {
                        tag: "b",
                        endpoint: b.clone(),
                        other: a.clone(),
                        log: log.clone(),
                    },
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let b_id = ui.find_descendant(&root, "b").unwrap().unwrap();
    let prepared = ui
        .prepare_detach(&b_id, NativeWindowOptions::new("B").unwrap())
        .unwrap();
    let window = ui.commit_detach(prepared).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    let a_id = ui.find_descendant(&root, "a").unwrap().unwrap();
    (ui, a_id, window, a, b, log)
}
#[test]
fn interleaved_requests_deliver_globally_and_restore_selected_window() {
    let (mut ui, _, b_window, a, b, log) = two_windows();
    request(&a);
    request(&b);
    request(&a);
    ui.select_window(&b_window).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*log.borrow(), ["a", "b", "a"]);
    assert_eq!(ui.current_window(), b_window);
}
#[test]
fn input_delivers_other_window_request_before_return() {
    let (mut ui, a_id, _, _, _, log) = two_windows();
    ui.select_window(&ui.primary_window()).unwrap();
    ui.focus(&a_id).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    assert_eq!(*log.borrow(), ["b"]);
}
#[test]
fn hidden_window_delivery_is_suspended_without_blocking_visible_requests() {
    let (mut ui, _, b_window, a, b, log) = two_windows();
    ui.select_window(&b_window).unwrap();
    ui.set_window_visible(false).unwrap();
    ui.select_window(&ui.primary_window()).unwrap();
    request(&a);
    request(&b);
    request(&a);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*log.borrow(), ["a", "a"]);
    assert_eq!(ui.application_deadline(), None);
    ui.select_window(&b_window).unwrap();
    ui.set_window_visible(true).unwrap();
    ui.select_window(&ui.primary_window()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*log.borrow(), ["a", "a", "b"]);
}
#[test]
fn tool_changes_deliver_one_change_per_global_request() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let a = ToolWindowController::new();
    let b = ToolWindowController::new();
    let log = Log::default();
    let layout = |key, controller: &ToolWindowController| {
        let log = log.clone();
        ToolWindowLayout::new(Element::new("body", Label::new("main")))
            .controller(controller)
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(
                ToolWindow::new(key, key, Element::new("panel", Label::new("panel")))
                    .expanded(true),
            )
            .on_change(move |_, change| {
                log.borrow_mut()
                    .push(format!("{}:{}", change.key, change.expanded));
                Ok(())
            })
    };
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("a", layout("a", &a)),
            Element::new("b", layout("b", &b)),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    a.collapse("a").unwrap();
    b.collapse("b").unwrap();
    a.expand("a").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*log.borrow(), ["a:false", "b:false", "a:true"]);
}

#[test]
fn sibling_changes_and_callback_errors_preserve_each_tool_delivery_slot() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let a = ToolWindowController::new();
    let b = ToolWindowController::new();
    let log = Log::default();
    let fail = Rc::new(std::cell::Cell::new(true));
    let a_log = log.clone();
    let a_fail = fail.clone();
    let a_layout = ToolWindowLayout::new(Element::new("main", Label::new("main")))
        .controller(&a)
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(
            ToolWindow::new("a1", "A1", Element::new("body1", Label::new("one"))).expanded(true),
        )
        .window(ToolWindow::new(
            "a2",
            "A2",
            Element::new("body2", Label::new("two")),
        ))
        .on_change(move |_, change| {
            a_log
                .borrow_mut()
                .push(format!("{}:{}", change.key, change.expanded));
            if a_fail.replace(false) {
                Err(RuntimeError::EmptyCommandName)
            } else {
                Ok(())
            }
        });
    let b_log = log.clone();
    let b_layout = ToolWindowLayout::new(Element::new("main", Label::new("main")))
        .controller(&b)
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(ToolWindow::new("b", "B", Element::new("body", Label::new("body"))).expanded(true))
        .on_change(move |_, change| {
            b_log
                .borrow_mut()
                .push(format!("{}:{}", change.key, change.expanded));
            Ok(())
        });
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("a", a_layout), Element::new("b", b_layout)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    a.expand("a2").unwrap();
    b.collapse("b").unwrap();
    a.expand("a1").unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::EmptyCommandName
    );
    assert_eq!(
        *log.borrow(),
        ["a1:false", "a2:true", "b:false", "a2:false", "a1:true"]
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(log.borrow().len(), 5, "failed callbacks must not replay");
}
