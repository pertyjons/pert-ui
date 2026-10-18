//! `HistoryNavigator` and `HistoryButtons` contracts: back and forward follow
//! the history and report the entry to the application, tooltips name the
//! target, the history list opens newest first with the current entry marked
//! and moves the history without a new branch, a dropped entry is refused,
//! the offered parts follow constants and bindings, side buttons work over
//! wrapped content, and a new description replaces the callback and offers.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{key, settle, widget};

struct Setup {
    ui: Runtime,
    root: WidgetId,
    history: NavigationHistory<&'static str>,
    restored: Rc<RefCell<Vec<&'static str>>>,
}

fn navigator(
    ui: &Runtime,
    history: &NavigationHistory<&'static str>,
    restored: &Rc<RefCell<Vec<&'static str>>>,
) -> HistoryNavigator<&'static str> {
    let record = restored.clone();
    HistoryNavigator::new(ui, history.clone(), move |_, entry| {
        record.borrow_mut().push(*entry.value());
        Ok(())
    })
    .unwrap()
}

/// A history with three visits, its navigator, and a log of restored places.
fn setup(
    build: impl FnOnce(HistoryNavigator<&'static str>) -> HistoryNavigator<&'static str>,
) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    for (value, label) in [("colors", "Colors"), ("knobs", "Knobs"), ("files", "Files")] {
        history.visit(value, label).unwrap();
    }
    let restored = Rc::new(RefCell::new(Vec::new()));
    let control = build(navigator(&ui, &history, &restored));
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([Element::new("nav", control)]),
        ))
        .unwrap();
    settle(&mut ui);
    Setup {
        ui,
        root,
        history,
        restored,
    }
}

impl Setup {
    fn click(&mut self, key: &str) {
        let id = widget(&self.ui, &self.root, key);
        let [x, y, w, h] = self.ui.bounds(&id).unwrap().unwrap().values();
        let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(center, kind))
                .unwrap();
        }
        settle(&mut self.ui);
    }
    fn current(&self) -> Option<&'static str> {
        self.history.current().get().unwrap().map(|e| *e.value())
    }
    fn semantics(&self, key: &str) -> Semantics {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == key && c.visible)
            .and_then(|c| c.details.semantics)
            .unwrap_or_else(|| panic!("semantics of {key}"))
    }
    fn shown(&self, key: &str) -> bool {
        self.ui
            .inspect()
            .components
            .iter()
            .any(|c| c.key == key && c.visible)
    }
    fn rows(&self) -> Vec<String> {
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.key == "content" && c.details.properties.contains_key("results"))
            .and_then(|c| c.details.semantics.clone())
            .expect("history list")
            .items
            .iter()
            .map(|item| item.name.clone().unwrap_or_default())
            .collect()
    }
    /// The help text the tooltip around `key` shows once it has been
    /// focused long enough for the help to appear.
    fn tooltip(&mut self, key: &str) -> String {
        let id = widget(&self.ui, &self.root, key);
        self.ui.focus(&id).unwrap();
        settle(&mut self.ui);
        let later = self.ui.now() + Duration::from_millis(600);
        self.ui.flush(later).unwrap();
        let snapshot = self.ui.inspect();
        let tip_key = format!("{key}-tip");
        let tip = snapshot
            .components
            .iter()
            .find(|c| c.key == tip_key)
            .map(|c| c.id)
            .expect("tooltip");
        let inside = |mut id: Option<u64>| {
            while let Some(current) = id {
                if current == tip {
                    return true;
                }
                id = snapshot
                    .components
                    .iter()
                    .find(|c| c.id == current)
                    .and_then(|c| c.parent);
            }
            false
        };
        let texts: Vec<String> = snapshot
            .components
            .iter()
            .filter(|c| c.key == "help" && c.visible && inside(c.parent))
            .filter_map(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1, "one visible help label: {texts:?}");
        texts.into_iter().next().unwrap()
    }
}

#[test]
fn back_and_forward_follow_the_history_and_report_the_entry() {
    let mut s = setup(|n| n);
    assert!(!s.semantics("back").disabled);
    assert!(s.semantics("forward").disabled);
    s.click("back");
    assert_eq!(*s.restored.borrow(), ["knobs"]);
    assert_eq!(s.current(), Some("knobs"));
    assert!(!s.semantics("forward").disabled);
    s.click("back");
    assert_eq!(s.current(), Some("colors"));
    assert!(s.semantics("back").disabled);
    s.click("forward");
    assert_eq!(*s.restored.borrow(), ["knobs", "colors", "knobs"]);
    assert_eq!(s.current(), Some("knobs"));
    // A disabled button does nothing.
    s.click("forward");
    s.click("forward");
    assert_eq!(s.current(), Some("files"));
    assert_eq!(s.restored.borrow().len(), 4);
}

#[test]
fn tooltips_name_the_entry_each_button_goes_to() {
    let mut s = setup(|n| n);
    assert_eq!(s.tooltip("back"), "Back to Knobs");
    s.click("back");
    assert_eq!(s.tooltip("forward"), "Forward to Files");
    assert_eq!(s.tooltip("history"), "History");
}

#[test]
fn the_list_shows_entries_newest_first_and_goes_to_the_chosen_one() {
    let mut s = setup(|n| n);
    s.click("back");
    s.click("history");
    assert_eq!(s.semantics("list").name.as_deref(), Some("History"));
    assert_eq!(s.rows(), ["Files", "Knobs", "Colors"]);
    for row in ["entry-0", "entry-1", "entry-2"] {
        assert_eq!(s.semantics(row).description.as_deref(), Some("History"));
    }
    // The current entry carries the detail "Current".
    let details: Vec<String> =
        s.ui.inspect()
            .components
            .into_iter()
            .filter(|c| c.key == "detail" && c.visible)
            .filter_map(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
            .collect();
    // Every row names its kind; the current one also says so.
    assert_eq!(details, ["History", "Current · History", "History"]);
    s.ui.text_input("col").unwrap();
    settle(&mut s.ui);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.current(), Some("colors"));
    assert_eq!(*s.restored.borrow(), ["knobs", "colors"]);
    // Going to an entry keeps the newer ones: forward reaches them again.
    assert!(s.history.can_go_forward().get().unwrap());
    assert_eq!(s.history.state().get().unwrap().entries().len(), 3);
}

#[test]
fn an_entry_dropped_while_the_list_is_open_is_refused() {
    let mut s = setup(|n| n);
    s.click("history");
    s.history.clear().unwrap();
    s.history.visit("other", "Other").unwrap();
    settle(&mut s.ui);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    // The row named an identity the history no longer has; nothing moved.
    assert_eq!(s.current(), Some("other"));
    assert!(s.restored.borrow().is_empty());
    let message =
        s.ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "message" && c.visible)
            .and_then(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            });
    assert_eq!(
        message.as_deref(),
        Some("That place is no longer in the history.")
    );
}

#[test]
fn the_offered_parts_follow_constants_and_bindings() {
    let s = setup(|n| n.list(false));
    assert!(s.shown("back"));
    assert!(!s.shown("history"));
    let width_without_list =
        s.ui.bounds(&widget(&s.ui, &s.root, "nav"))
            .unwrap()
            .unwrap()
            .width();

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit("a", "A").unwrap();
    let offered = ui.signal(false);
    let restored = Rc::new(RefCell::new(Vec::new()));
    let control = navigator(&ui, &history, &restored).buttons(offered.read_only());
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([Element::new("nav", control)]),
        ))
        .unwrap();
    settle(&mut ui);
    let shown = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == key && c.visible)
    };
    let nav = widget(&ui, &root, "nav");
    assert!(!shown(&ui, "back") && !shown(&ui, "forward"));
    assert!(shown(&ui, "history"));
    let narrow = ui.bounds(&nav).unwrap().unwrap().width();
    offered.set(true).unwrap();
    settle(&mut ui);
    assert!(shown(&ui, "back") && shown(&ui, "forward"));
    assert!(ui.bounds(&nav).unwrap().unwrap().width() > narrow);
    assert!(width_without_list > narrow);
    let props = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "nav")
        .unwrap()
        .details;
    assert_eq!(props.properties["buttons"], InspectValue::Bool(true));
    assert_eq!(props.semantics.unwrap().role, SemanticRole::Toolbar);
}

#[test]
fn side_buttons_over_wrapped_content_go_back_and_forward() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit(1, "One").unwrap();
    history.visit(2, "Two").unwrap();
    let enabled = ui.signal(true);
    let restored = Rc::new(RefCell::new(Vec::new()));
    let record = restored.clone();
    let clicks = Rc::new(RefCell::new(0));
    let counted = clicks.clone();
    ui.mount_tree(Element::new(
        "body",
        HistoryButtons::new(
            history.clone(),
            Element::new(
                "button",
                Button::new("Content", move |_| {
                    *counted.borrow_mut() += 1;
                    Ok(())
                }),
            ),
            move |_, entry| {
                record.borrow_mut().push(*entry.value());
                Ok(())
            },
        )
        .enabled(enabled.read_only()),
    ))
    .unwrap();
    settle(&mut ui);
    let point = Point::new(10.0, 10.0).unwrap();
    let side = |ui: &mut Runtime, button| {
        for kind in [PointerKind::Press(button), PointerKind::Release(button)] {
            ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        }
        settle(ui);
    };
    side(&mut ui, PointerButton::Back);
    assert_eq!(*restored.borrow(), [1]);
    side(&mut ui, PointerButton::Forward);
    assert_eq!(*restored.borrow(), [1, 2]);
    // Nowhere to go: nothing is restored. The button ignores side buttons, so
    // its action never ran.
    side(&mut ui, PointerButton::Forward);
    assert_eq!(*restored.borrow(), [1, 2]);
    assert_eq!(*clicks.borrow(), 0);
    enabled.set(false).unwrap();
    settle(&mut ui);
    side(&mut ui, PointerButton::Back);
    assert_eq!(*restored.borrow(), [1, 2]);
    assert_eq!(
        history.current().get().unwrap().map(|e| *e.value()),
        Some(2)
    );
}

#[test]
fn describing_the_control_again_replaces_the_callback_and_offers() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit("colors", "Colors").unwrap();
    history.visit("knobs", "Knobs").unwrap();
    let (first, second, third) = (
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Vec::new())),
        Rc::new(RefCell::new(Vec::new())),
    );
    let before = Element::new("nav", navigator(&ui, &history, &first));
    let after = Element::new("nav", navigator(&ui, &history, &second).buttons(false));
    let fresh = Element::new("nav", navigator(&ui, &history, &third).buttons(false));
    let checked = check_redescription(&mut ui, before, after, fresh).unwrap();
    let mut s = Setup {
        ui,
        root: checked.root,
        history,
        restored: first,
    };
    assert!(!s.shown("back"));
    // The list still opens on the mounted picker and runs the new callback.
    s.click("history");
    s.ui.text_input("col").unwrap();
    settle(&mut s.ui);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert!(s.restored.borrow().is_empty());
    assert!(third.borrow().is_empty());
    assert_eq!(*second.borrow(), ["colors"]);
}

#[test]
fn localized_children_survive_ancestor_redescription_with_retained_identity() {
    let mut s = setup(|nav| nav);
    let before = widget(&s.ui, &s.root, "back");
    let strings = UiStrings::english()
        .with(StringRole::HistoryBack, "Bakåt")
        .unwrap();
    s.ui.ui_settings().set_strings(strings).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.semantics("back").name.as_deref(), Some("Bakåt"));
    s.ui.reconcile_children(
        &s.root,
        vec![Element::new(
            "nav",
            navigator(&s.ui, &s.history, &s.restored),
        )],
    )
    .unwrap();
    settle(&mut s.ui);
    assert_eq!(widget(&s.ui, &s.root, "back"), before);
    assert_eq!(s.semantics("back").name.as_deref(), Some("Bakåt"));
    s.click("back");
    assert_eq!(s.current(), Some("knobs"));
}

#[test]
fn a_new_history_description_uses_only_the_retained_history_for_parts() {
    let mut s = setup(|nav| nav);
    let other = NavigationHistory::new(&s.ui).unwrap();
    other.visit("other", "Other").unwrap();
    let restored = Rc::new(RefCell::new(Vec::new()));
    s.ui.reconcile_children(
        &s.root,
        vec![Element::new("nav", navigator(&s.ui, &other, &restored))],
    )
    .unwrap();
    settle(&mut s.ui);
    assert!(!s.semantics("back").disabled);
    s.click("back");
    assert_eq!(s.current(), Some("knobs"));
    assert_eq!(*restored.borrow(), ["knobs"]);
    assert_eq!(
        other.current().get().unwrap().map(|entry| *entry.value()),
        Some("other")
    );
    assert!(s.restored.borrow().is_empty());
}

#[test]
fn callback_only_redescription_keeps_generated_parts_without_updates() {
    let mut s = setup(|nav| nav);
    s.ui.set_profiling(true).unwrap();
    s.ui.reconcile_children(
        &s.root,
        vec![Element::new(
            "nav",
            navigator(&s.ui, &s.history, &s.restored),
        )],
    )
    .unwrap();
    settle(&mut s.ui);
    let child_updates: Vec<_> =
        s.ui.profile()
            .stats
            .into_iter()
            .filter(|stat| {
                stat.operation == ProfileOperation::Update && stat.key != "app" && stat.key != "nav"
            })
            .collect();
    assert!(child_updates.is_empty(), "{child_updates:?}");
}

#[test]
fn the_history_icons_have_glyphs_in_the_bundled_font() {
    let ui = Runtime::new();
    for name in ["arrow_back", "arrow_forward", "history"] {
        assert!(ui.fonts().material_icon(name).is_ok(), "{name}");
    }
}

#[test]
fn a_fixed_denial_is_never_undone_by_a_later_binding() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit("colors", "Colors").unwrap();
    let allowed = ui.signal(true);
    let restored = Rc::new(RefCell::new(Vec::new()));
    ui.mount_tree(Element::new(
        "nav",
        navigator(&ui, &history, &restored)
            .list(false)
            .list(allowed.read_only()),
    ))
    .unwrap();
    settle(&mut ui);
    let shown = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == key && c.visible)
    };
    assert!(shown(&ui, "back"));
    assert!(!shown(&ui, "history"));

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    history.visit(1, "One").unwrap();
    history.visit(2, "Two").unwrap();
    let allowed = ui.signal(true);
    let restored = Rc::new(RefCell::new(Vec::new()));
    let record = restored.clone();
    ui.mount_tree(Element::new(
        "body",
        HistoryButtons::new(
            history,
            Element::new("text", Label::new("Content")),
            move |_, entry| {
                record.borrow_mut().push(*entry.value());
                Ok(())
            },
        )
        .enabled(false)
        .enabled(allowed.read_only()),
    ))
    .unwrap();
    settle(&mut ui);
    let point = Point::new(10.0, 10.0).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Back),
        PointerKind::Release(PointerButton::Back),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    settle(&mut ui);
    assert!(restored.borrow().is_empty());
}

/// A 100×100 square that logs the side-button transitions it receives and
/// handles them when `handles` is set.
struct SideLog {
    log: Rc<RefCell<Vec<PointerKind>>>,
    handles: bool,
}
impl Component for SideLog {
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([100, 100]))
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn pointer_input(
        &mut self,
        _: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            kind @ (PointerKind::Press(PointerButton::Back | PointerButton::Forward)
            | PointerKind::Release(PointerButton::Back | PointerButton::Forward)) => {
                self.log.borrow_mut().push(kind);
                Ok(if self.handles {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                })
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
}

#[test]
fn side_buttons_pair_each_release_with_a_press_the_control_handled() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let history = NavigationHistory::new(&ui).unwrap();
    for value in [1, 2, 3] {
        history.visit(value, "Place").unwrap();
    }
    let restored = Rc::new(RefCell::new(Vec::new()));
    let record = restored.clone();
    let inner: Rc<RefCell<Vec<PointerKind>>> = Rc::default();
    let outer: Rc<RefCell<Vec<PointerKind>>> = Rc::default();
    let handling: Rc<RefCell<Vec<PointerKind>>> = Rc::default();
    let log = |log: &Rc<RefCell<Vec<PointerKind>>>, handles| SideLog {
        log: log.clone(),
        handles,
    };
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new(
                "wrapped",
                HistoryButtons::new(
                    history.clone(),
                    Element::new(
                        "content",
                        Column::new([
                            Element::new("plain", log(&inner, false)),
                            Element::new("handling", log(&handling, true)),
                        ]),
                    ),
                    move |_, entry| {
                        record.borrow_mut().push(*entry.value());
                        Ok(())
                    },
                ),
            ),
            Element::new("outside", log(&outer, false)),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let at = |x: f32, y: f32| Point::new(x, y).unwrap();
    let (plain, handled_child, outside) = (at(50.0, 50.0), at(50.0, 150.0), at(250.0, 50.0));
    let send = |ui: &mut Runtime, point, kind| {
        let outcome = ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        settle(ui);
        outcome
    };
    // Pressed inside, released over a sibling: the control keeps the release.
    send(&mut ui, plain, PointerKind::Press(PointerButton::Back));
    assert_eq!(*restored.borrow(), [2]);
    send(&mut ui, outside, PointerKind::Release(PointerButton::Back));
    assert!(outer.borrow().is_empty(), "{:?}", outer.borrow());
    // Pressed outside, released inside: not this control's release.
    send(&mut ui, outside, PointerKind::Press(PointerButton::Back));
    let release = send(&mut ui, plain, PointerKind::Release(PointerButton::Back));
    assert_eq!(release, InputOutcome::Ignored);
    assert_eq!(*restored.borrow(), [2]);
    // A descendant that handles the side button keeps it from the control.
    send(
        &mut ui,
        handled_child,
        PointerKind::Press(PointerButton::Forward),
    );
    send(
        &mut ui,
        handled_child,
        PointerKind::Release(PointerButton::Forward),
    );
    assert_eq!(*restored.borrow(), [2]);
    assert_eq!(handling.borrow().len(), 2);
    // The plain descendant saw the press it ignored before the control did.
    assert_eq!(
        inner.borrow().first(),
        Some(&PointerKind::Press(PointerButton::Back))
    );
}

#[test]
fn history_buttons_follow_a_replacement_history_and_forget_a_held_press() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let first = NavigationHistory::new(&ui).unwrap();
    first.visit(1, "One").unwrap();
    first.visit(2, "Two").unwrap();
    let second = NavigationHistory::new(&ui).unwrap();
    second.visit(10, "Ten").unwrap();
    second.visit(20, "Twenty").unwrap();
    let restored = Rc::new(RefCell::new(Vec::new()));
    let buttons = |history: &NavigationHistory<i32>| {
        let record = restored.clone();
        Element::new(
            "body",
            HistoryButtons::new(
                history.clone(),
                Element::new("text", Label::new("Content")),
                move |_, entry| {
                    record.borrow_mut().push(*entry.value());
                    Ok(())
                },
            ),
        )
    };
    let root = ui
        .mount_tree(Element::new("app", Column::new([buttons(&first)])))
        .unwrap();
    settle(&mut ui);
    let point = Point::new(10.0, 10.0).unwrap();
    let send = |ui: &mut Runtime, kind| {
        let outcome = ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        settle(ui);
        outcome
    };
    // The press moves the first history and stays held across the handoff.
    send(&mut ui, PointerKind::Press(PointerButton::Back));
    assert_eq!(*restored.borrow(), [1]);
    ui.reconcile_children(&root, vec![buttons(&second)])
        .unwrap();
    settle(&mut ui);
    // The held press was forgotten: its release is nobody's.
    assert_eq!(
        send(&mut ui, PointerKind::Release(PointerButton::Back)),
        InputOutcome::Ignored
    );
    assert_eq!(*restored.borrow(), [1]);
    // The next press moves the second history; the first stays where it was.
    send(&mut ui, PointerKind::Press(PointerButton::Back));
    send(&mut ui, PointerKind::Release(PointerButton::Back));
    assert_eq!(*restored.borrow(), [1, 10]);
    assert_eq!(
        second.current().get().unwrap().map(|e| *e.value()),
        Some(10)
    );
    assert_eq!(first.current().get().unwrap().map(|e| *e.value()), Some(1));
    // Describing the same history again keeps a held press.
    send(&mut ui, PointerKind::Press(PointerButton::Forward));
    assert_eq!(*restored.borrow(), [1, 10, 20]);
    ui.reconcile_children(&root, vec![buttons(&second)])
        .unwrap();
    settle(&mut ui);
    assert_eq!(
        send(&mut ui, PointerKind::Release(PointerButton::Forward)),
        InputOutcome::Handled
    );
}
