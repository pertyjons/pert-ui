//! Reordering a `Tabs` strip by drag and by keyboard, through the facade.
//!
//! The strip is a drag source and its own drop target, built on the session
//! pinned in `drag_and_drop.rs`. These pin what it adds: a keyed move handed to
//! the application, which owns the order; the insertion line; auto-scroll over
//! the overflow arrows; the keyboard equivalent; and how reordering combines
//! with detaching.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

/// The application's tab order and the edits it made, one per drop.
#[derive(Default)]
struct Model {
    order: Vec<String>,
    edits: Vec<String>,
    refuse: bool,
}
impl Model {
    fn apply(&mut self, drop: &ListDrop<'_>) -> bool {
        if self.refuse {
            return false;
        }
        self.edits
            .push(format!("{:?} -> {:?}", drop.items(), drop.position()));
        let moved = drop.items().to_vec();
        let boundary = match drop.position() {
            DropPosition::Before(key) => self.order.iter().position(|k| k == key),
            DropPosition::After(key) => self.order.iter().position(|k| k == key).map(|i| i + 1),
            _ => None,
        }
        .unwrap_or(self.order.len());
        let leaving = self.order[..boundary]
            .iter()
            .filter(|key| moved.contains(key))
            .count();
        self.order.retain(|key| !moved.contains(key));
        self.order
            .splice(boundary - leaving..boundary - leaving, moved);
        true
    }
}

struct Stage {
    ui: Runtime,
    tabs: WidgetId,
    model: Rc<RefCell<Model>>,
}

fn tab(key: &str) -> Element {
    Tab::new(
        key,
        format!("Tab {key}"),
        Element::new("body", Label::new(format!("Content {key}"))),
    )
    .into_element()
}

fn stage(names: &[&str], width: f32, detachable: bool) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 300.0, 1.0).unwrap())
        .unwrap();
    let model = Rc::new(RefCell::new(Model {
        order: names.iter().map(|name| (*name).to_owned()).collect(),
        ..Model::default()
    }));
    let edits = model.clone();
    let strip = Tabs::new([])
        .detachable(detachable)
        .reorderable(move |_, drop| Ok(edits.borrow_mut().apply(drop)));
    let tabs = ui
        .mount_tree(Element::new("tabs", strip).children(names.iter().map(|name| tab(name))))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    Stage { ui, tabs, model }
}

impl Stage {
    fn pointer(&mut self, x: f32, y: f32, kind: PointerKind) {
        self.ui
            .pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
    fn press(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Press(PointerButton::Primary));
    }
    fn release(&mut self, x: f32, y: f32) {
        self.pointer(x, y, PointerKind::Release(PointerButton::Primary));
    }
    /// Describes the tabs again in the model's order, as the application does
    /// after its edit.
    fn describe(&mut self) {
        let order = self.model.borrow().order.clone();
        self.ui
            .reconcile_children(&self.tabs, order.iter().map(|key| tab(key)).collect())
            .unwrap();
        self.ui.flush(self.ui.now()).unwrap();
    }
    fn edits(&self) -> Vec<String> {
        std::mem::take(&mut self.model.borrow_mut().edits)
    }
    fn property(&self, name: &str) -> Option<InspectValue> {
        self.ui
            .inspect()
            .components
            .iter()
            .find(|component| component.key == "tabs")
            .and_then(|component| component.details.properties.get(name).cloned())
    }
    /// Title text rectangles in the strip, left to right.
    fn titles(&mut self) -> Vec<Rect> {
        let wanted = Palette::dark().color(ColorRole::OnTab);
        let scene = self.ui.paint().unwrap();
        let mut titles: Vec<Rect> = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::DrawMask { rect, color, .. } if *color == wanted => Some(*rect),
                _ => None,
            })
            .filter(|rect| rect.y() + rect.height() <= 36.0)
            .collect();
        titles.sort_by(|a, b| a.x().total_cmp(&b.x()));
        titles
    }
    /// Thin Primary rectangles spanning the strip from its top: the insertion
    /// line, and neither the selected title's underline nor its focus outline.
    fn insertion_lines(&mut self) -> Vec<Rect> {
        let wanted = Palette::dark().color(ColorRole::Primary);
        let scene = self.ui.paint().unwrap();
        scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == wanted => Some(*rect),
                _ => None,
            })
            .filter(|rect| rect.width() <= 4.0 && rect.y() == 0.0 && rect.height() > 4.0)
            .collect()
    }
}

fn centre_y() -> f32 {
    15.0
}

#[test]
fn dragging_a_title_hands_the_application_one_keyed_move() {
    let mut stage = stage(&["a", "b", "c"], 800.0, false);
    let views = stage.ui.children(&stage.tabs).unwrap();
    let titles = stage.titles();
    // From the first title to the right half of the last: after it.
    let from = (titles[0].x() + 2.0, centre_y());
    let to = titles[2].x() + titles[2].width() - 1.0;
    stage.press(from.0, from.1);
    stage.pointer(from.0 + 6.0, from.1, PointerKind::Move);
    assert!(
        stage.ui.drag_active(),
        "within the strip, past the threshold"
    );
    stage.pointer(to, centre_y(), PointerKind::Move);
    assert_eq!(stage.ui.cursor_icon(), CursorIcon::Grabbing);
    stage.release(to, centre_y());
    assert_eq!(stage.edits(), ["[\"a\"] -> After(\"c\")"]);
    stage.describe();
    // The views move with their keys; nothing is remounted.
    let moved = stage.ui.children(&stage.tabs).unwrap();
    assert_eq!(
        moved,
        [views[1].clone(), views[2].clone(), views[0].clone()]
    );
    assert_eq!(
        stage.property("selected"),
        Some(InspectValue::Text("a".into())),
        "a drop selects the moved tab, as a click would"
    );
}

#[test]
fn a_press_below_the_threshold_still_only_selects() {
    let mut stage = stage(&["a", "b"], 800.0, false);
    let titles = stage.titles();
    let x = titles[1].x() + 2.0;
    stage.press(x, centre_y());
    stage.pointer(x + 2.0, centre_y(), PointerKind::Move);
    assert!(!stage.ui.drag_active());
    stage.release(x + 2.0, centre_y());
    assert!(stage.edits().is_empty());
    assert_eq!(
        stage.property("selected"),
        Some(InspectValue::Text("b".into()))
    );
}

#[test]
fn an_insertion_line_marks_the_boundary_while_the_drag_is_over_the_strip() {
    let mut stage = stage(&["a", "b", "c"], 800.0, false);
    assert!(stage.insertion_lines().is_empty());
    let titles = stage.titles();
    stage.press(titles[2].x() + 2.0, centre_y());
    stage.pointer(titles[2].x() + 8.0, centre_y(), PointerKind::Move);
    // The left half of the second title: before it, between the first two.
    stage.pointer(titles[1].x() + 1.0, centre_y(), PointerKind::Move);
    let lines = stage.insertion_lines();
    assert_eq!(lines.len(), 1);
    let line = lines[0];
    assert!(line.x() > titles[0].x() + titles[0].width() && line.x() < titles[1].x());
    // Off the strip there is nothing to mark; back on it the line returns.
    stage.pointer(titles[1].x() + 1.0, 200.0, PointerKind::Move);
    assert!(stage.insertion_lines().is_empty());
    assert_eq!(
        stage.property("detaching"),
        None,
        "a strip that only reorders"
    );
    stage.pointer(titles[1].x() + 1.0, centre_y(), PointerKind::Move);
    assert_eq!(stage.insertion_lines().len(), 1);
    stage.release(titles[1].x() + 1.0, centre_y());
    assert!(stage.insertion_lines().is_empty());
    assert_eq!(stage.edits(), ["[\"c\"] -> Before(\"b\")"]);
}

#[test]
fn a_refused_move_and_a_release_off_a_strip_that_only_reorders_change_nothing() {
    let mut stage = stage(&["a", "b"], 800.0, false);
    stage.model.borrow_mut().refuse = true;
    let titles = stage.titles();
    stage.press(titles[0].x() + 2.0, centre_y());
    stage.pointer(titles[0].x() + 8.0, centre_y(), PointerKind::Move);
    stage.pointer(
        titles[1].x() + titles[1].width(),
        centre_y(),
        PointerKind::Move,
    );
    stage.release(titles[1].x() + titles[1].width(), centre_y());
    assert_eq!(stage.model.borrow().order, ["a", "b"]);
    assert_eq!(
        stage.property("selected"),
        Some(InspectValue::Text("a".into())),
        "a refused move selects nothing"
    );
    stage.model.borrow_mut().refuse = false;
    stage.press(titles[1].x() + 2.0, centre_y());
    stage.pointer(titles[1].x() + 8.0, centre_y(), PointerKind::Move);
    stage.pointer(200.0, 200.0, PointerKind::Move);
    stage.release(200.0, 200.0);
    assert!(stage.edits().is_empty());
    assert!(stage.ui.take_presentation_requests().unwrap().is_empty());
}

#[test]
fn with_detaching_too_the_strip_reorders_and_a_release_elsewhere_detaches() {
    let mut stage = stage(&["a", "b"], 800.0, true);
    let first = stage.ui.children(&stage.tabs).unwrap()[0].clone();
    let titles = stage.titles();
    stage.press(titles[0].x() + 2.0, centre_y());
    stage.pointer(titles[0].x() + 8.0, centre_y(), PointerKind::Move);
    assert!(stage.ui.drag_active(), "a reorderable strip starts at once");
    assert_eq!(stage.property("detaching"), None);
    stage.pointer(200.0, 200.0, PointerKind::Move);
    assert_eq!(
        stage.property("detaching"),
        Some(InspectValue::Text("a".into()))
    );
    stage.release(200.0, 200.0);
    assert!(stage.edits().is_empty());
    let requests = stage.ui.take_presentation_requests().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(matches!(
        requests[0].action(),
        PresentationAction::Detach { view, .. } if view == &first
    ));
}

#[test]
fn alt_with_an_arrow_moves_the_selected_tab_without_a_pointer() {
    let mut stage = stage(&["a", "b", "c"], 800.0, false);
    let titles = stage.titles();
    // Select and focus the middle tab.
    stage.press(titles[1].x() + 2.0, centre_y());
    stage.release(titles[1].x() + 2.0, centre_y());
    let key = |stage: &mut Stage, code| {
        stage
            .ui
            .key_input(KeyCode::AltLeft, KeyAction::Press)
            .unwrap();
        stage.ui.key_input(code, KeyAction::Press).unwrap();
        stage.ui.key_input(code, KeyAction::Release).unwrap();
        stage
            .ui
            .key_input(KeyCode::AltLeft, KeyAction::Release)
            .unwrap();
        stage.ui.flush(stage.ui.now()).unwrap();
    };
    key(&mut stage, KeyCode::ArrowRight);
    assert_eq!(stage.edits(), ["[\"b\"] -> After(\"c\")"]);
    stage.describe();
    key(&mut stage, KeyCode::ArrowRight);
    assert!(stage.edits().is_empty(), "the last tab has nowhere to go");
    key(&mut stage, KeyCode::ArrowLeft);
    assert_eq!(stage.edits(), ["[\"b\"] -> Before(\"c\")"]);
    assert_eq!(
        stage.property("selected"),
        Some(InspectValue::Text("b".into())),
        "the moved tab stays selected"
    );
}

#[test]
fn a_drag_over_an_overflow_arrow_scrolls_the_titles_and_then_stops() {
    let names: Vec<String> = (0..20).map(|index| format!("t{index}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut stage = stage(&names, 300.0, false);
    let scroll = |stage: &Stage| match stage.property("scroll_x") {
        Some(InspectValue::Number(value)) => value,
        other => panic!("scroll_x: {other:?}"),
    };
    // The first mask is the Previous arrow's chevron; the next is a title.
    let titles = stage.titles();
    stage.press(titles[1].x() + 2.0, centre_y());
    stage.pointer(titles[1].x() + 8.0, centre_y(), PointerKind::Move);
    assert!(stage.ui.drag_active());
    stage.pointer(150.0, centre_y(), PointerKind::Move);
    assert_eq!(stage.ui.next_deadline(), None, "nothing scrolls mid-strip");
    // The Next arrow sits at the right end of an overflowing strip.
    stage.pointer(295.0, centre_y(), PointerKind::Move);
    let first = stage.ui.next_deadline().expect("an auto-scroll timer");
    stage.ui.flush(first).unwrap();
    let after_one = scroll(&stage);
    assert!(after_one > 0.0);
    let second = stage.ui.next_deadline().expect("it repeats while resting");
    stage.ui.flush(second).unwrap();
    assert!(scroll(&stage) > after_one);
    stage.pointer(150.0, centre_y(), PointerKind::Move);
    assert_eq!(stage.ui.next_deadline(), None, "leaving the arrow stops it");
    stage.release(150.0, centre_y());
    assert_eq!(stage.ui.next_deadline(), None);
    assert_eq!(stage.edits().len(), 1, "and the tab was moved along");
}

#[test]
fn another_strips_tab_is_not_taken() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0).unwrap())
        .unwrap();
    let edits = Rc::new(RefCell::new(Vec::new()));
    let strip = |name: &str| {
        let edits = edits.clone();
        let name = name.to_owned();
        Element::new(
            name.clone(),
            Tabs::new([
                Tab::new("x", "Tab x", Element::new("a", Label::new("A"))),
                Tab::new("y", "Tab y", Element::new("b", Label::new("B"))),
            ])
            .reorderable(move |_, drop| {
                edits
                    .borrow_mut()
                    .push(format!("{name}: {:?}", drop.items()));
                Ok(true)
            }),
        )
    };
    ui.mount_tree(Element::new(
        "page",
        Column::new([strip("upper"), strip("lower")]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let lower = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "lower")
        .unwrap()
        .id;
    let pointer = |ui: &mut Runtime, x: f32, y: f32, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
        ui.flush(ui.now()).unwrap();
    };
    pointer(
        &mut ui,
        20.0,
        15.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 30.0, 15.0, PointerKind::Move);
    // Down to the lower strip, found by its own tab titles.
    let mut y = 40.0;
    while ui.inspect().drag.unwrap().target != Some(lower) && y < 300.0 {
        pointer(&mut ui, 30.0, y, PointerKind::Move);
        y += 4.0;
    }
    assert!(
        y >= 300.0,
        "the lower strip never takes the upper one's tab"
    );
    pointer(
        &mut ui,
        30.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(edits.borrow().is_empty());
}
