//! Navigation and input helpers the gallery tests share: mounting the
//! gallery, finding components by key, opening pages and driving input.
use crate::gallery::Gallery;
use pert_ui::*;
use std::time::Duration;

pub(super) fn mounted() -> (Runtime, WidgetId) {
    let mut ui = Runtime::new();
    // App initializes before the native viewport exists.
    let root = Gallery::new().unwrap().mount(&mut ui).unwrap();
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    (ui, root)
}
pub(super) fn settle(ui: &mut Runtime) {
    ui.flush(ui.inspect().at).unwrap();
}
/// The widget keyed `key` in the subtree rooted at `root`, `root` itself
/// included. Panics when no widget or more than one carries the key, so a
/// repeated key fails the test instead of silently retargeting it.
pub(super) fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    let own = (ui.key(root).unwrap() == key).then(|| root.clone());
    let below = ui
        .find_descendant(root, key)
        .unwrap_or_else(|error| panic!("looking up {key}: {error}"));
    match (own, below) {
        (Some(_), Some(_)) => panic!(
            "looking up {key}: {}",
            RuntimeError::AmbiguousKey(key.to_owned())
        ),
        (Some(id), None) | (None, Some(id)) => id,
        (None, None) => panic!("missing {key}"),
    }
}
pub(super) fn press(ui: &mut Runtime, key: KeyCode) {
    ui.key_input(key, KeyAction::Press).unwrap();
    ui.key_input(key, KeyAction::Release).unwrap();
    settle(ui);
}
pub(super) fn select(ui: &mut Runtime, tabs: &WidgetId, index: usize) {
    ui.focus(tabs).unwrap();
    press(ui, KeyCode::Home);
    for _ in 0..index {
        press(ui, KeyCode::ArrowRight);
    }
}
/// Every page in presentation order: its category's index in the top strip, the
/// category's strip of pages, the page's index there and its key.
pub(super) fn pages(ui: &Runtime, root: &WidgetId) -> Vec<(usize, WidgetId, usize, String)> {
    let top = find(ui, root, "gallery-tabs");
    let mut pages = Vec::new();
    for (category, view) in ui.children(&top).unwrap().iter().enumerate() {
        let strip = find(ui, view, "category-pages");
        for (index, page) in ui.children(&strip).unwrap().iter().enumerate() {
            pages.push((
                category,
                strip.clone(),
                index,
                ui.key(page).unwrap().to_string(),
            ));
        }
    }
    pages
}
/// The mounted page keyed `key`, found through its category's strip of pages,
/// so a lookup below it cannot reach a component of another page.
pub(super) fn page(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    let (_, strip, index, _) = pages(ui, root)
        .into_iter()
        .find(|page| page.3 == key)
        .unwrap_or_else(|| panic!("no gallery page {key}"));
    ui.children(&strip).unwrap()[index].clone()
}
pub(super) fn page_keys(ui: &Runtime, root: &WidgetId) -> Vec<String> {
    pages(ui, root).into_iter().map(|page| page.3).collect()
}
/// Opens a page by key through the keyboard: its category, then the page.
pub(super) fn open(ui: &mut Runtime, root: &WidgetId, key: &str) {
    let (category, strip, index, _) = pages(ui, root)
        .into_iter()
        .find(|page| page.3 == key)
        .unwrap_or_else(|| panic!("no gallery page {key}"));
    let top = find(ui, root, "gallery-tabs");
    select(ui, &top, category);
    select(ui, &strip, index);
}
pub(super) fn category_of(ui: &Runtime, root: &WidgetId, key: &str) -> usize {
    pages(ui, root)
        .into_iter()
        .find(|page| page.3 == key)
        .unwrap_or_else(|| panic!("no gallery page {key}"))
        .0
}
pub(super) fn click(ui: &mut Runtime, button: &WidgetId) {
    ui.focus(button).unwrap();
    press(ui, KeyCode::Enter);
}
pub(super) fn text(ui: &Runtime, key: &str) -> Vec<String> {
    ui.inspect()
        .components
        .iter()
        .filter(|item| item.key == key)
        .filter_map(|item| match item.details.properties.get("text") {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .collect()
}
/// Chooses Settings › (Theme or Density) › option through the keyboard. The
/// menu bar captures focus when it opens and returns it when a row is chosen.
pub(super) fn choose_setting(ui: &mut Runtime, submenu: usize, option: usize) {
    let focused = ui.focused();
    press(ui, KeyCode::F10);
    press(ui, KeyCode::ArrowDown);
    for _ in 0..submenu {
        press(ui, KeyCode::ArrowDown);
    }
    press(ui, KeyCode::ArrowRight);
    for _ in 0..option {
        press(ui, KeyCode::ArrowDown);
    }
    press(ui, KeyCode::Enter);
    assert_eq!(ui.focused(), focused);
}
/// Whether a gallery dialog, keyed by its section, is presented.
pub(super) fn dialog_shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|item| item.key == key && item.visible)
}
// At the desktop viewport every category title fits. Locate a title in the top
// strip from the rendered header geometry, so this exercises the same pointer
// path as the app; the category strips below it start further down.
pub(super) fn category_title_point(ui: &mut Runtime, tabs: &WidgetId, index: usize) -> Point {
    let [_, y, _, _] = ui.bounds(tabs).unwrap().unwrap().values();
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { rect, color, .. }
                if *color == Palette::dark().color(ColorRole::OnTab) =>
            {
                let [x, top, width, height] = rect.values();
                (top >= y && top + height <= y + 36.0)
                    .then(|| Point::new(x + width / 2.0, top + height / 2.0).unwrap())
            }
            _ => None,
        })
        .nth(index)
        .unwrap()
}
// The top-strip title of the category holding the Input page.
pub(super) fn input_category_point(ui: &mut Runtime, root: &WidgetId) -> Point {
    let tabs = find(ui, root, "gallery-tabs");
    let index = category_of(ui, root, "input");
    category_title_point(ui, &tabs, index)
}
pub(super) fn pointer_click(ui: &mut Runtime, point: Point) {
    for kind in [
        PointerKind::Move,
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
}
/// The first inspected parameter's state; the gallery's parameters are continuous.
pub(super) fn first_parameter(ui: &mut Runtime) -> ParameterSnapshot<f32> {
    match &ui.inspect().parameters[0].parameter {
        AnyParameterSnapshot::Continuous { state, .. } => *state,
        other => panic!(
            "expected a continuous parameter, found {:?}",
            other.family()
        ),
    }
}

/// The inspected base value of the parameter whose key is `key`.
pub(super) fn parameter_base(ui: &mut Runtime, key: &str) -> AnyValue {
    ui.inspect()
        .parameters
        .iter()
        .find(|parameter| parameter.id.key() == key)
        .unwrap_or_else(|| panic!("no parameter {key}"))
        .parameter
        .base()
}

pub(super) fn pointer_at(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    settle(ui);
}
pub(super) fn center_of(ui: &Runtime, id: &WidgetId) -> (f32, f32) {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    (x + w / 2.0, y + h / 2.0)
}
/// Presses at one point, crosses the drag threshold and releases at another.
pub(super) fn drag_between(ui: &mut Runtime, from: (f32, f32), to: (f32, f32)) {
    pointer_at(
        ui,
        from.0,
        from.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer_at(ui, from.0 + 6.0, from.1, PointerKind::Move);
    pointer_at(ui, to.0, to.1, PointerKind::Move);
    pointer_at(ui, to.0, to.1, PointerKind::Release(PointerButton::Primary));
}
