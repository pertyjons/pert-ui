//! Links: button activation with link presentation, semantics and cursor.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc};
use support::{key, press, settle, widget};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}

/// A link keyed `link` that counts its activations, beside a button with the
/// same text and a second Tab stop.
fn mount(ui: &mut Runtime, link: impl FnOnce(Link) -> Link) -> (WidgetId, Rc<Cell<u32>>) {
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new(
                    "link",
                    link(Link::new("Mixer", move |_| {
                        counter.set(counter.get() + 1);
                        Ok(())
                    })),
                ),
                Element::new("button", Button::new("Mixer", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    settle(ui);
    (root, count)
}

fn semantics(ui: &Runtime, key: &str) -> Semantics {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.semantics)
        .unwrap_or_else(|| panic!("no semantics for {key}"))
}

fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    Point::new(x + w / 2.0, y + h / 2.0).unwrap()
}

fn pointer(ui: &mut Runtime, at: Point, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
    settle(ui);
}

/// Filled rectangles in `color`, which for a link's color are underlines.
fn rects_in(ui: &mut Runtime, color: Color) -> usize {
    settle(ui);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| matches!(command, DrawCommand::FillRect { color: c, .. } if *c == color))
        .count()
}

fn link_color(ui: &Runtime) -> Color {
    ui.theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::Link)
}

#[test]
fn a_link_reports_the_link_role_and_is_text_sized() {
    let mut ui = ui();
    let (root, _) = mount(&mut ui, |link| link);
    let link = semantics(&ui, "link");
    assert_eq!(link.role, SemanticRole::Link);
    assert_eq!(link.name.as_deref(), Some("Mixer"));
    assert!(link.actions.contains(&SemanticAction::Activate));
    assert!(!link.disabled);
    // No button padding or minimum size: the same text is smaller in both axes.
    let [_, _, lw, lh] = ui
        .bounds(&widget(&ui, &root, "link"))
        .unwrap()
        .unwrap()
        .values();
    let [_, _, bw, bh] = ui
        .bounds(&widget(&ui, &root, "button"))
        .unwrap()
        .unwrap()
        .values();
    assert!(lw < bw, "link {lw} vs button {bw}");
    assert!(lh < bh, "link {lh} vs button {bh}");
}

#[test]
fn pointer_activates_once_and_a_release_outside_cancels() {
    let mut ui = ui();
    let (root, count) = mount(&mut ui, |link| link);
    let link = widget(&ui, &root, "link");
    let at = center(&ui, &link);
    pointer(&mut ui, at, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Pointer);
    pointer(&mut ui, at, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, at, PointerKind::Release(PointerButton::Primary));
    assert_eq!(count.get(), 1);
    let outside = Point::new(590.0, 290.0).unwrap();
    pointer(&mut ui, at, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, outside, PointerKind::Move);
    pointer(
        &mut ui,
        outside,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(count.get(), 1);
}

#[test]
fn enter_and_space_activate_on_release_and_escape_cancels() {
    let mut ui = ui();
    let (root, count) = mount(&mut ui, |link| link);
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(widget(&ui, &root, "link")));
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Space);
    assert_eq!(count.get(), 2);
    // An OS repeat neither re-arms nor activates a second time.
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(count.get(), 3);
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    key(&mut ui, KeyCode::Escape);
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(count.get(), 3);
}

#[test]
fn a_disabled_link_is_muted_inert_and_skipped_by_tab() {
    let mut ui = ui();
    let (root, count) = mount(&mut ui, Link::disabled);
    let link = widget(&ui, &root, "link");
    let semantics = semantics(&ui, "link");
    assert!(semantics.disabled);
    assert!(semantics.actions.is_empty());
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(widget(&ui, &root, "button")));
    let at = center(&ui, &link);
    pointer(&mut ui, at, PointerKind::Move);
    assert_ne!(ui.cursor_icon(), CursorIcon::Pointer);
    pointer(&mut ui, at, PointerKind::Press(PointerButton::Primary));
    pointer(&mut ui, at, PointerKind::Release(PointerButton::Primary));
    assert_eq!(count.get(), 0);
    let color = link_color(&ui);
    assert_eq!(rects_in(&mut ui, color), 0);
}

#[test]
fn disabling_through_the_signal_releases_focus_and_stops_activation() {
    let mut ui = ui();
    let enabled = ui.signal(true);
    let source = enabled.read_only();
    let (root, count) = mount(&mut ui, move |link| link.enabled(source));
    let link = widget(&ui, &root, "link");
    ui.focus(&link).unwrap();
    settle(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    enabled.set(false).unwrap();
    settle(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(count.get(), 0);
    assert_ne!(ui.focused(), Some(link));
    assert!(semantics(&ui, "link").disabled);
}

#[test]
fn the_underline_follows_hover_or_is_always_drawn() {
    let mut ui = ui();
    let (root, _) = mount(&mut ui, |link| link);
    let color = link_color(&ui);
    assert_eq!(rects_in(&mut ui, color), 0, "no underline at rest");
    let at = center(&ui, &widget(&ui, &root, "link"));
    pointer(&mut ui, at, PointerKind::Move);
    assert_eq!(rects_in(&mut ui, color), 1, "underline while hovered");
    pointer(&mut ui, at, PointerKind::Leave);
    assert_eq!(rects_in(&mut ui, color), 0, "underline gone after leaving");

    let mut ui = self::ui();
    mount(&mut ui, |link| link.underline(LinkUnderline::Always));
    assert_eq!(rects_in(&mut ui, color), 1, "always underlined");
}

#[test]
fn a_bound_text_relabels_the_link() {
    let mut ui = ui();
    let text = ui.signal("Mixer".to_owned());
    ui.mount_tree(Element::new(
        "row",
        Row::new([Element::new(
            "link",
            Link::new(text.read_only(), |_| Ok(())),
        )]),
    ))
    .unwrap();
    settle(&mut ui);
    assert_eq!(semantics(&ui, "link").name.as_deref(), Some("Mixer"));
    text.set("Arrangement".to_owned()).unwrap();
    settle(&mut ui);
    assert_eq!(semantics(&ui, "link").name.as_deref(), Some("Arrangement"));
}
