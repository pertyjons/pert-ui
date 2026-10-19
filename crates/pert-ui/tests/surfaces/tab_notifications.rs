//! `Tabs::on_select`: every selection change is reported once, after it took
//! effect, with its origin, in order, from an event phase of the flush.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type Log = Rc<RefCell<Vec<(Option<String>, Option<String>, ChangeOrigin)>>>;

fn tab(key: &str) -> Tab {
    Tab::new(
        key,
        format!("Tab {key}"),
        Element::new("body", Label::new(format!("Content {key}"))),
    )
}

fn recording(
    log: &Log,
) -> impl FnMut(&mut EventContext, &SelectionChange) -> Result<(), RuntimeError> + 'static {
    let log = log.clone();
    move |_, change| {
        log.borrow_mut().push((
            change.previous.clone(),
            change.selected.clone(),
            change.origin,
        ));
        Ok(())
    }
}

fn some(key: &str) -> Option<String> {
    Some(key.to_owned())
}

fn mount(ui: &mut Runtime, strip: Tabs) -> WidgetId {
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let tabs = ui.mount_tree(Element::new("tabs", strip)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    tabs
}

fn at(ui: &Runtime, root: &WidgetId, path: &[&str]) -> WidgetId {
    ui.find(root, path).unwrap().unwrap()
}

fn restore(ui: &mut Runtime, path: &str, key: &str) {
    let document = format!("pert-ui-workspace 1\nentry \"{path}\" tabs selected=\"{key}\"\n");
    let report = ui
        .restore_workspace(&Workspace::decode(&document).unwrap())
        .unwrap();
    assert!(report.is_complete(), "{report:?}");
}

fn click(ui: &mut Runtime, x: f32) {
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, 18.0).unwrap(), kind))
            .unwrap();
    }
}

#[test]
fn each_origin_is_reported_and_the_initial_selection_is_not() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let tabs = mount(
        &mut ui,
        Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(recording(&log)),
    );
    assert!(
        log.borrow().is_empty(),
        "the initial selection is not a change"
    );

    // Keys on the focused strip and a click on the first title.
    ui.focus(&tabs).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    ui.flush(ui.now()).unwrap();
    click(&mut ui, 8.0);
    ui.flush(ui.now()).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
    ui.flush(ui.now()).unwrap();
    // Selecting by key, and asking for the tab already shown.
    assert_eq!(ui.reveal_child(&tabs, "a").unwrap(), ChildReveal::Changed);
    assert_eq!(
        ui.reveal_child(&tabs, "a").unwrap(),
        ChildReveal::AlreadyVisible
    );
    ui.flush(ui.now()).unwrap();
    // Revealing a descendant of an unselected tab.
    let body = at(&ui, &tabs, &["b", "body"]);
    assert!(ui.reveal(&body).unwrap().is_revealed());
    // Workspace restoration.
    restore(&mut ui, "tabs", "c");
    ui.flush(ui.now()).unwrap();
    // Removing the selection.
    ui.reconcile_children(
        &tabs,
        vec![tab("a").into_element(), tab("b").into_element()],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();

    assert_eq!(
        *log.borrow(),
        [
            (some("a"), some("b"), ChangeOrigin::User),
            (some("b"), some("a"), ChangeOrigin::User),
            (some("a"), some("c"), ChangeOrigin::User),
            (some("c"), some("a"), ChangeOrigin::Reveal),
            (some("a"), some("b"), ChangeOrigin::Reveal),
            (some("b"), some("c"), ChangeOrigin::Restore),
            (some("c"), some("b"), ChangeOrigin::Redescription),
        ]
    );
}

#[test]
fn detaching_and_redocking_the_selection_are_presentation_changes() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let tabs = mount(
        &mut ui,
        Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(recording(&log)),
    );
    let primary = ui.primary_window();
    let views = ui.children(&tabs).unwrap();
    let prepared = ui
        .prepare_detach(&views[0], NativeWindowOptions::new("a").unwrap())
        .unwrap();
    let window = ui.commit_detach(prepared).unwrap();
    ui.select_window(&primary).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        *log.borrow(),
        [(some("a"), some("b"), ChangeOrigin::Presentation)]
    );
    // Pressing a detached title activates its window and changes nothing.
    click(&mut ui, 8.0);
    ui.flush(ui.now()).unwrap();
    assert_eq!(log.borrow().len(), 1);
    ui.redock(&window).unwrap();
    ui.select_window(&primary).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        log.borrow()[1..],
        [(some("b"), some("a"), ChangeOrigin::Presentation)]
    );
}

#[test]
fn several_changes_in_one_flush_arrive_in_order_and_after_they_took_effect() {
    let log = Log::default();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Runtime::new();
    let seen = observed.clone();
    let record = recording(&log);
    let mut record = record;
    let tabs = mount(
        &mut ui,
        Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(move |cx, change| {
            // The strip already shows the new selection when the callback runs.
            seen.borrow_mut().push(cx.id());
            record(cx, change)
        }),
    );
    // Input entry points and `Runtime::reveal_child` settle in a flush of
    // their own first, which delivers; restoration and a new description
    // wait for the next flush.
    restore(&mut ui, "tabs", "b");
    ui.reconcile_children(
        &tabs,
        vec![tab("a").into_element(), tab("c").into_element()],
    )
    .unwrap();
    assert!(log.borrow().is_empty(), "nothing runs before the flush");
    assert!(
        ui.next_deadline().is_some(),
        "pending notifications keep the runtime due"
    );
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        *log.borrow(),
        [
            (some("a"), some("b"), ChangeOrigin::Restore),
            (some("b"), some("c"), ChangeOrigin::Redescription),
        ]
    );
    assert!(observed.borrow().iter().all(|id| *id == tabs));
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn a_callback_may_write_bindings_that_the_same_flush_settles() {
    let mut ui = Runtime::new();
    let page = ui.signal(String::from("none"));
    let write = page.clone();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new(
                    "tabs",
                    Tabs::new([tab("a"), tab("b")]).on_select(move |_, change| {
                        write.set(change.selected.clone().unwrap_or_default())?;
                        Ok(())
                    }),
                ),
                Element::new("page", Label::new(page.read_only())),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tabs = at(&ui, &root, &["tabs"]);
    ui.reveal_child(&tabs, "b").unwrap();
    ui.flush(ui.now()).unwrap();
    let text = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "page")
        .and_then(|component| component.details.properties.get("text").cloned());
    assert_eq!(text, Some(InspectValue::Text("b".into())));
    assert_eq!(
        ui.next_deadline(),
        None,
        "the write settled in the same flush"
    );
}

/// Writes `value` into `target` from its first notification delivery.
struct WriteOnDelivery {
    target: Signal<String>,
    value: &'static str,
    notifier: Option<Notifier>,
}
impl Component for WriteOnDelivery {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.notifier.is_none() {
            let notifier = cx.notifier();
            notifier.request();
            self.notifier = Some(notifier);
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.target.set(self.value.to_owned())
    }
}

#[test]
fn a_callback_write_reaches_derived_bindings_in_the_same_flush() {
    let mut ui = Runtime::new();
    let page = ui.signal(String::from("none"));
    let shown = page.map(|key| format!("Page {key}")).unwrap();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "writer",
                WriteOnDelivery {
                    target: page.clone(),
                    value: "b",
                    notifier: None,
                },
            ),
            Element::new("page", Label::new(shown)),
        ]),
    ))
    .unwrap();
    // One flush delivers the notification and settles what its write changed.
    ui.flush(Duration::ZERO).unwrap();
    let text = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "page")
        .and_then(|component| component.details.properties.get("text").cloned());
    assert_eq!(text, Some(InspectValue::Text("Page b".into())));
    assert_eq!(
        ui.next_deadline(),
        None,
        "the derived binding settled in the same flush"
    );
}

#[test]
fn a_failing_callback_keeps_the_change_delivers_the_rest_and_fails_the_flush() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let mut record = recording(&log);
    let tabs = mount(
        &mut ui,
        Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(move |cx, change| {
            record(cx, change)?;
            if change.selected.as_deref() == Some("b") {
                Err(RuntimeError::Unmounted)
            } else {
                Ok(())
            }
        }),
    );
    restore(&mut ui, "tabs", "b");
    ui.reconcile_children(
        &tabs,
        vec![tab("a").into_element(), tab("c").into_element()],
    )
    .unwrap();
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::Unmounted));
    assert_eq!(log.borrow().len(), 2, "the later change is still delivered");
    let selected = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "tabs")
        .and_then(|component| component.details.properties.get("selected").cloned());
    assert_eq!(selected, Some(InspectValue::Text("c".into())), "not undone");
    ui.flush(ui.now()).unwrap();
    assert_eq!(log.borrow().len(), 2, "nothing is delivered twice");
}

#[test]
fn a_strip_removed_before_delivery_reports_nothing() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let tabs = mount(
        &mut ui,
        Tabs::new([tab("a"), tab("b")]).on_select(recording(&log)),
    );
    restore(&mut ui, "tabs", "b");
    ui.unmount(&tabs).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(log.borrow().is_empty());
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn a_hidden_strip_still_reports_and_restoring_before_the_first_update_does_not() {
    let log = Log::default();
    let mut ui = Runtime::new();
    let outer = Tabs::new([
        Tab::new(
            "first",
            "First",
            Element::new(
                "inner",
                Tabs::new([tab("a"), tab("b")]).on_select(recording(&log)),
            ),
        ),
        tab("second"),
    ]);
    let outer = mount(&mut ui, outer);
    let inner = at(&ui, &outer, &["first", "inner"]);
    // Hide the inner strip, then restore its selection.
    ui.reveal_child(&outer, "second").unwrap();
    ui.flush(ui.now()).unwrap();
    restore(&mut ui, "tabs/first/inner", "b");
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        *log.borrow(),
        [(some("a"), some("b"), ChangeOrigin::Restore)]
    );
    assert!(ui.children(&inner).is_ok());

    // A strip restored before its first update starts there silently.
    let log = Log::default();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let _tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b")]).on_select(recording(&log)),
        ))
        .unwrap();
    restore(&mut ui, "tabs", "b");
    ui.flush(Duration::ZERO).unwrap();
    assert!(log.borrow().is_empty());
}

#[test]
fn describing_again_replaces_the_callback_and_without_one_discards_pending_changes() {
    let first = Log::default();
    let second = Log::default();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "tabs",
                Tabs::new([tab("a"), tab("b")]).on_select(recording(&first)),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    restore(&mut ui, "root/tabs", "b");
    // Replaced before delivery: the pending change goes to the new callback.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b")]).on_select(recording(&second)),
        )],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(first.borrow().is_empty());
    assert_eq!(
        *second.borrow(),
        [(some("a"), some("b"), ChangeOrigin::Restore)]
    );
    // Without a callback a pending change is discarded.
    restore(&mut ui, "root/tabs", "a");
    ui.reconcile_children(
        &root,
        vec![Element::new("tabs", Tabs::new([tab("a"), tab("b")]))],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(second.borrow().len(), 1);
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn changes_of_two_strips_arrive_in_the_order_they_happened() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let strip = |name: &'static str| {
        let log = log.clone();
        Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(move |_, change| {
            log.borrow_mut()
                .push((name, change.selected.clone(), change.origin));
            Ok(())
        })
    };
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("first", strip("first")),
                Element::new("second", strip("second")),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // First restores, then second, then first changes again: three changes
    // before one flush delivers them.
    let document = concat!(
        "pert-ui-workspace 1\n",
        "entry \"root/first\" tabs selected=\"b\"\n",
        "entry \"root/second\" tabs selected=\"b\"\n",
    );
    let report = ui
        .restore_workspace(&Workspace::decode(document).unwrap())
        .unwrap();
    assert!(report.is_complete(), "{report:?}");
    let first = at(&ui, &root, &["first"]);
    ui.reconcile_children(
        &first,
        vec![tab("a").into_element(), tab("c").into_element()],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        *log.borrow(),
        [
            ("first", some("b"), ChangeOrigin::Restore),
            ("second", some("b"), ChangeOrigin::Restore),
            ("first", some("c"), ChangeOrigin::Redescription),
        ]
    );
}
