//! Which tab a strip selects when its selection is removed, and which tabs
//! keyboard stepping reaches.
use pert_ui::*;
use std::time::Duration;

fn tab(key: &str) -> Element {
    Tab::new(
        key,
        format!("Tab {key}"),
        Element::new("body", Label::new(format!("Content {key}"))),
    )
    .into_element()
}

fn strip(ui: &mut Runtime, keys: &[&str]) -> WidgetId {
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let tabs = ui
        .mount_tree(Element::new("tabs", Tabs::new([])).children(keys.iter().map(|key| tab(key))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    tabs
}

fn selected(ui: &Runtime) -> Option<InspectValue> {
    ui.inspect()
        .components
        .iter()
        .find(|component| component.key == "tabs")
        .and_then(|component| component.details.properties.get("selected").cloned())
}

fn text(key: &str) -> Option<InspectValue> {
    Some(InspectValue::Text(key.into()))
}

fn select(ui: &mut Runtime, key: &str) {
    let document = format!("pert-ui-workspace 1\nentry \"tabs\" tabs selected=\"{key}\"\n");
    let report = ui
        .restore_workspace(&Workspace::decode(&document).unwrap())
        .unwrap();
    assert!(report.is_complete());
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(ui), text(key));
}

fn press(ui: &mut Runtime, keys: &[KeyCode]) {
    for key in keys {
        ui.key_input(*key, KeyAction::Press).unwrap();
    }
    for key in keys.iter().rev() {
        ui.key_input(*key, KeyAction::Release).unwrap();
    }
    ui.flush(ui.now()).unwrap();
}

#[test]
fn removing_the_selection_while_inserting_selects_its_following_neighbour_by_key() {
    let mut ui = Runtime::new();
    let tabs = strip(&mut ui, &["a", "b", "c", "d"]);
    select(&mut ui, "c");
    // `c` goes and `x` arrives first: `c`'s old position in the new order is
    // `b`, but its following neighbour is `d`.
    ui.reconcile_children(&tabs, ["x", "a", "b", "d"].into_iter().map(tab).collect())
        .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(&ui), text("d"));
    // Without a following neighbour, the nearest preceding one is chosen.
    ui.reconcile_children(&tabs, ["x", "a", "b"].into_iter().map(tab).collect())
        .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(&ui), text("b"));
}

#[test]
fn keyboard_stepping_passes_a_detached_tab() {
    let mut ui = Runtime::new();
    let tabs = strip(&mut ui, &["a", "b", "c"]);
    let views = ui.children(&tabs).unwrap();
    let primary = ui.primary_window();
    // Only a shown view can be detached.
    select(&mut ui, "b");
    let prepared = ui
        .prepare_detach(&views[1], NativeWindowOptions::new("b").unwrap())
        .unwrap();
    let _window = ui.commit_detach(prepared).unwrap();
    ui.select_window(&primary).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(&ui), text("c"), "detaching the selection moves it");
    // A click on the first title selects and focuses the strip.
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(8.0, 18.0).unwrap(), kind))
            .unwrap();
    }
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(&ui), text("a"));

    press(&mut ui, &[KeyCode::ArrowRight]);
    assert_eq!(selected(&ui), text("c"), "Right passes the detached tab");
    press(&mut ui, &[KeyCode::ArrowLeft]);
    assert_eq!(selected(&ui), text("a"), "and so does Left");
    press(&mut ui, &[KeyCode::ControlLeft, KeyCode::Tab]);
    assert_eq!(selected(&ui), text("c"), "and Ctrl+Tab");
    press(
        &mut ui,
        &[KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::Tab],
    );
    assert_eq!(selected(&ui), text("a"), "and Ctrl+Shift+Tab");
    press(&mut ui, &[KeyCode::End]);
    assert_eq!(selected(&ui), text("c"));
    // With the last tab detached as well, End stops at the last docked one.
    let prepared = ui
        .prepare_detach(&views[2], NativeWindowOptions::new("c").unwrap())
        .unwrap();
    let _window = ui.commit_detach(prepared).unwrap();
    ui.select_window(&primary).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(selected(&ui), text("a"), "detaching the selection moves it");
    press(&mut ui, &[KeyCode::End]);
    assert_eq!(selected(&ui), text("a"));
}

#[test]
fn selected_chooses_the_initial_tab_and_a_redescription_keeps_the_selection() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = seen.clone();
    let strip = |key: &str, log: std::rc::Rc<std::cell::RefCell<Vec<Option<String>>>>| {
        Element::new(
            "tabs",
            Tabs::new(["a", "b", "c"].into_iter().map(|key| {
                Tab::new(
                    key,
                    format!("Tab {key}"),
                    Element::new("body", Label::new(format!("Content {key}"))),
                )
            }))
            .selected(key)
            .on_select(move |_, change| {
                log.borrow_mut().push(change.selected.clone());
                Ok(())
            }),
        )
    };
    let root = ui
        .mount_tree(Element::new("root", Column::new([strip("b", log.clone())])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), text("b"));
    assert!(
        seen.borrow().is_empty(),
        "the initial selection is not a change"
    );
    let tabs = ui.find_descendant(&root, "tabs").unwrap().unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == "b" && c.visible)
    );
    ui.reconcile_children(&root, vec![strip("c", log.clone())])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), text("b"));
    assert!(seen.borrow().is_empty());
    assert_eq!(ui.children(&tabs).unwrap().len(), 3);
    // The workspace records the initial selection before the first update.
    let mut fresh = Runtime::new();
    fresh
        .set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    fresh.mount_tree(strip("c", log)).unwrap();
    let document = fresh.capture_workspace().unwrap().encode();
    assert!(document.contains("selected=\"c\""), "{document}");
}

#[test]
fn an_unknown_initial_selection_is_rejected_or_falls_back_for_raw_children() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let described = Tabs::new([Tab::new(
        "a",
        "Tab a",
        Element::new("body", Label::new("Content a")),
    )])
    .selected("missing");
    assert_eq!(
        ui.mount_tree(Element::new("tabs", described)).err(),
        Some(RuntimeError::Layout(LayoutError::UnknownKey))
    );
    assert!(ui.inspect().components.is_empty());
    // Raw children cannot be checked when the strip is described: the key
    // falls back to the first tab at the first update.
    ui.mount_tree(
        Element::new("tabs", Tabs::new([]).selected("missing")).children([tab("a"), tab("b")]),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(selected(&ui), text("a"));
}
