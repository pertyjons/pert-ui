//! The built-in zoom-and-pan view, through the facade.
//!
//! These pin `ZoomView`'s gestures, its controller, its workspace participation
//! and what a pan and a zoom cost, as described in
//! [transforms](../../../docs/rendering-platform.md).
use crate::support;

use pert_ui::*;
use std::{cell::Cell, rc::Rc};
use support::{press, settle};

fn point(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
fn scale(factor: f32) -> ViewScale {
    ViewScale::new(factor).unwrap()
}
fn view(factor: f32, x: f32, y: f32) -> ViewTransform {
    ViewTransform::new(scale(factor), point(x, y))
}
fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn primary_key() -> KeyCode {
    KeyCode::primary()
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(point(x, y), kind))
        .unwrap();
    settle(ui);
}
fn origin(ui: &Runtime, id: &WidgetId) -> [f32; 4] {
    ui.bounds(id).unwrap().expect("laid out").values()
}
fn close(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-3)
}

/// A fixed-size block that counts its updates and paints.
struct Block {
    size: Size,
    updates: Rc<Cell<u32>>,
    paints: Rc<Cell<u32>>,
}
impl Block {
    fn new(width: f32, height: f32, updates: &Rc<Cell<u32>>, paints: &Rc<Cell<u32>>) -> Self {
        Self {
            size: Size::new(width, height).unwrap(),
            updates: updates.clone(),
            paints: paints.clone(),
        }
    }
}
impl Component for Block {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        painter.fill_rect(
            Rect::from_size(self.size),
            painter.color(ColorRole::Primary),
        );
        Ok(())
    }
}

fn block(key: &str, width: f32, height: f32) -> Element {
    let counter = Rc::new(Cell::new(0));
    Element::new(key, Block::new(width, height, &counter, &counter))
}

/// Mounts a view of two blocks, 100×50 at (20, 20) and (220, 120).
fn two_blocks(ui: &mut Runtime, configure: impl FnOnce(ZoomView) -> ZoomView) -> Vec<WidgetId> {
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            configure(ZoomView::new([
                (point(20.0, 20.0), block("a", 100.0, 50.0)),
                (point(220.0, 120.0), block("b", 100.0, 50.0)),
            ])),
        ))
        .unwrap();
    settle(ui);
    let mut ids = vec![root.clone()];
    ids.extend(ui.children(&root).unwrap());
    ids
}

#[test]
fn items_are_placed_at_their_origins_and_move_with_their_keys() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new("root", Column::default()))
        .unwrap();
    let describe = |a: (f32, f32), b: (f32, f32)| {
        vec![Element::new(
            "canvas",
            ZoomView::new([
                (point(a.0, a.1), block("a", 10.0, 10.0)),
                (point(b.0, b.1), block("b", 10.0, 10.0)),
            ]),
        )]
    };
    ui.reconcile_children(&root, describe((5.0, 5.0), (50.0, 60.0)))
        .unwrap();
    settle(&mut ui);
    let canvas = ui.children(&root).unwrap()[0].clone();
    let items = ui.children(&canvas).unwrap();
    assert_eq!(origin(&ui, &items[1])[..2], [50.0, 60.0]);
    // The same keys, described in another order with other origins: each item
    // keeps its identity and moves with its key.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "canvas",
            ZoomView::new([
                (point(80.0, 90.0), block("b", 10.0, 10.0)),
                (point(1.0, 2.0), block("a", 10.0, 10.0)),
            ]),
        )],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(origin(&ui, &items[0])[..2], [1.0, 2.0]);
    assert_eq!(origin(&ui, &items[1])[..2], [80.0, 90.0]);
}

#[test]
fn the_wheel_pans_and_zooms_at_the_pointer_with_the_primary_modifier() {
    let mut ui = ui();
    let ids = two_blocks(&mut ui, |view| view);
    let b = &ids[2];
    pointer(&mut ui, 10.0, 10.0, PointerKind::Scroll(point(30.0, 40.0)));
    assert_eq!(origin(&ui, b), [190.0, 80.0, 100.0, 50.0], "scrolling pans");

    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(&mut ui, 10.0, 10.0, PointerKind::Scroll(point(0.0, 10.0)));
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(
        origin(&ui, b)[..2],
        [180.0, 80.0],
        "Shift turns a vertical wheel horizontal"
    );

    // One doubling of scroll distance upward doubles the scale, and the point
    // under the pointer stays where it is.
    ui.key_input(primary_key(), KeyAction::Press).unwrap();
    pointer(
        &mut ui,
        180.0,
        80.0,
        PointerKind::Scroll(point(0.0, -200.0)),
    );
    ui.key_input(primary_key(), KeyAction::Release).unwrap();
    assert!(close(origin(&ui, b), [180.0, 80.0, 200.0, 100.0]));
}

#[test]
fn a_zooming_wheel_swaps_the_gestures_and_stays_within_the_limits() {
    let mut ui = ui();
    let limits = ScaleLimits::new(scale(4.0), scale(0.5));
    assert_eq!((limits.min(), limits.max()), (scale(0.5), scale(4.0)));
    let ids = two_blocks(&mut ui, |view| {
        view.wheel(WheelGesture::Zoom).scale_limits(limits)
    });
    let a = &ids[1];
    for _ in 0..10 {
        pointer(&mut ui, 20.0, 20.0, PointerKind::Scroll(point(0.0, -200.0)));
    }
    assert!(
        close(origin(&ui, a), [20.0, 20.0, 400.0, 200.0]),
        "held at the largest scale"
    );
    for _ in 0..10 {
        pointer(&mut ui, 20.0, 20.0, PointerKind::Scroll(point(0.0, 200.0)));
    }
    assert!(close(origin(&ui, a), [20.0, 20.0, 50.0, 25.0]));
    ui.key_input(primary_key(), KeyAction::Press).unwrap();
    pointer(&mut ui, 20.0, 20.0, PointerKind::Scroll(point(0.0, 20.0)));
    assert!(
        close(origin(&ui, a), [20.0, 0.0, 50.0, 25.0]),
        "the primary modifier pans"
    );
}

#[test]
fn dragging_the_background_or_with_the_middle_button_pans() {
    let mut ui = ui();
    let pressed = Rc::new(Cell::new(0));
    let count = pressed.clone();
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            ZoomView::new([(
                point(20.0, 20.0),
                Element::new(
                    "button",
                    Button::new("Press", move |_| {
                        count.set(count.get() + 1);
                        Ok(())
                    }),
                ),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let button = ui.children(&root).unwrap()[0].clone();
    let start = origin(&ui, &button);

    // The button takes its own primary press, and the view does not pan.
    let inside = (start[0] + 4.0, start[1] + 4.0);
    pointer(
        &mut ui,
        inside.0,
        inside.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, inside.0 + 50.0, inside.1, PointerKind::Move);
    pointer(
        &mut ui,
        inside.0 + 50.0,
        inside.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(origin(&ui, &button), start);

    // The middle button over the button pans, since the button ignores it.
    pointer(
        &mut ui,
        inside.0,
        inside.1,
        PointerKind::Press(PointerButton::Middle),
    );
    pointer(&mut ui, inside.0 + 30.0, inside.1 + 10.0, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Grabbing);
    pointer(
        &mut ui,
        inside.0 + 30.0,
        inside.1 + 10.0,
        PointerKind::Release(PointerButton::Middle),
    );
    assert_eq!(
        origin(&ui, &button)[..2],
        [start[0] + 30.0, start[1] + 10.0]
    );
    assert_eq!(
        pressed.get(),
        1,
        "only the primary click activated the button"
    );

    // The background pans with the primary button and takes focus.
    pointer(
        &mut ui,
        300.0,
        250.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 290.0, 240.0, PointerKind::Move);
    pointer(
        &mut ui,
        290.0,
        240.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(origin(&ui, &button)[..2], [start[0] + 20.0, start[1]]);
    assert_eq!(ui.focused(), Some(root.clone()));

    // Escape ends a pan where it is.
    pointer(
        &mut ui,
        300.0,
        250.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 310.0, 250.0, PointerKind::Move);
    press(&mut ui, KeyCode::Escape);
    pointer(&mut ui, 350.0, 250.0, PointerKind::Move);
    assert_eq!(origin(&ui, &button)[..2], [start[0] + 30.0, start[1]]);
    assert_ne!(ui.cursor_icon(), CursorIcon::Grabbing);
}

#[test]
fn the_keyboard_pans_zooms_resets_and_fits() {
    let mut ui = ui();
    let ids = two_blocks(&mut ui, |view| view);
    let (root, a, b) = (&ids[0], &ids[1], &ids[2]);
    ui.focus(root).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(origin(&ui, a)[..2], [-20.0, -20.0]);

    // `=` zooms at the center, (200, 150).
    press(&mut ui, KeyCode::Equal);
    let zoomed = origin(&ui, a);
    assert!(close(
        zoomed,
        [200.0 - 1.25 * 220.0, 150.0 - 1.25 * 170.0, 125.0, 62.5]
    ));
    press(&mut ui, KeyCode::Minus);
    assert!(close(origin(&ui, a), [-20.0, -20.0, 100.0, 50.0]));
    press(&mut ui, KeyCode::Equal);
    press(&mut ui, KeyCode::Digit0);
    assert!(
        close(origin(&ui, a), [-20.0, -20.0, 100.0, 50.0]),
        "100% at the center"
    );

    // Home fits both blocks, 300×150 of content, into 400×300 less the margins:
    // at most 100%, centered.
    press(&mut ui, KeyCode::Home);
    assert!(close(origin(&ui, a), [50.0, 75.0, 100.0, 50.0]));
    assert!(close(origin(&ui, b), [250.0, 175.0, 100.0, 50.0]));
}

#[test]
fn fitting_large_content_shrinks_it_within_the_limits() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    assert!(!zoom.fit().unwrap(), "nothing arranged yet");
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            ZoomView::new([
                (point(0.0, 0.0), block("a", 10.0, 10.0)),
                (point(3990.0, 1990.0), block("b", 10.0, 10.0)),
            ])
            .scale_limits(ScaleLimits::new(ViewScale::MIN, scale(1.0)))
            .controller(&zoom),
        ))
        .unwrap();
    settle(&mut ui);
    assert!(zoom.fit().unwrap());
    settle(&mut ui);
    // 4000 wide into 400 - 2·16 = 368: the width decides.
    let fitted = zoom.view().get().unwrap();
    assert!((fitted.scale().get() - 368.0 / 4000.0).abs() < 1e-6);
    let b = ui.children(&root).unwrap()[1].clone();
    let [x, _, w, _] = origin(&ui, &b);
    assert!(
        (x + w - (400.0 - 16.0)).abs() < 1e-3,
        "right edge at the margin"
    );

    // Held at the smallest scale, the content stays centered.
    ui.unmount(&root).unwrap();
    let small = ZoomController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "narrow",
            ZoomView::new([
                (point(-2000.0, 0.0), block("a", 10.0, 10.0)),
                (point(1990.0, 0.0), block("b", 10.0, 10.0)),
            ])
            .scale_limits(ScaleLimits::new(scale(0.5), scale(2.0)))
            .controller(&small),
        ))
        .unwrap();
    settle(&mut ui);
    assert!(small.fit().unwrap());
    settle(&mut ui);
    assert_eq!(small.view().get().unwrap().scale(), scale(0.5));
    let a = ui.children(&root).unwrap()[0].clone();
    let b = ui.children(&root).unwrap()[1].clone();
    let (left, right) = (origin(&ui, &a)[0], origin(&ui, &b));
    assert!(
        ((left + right[0] + right[2]) / 2.0 - 200.0).abs() < 1e-3,
        "{left} {right:?} {:?}",
        small.view().get().unwrap()
    );
}

#[test]
fn the_controller_follows_gestures_and_its_writes_are_shown() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    zoom.set(view(2.0, 10.0, 0.0)).unwrap();
    let ids = two_blocks(&mut ui, |view| view.controller(&zoom));
    let a = &ids[1];
    assert!(
        close(origin(&ui, a), [50.0, 40.0, 200.0, 100.0]),
        "starts at the controller's view"
    );

    pointer(&mut ui, 0.0, 0.0, PointerKind::Scroll(point(10.0, 0.0)));
    assert_eq!(
        zoom.view().get().unwrap(),
        view(2.0, 0.0, 0.0),
        "a gesture is published"
    );

    // A view outside the gesture limits is shown as it is.
    zoom.set(view(16.0, 0.0, 0.0)).unwrap();
    settle(&mut ui);
    assert!(close(origin(&ui, a), [320.0, 320.0, 1600.0, 800.0]));

    let snapshot = ui.inspect();
    let inspected = snapshot
        .components
        .iter()
        .find(|item| item.key == "canvas")
        .unwrap();
    assert_eq!(
        inspected.details.properties.get("view_scale"),
        Some(&InspectValue::Number(16.0))
    );
    assert_eq!(
        inspected.details.semantics.as_ref().map(|s| s.role),
        Some(SemanticRole::ScrollArea)
    );
}

#[test]
fn a_pan_moves_retained_drawing_while_a_zoom_prepares_the_content_again() {
    let mut ui = ui();
    let (updates, paints) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let zoom = ZoomController::new(&ui);
    ui.mount_tree(Element::new(
        "canvas",
        ZoomView::new([(
            point(0.0, 0.0),
            Element::new("block", Block::new(50.0, 40.0, &updates, &paints)),
        )])
        .controller(&zoom),
    ))
    .unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    let before = (updates.get(), paints.get());

    pointer(&mut ui, 5.0, 5.0, PointerKind::Scroll(point(-25.0, -5.0)));
    ui.paint().unwrap();
    assert_eq!(
        (updates.get(), paints.get()),
        before,
        "a pan leaves the content alone"
    );

    zoom.set(view(2.0, 25.0, 5.0)).unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    assert_eq!((updates.get(), paints.get()), (before.0 + 1, before.1 + 1));
    assert_eq!(ui.next_deadline(), None, "and nothing stays scheduled");
}

#[test]
fn the_view_is_workspace_state() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    let ids = two_blocks(&mut ui, |view| view.controller(&zoom));
    zoom.set(view(1.5, -40.0, 12.0)).unwrap();
    settle(&mut ui);
    let saved = ui.capture_workspace().unwrap();
    let decoded = Workspace::decode(&saved.encode()).unwrap();
    let (_, state) = decoded.entries().next().unwrap();
    assert_eq!(state.kind(), "zoom");
    assert_eq!(state.number("scale"), Ok(1.5));

    zoom.set(ViewTransform::IDENTITY).unwrap();
    settle(&mut ui);
    assert!(ui.restore_workspace(&decoded).unwrap().is_complete());
    assert_eq!(
        zoom.view().get().unwrap(),
        view(1.5, -40.0, 12.0),
        "the controller follows"
    );
    settle(&mut ui);
    assert!(close(origin(&ui, &ids[1]), [-10.0, 42.0, 150.0, 75.0]));

    let with = |factor: f64| {
        Workspace::new(
            WorkspaceSettings::default(),
            [(
                WorkspacePath::new(["canvas"]).unwrap(),
                ViewState::new("zoom")
                    .unwrap()
                    .with("scale", WorkspaceValue::number(factor).unwrap())
                    .unwrap()
                    .with("x", WorkspaceValue::number(0.0).unwrap())
                    .unwrap()
                    .with("y", WorkspaceValue::number(0.0).unwrap())
                    .unwrap(),
            )],
        )
        .unwrap()
    };
    let report = ui.restore_workspace(&with(64.0)).unwrap();
    assert_eq!(
        report
            .rejected()
            .map(|(_, why)| why.clone())
            .collect::<Vec<_>>(),
        [RestoreRejection::OutOfRange("scale".to_owned())]
    );

    // A pan in progress owns the view.
    pointer(
        &mut ui,
        390.0,
        290.0,
        PointerKind::Press(PointerButton::Primary),
    );
    let report = ui.restore_workspace(&with(2.0)).unwrap();
    assert_eq!(
        report
            .rejected()
            .map(|(_, why)| why.clone())
            .collect::<Vec<_>>(),
        [RestoreRejection::Busy]
    );
}

#[test]
fn focus_moving_to_an_item_outside_the_view_pans_it_into_view() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            ZoomView::new([
                (
                    point(10.0, 10.0),
                    Element::new("near", Button::new("Near", |_| Ok(()))),
                ),
                (
                    point(900.0, 700.0),
                    Element::new("far", Button::new("Far", |_| Ok(()))),
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let far = ui.children(&root).unwrap()[1].clone();
    ui.focus(&far).unwrap();
    settle(&mut ui);
    let [x, y, w, h] = origin(&ui, &far);
    assert!(
        x >= 0.0 && y >= 0.0 && x + w <= 400.0 && y + h <= 300.0,
        "{x}, {y}"
    );
}

#[test]
fn the_grid_follows_the_view_and_thins_out_when_small() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    ui.mount_tree(Element::new(
        "canvas",
        ZoomView::new([])
            .grid(Length::new(40.0).unwrap())
            .controller(&zoom),
    ))
    .unwrap();
    settle(&mut ui);
    let lines = |ui: &mut Runtime| {
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .filter(|command| matches!(command, DrawCommand::FillRect { .. }))
            .count()
    };
    // 400 / 40 vertical and 300 / 40 horizontal lines, from zero.
    assert_eq!(lines(&mut ui), 10 + 8);
    zoom.set(view(0.125, 0.0, 0.0)).unwrap();
    settle(&mut ui);
    assert_eq!(
        lines(&mut ui),
        0,
        "five-pixel squares are too dense to draw"
    );
}

#[test]
fn keys_an_item_ignores_and_modified_chords_leave_the_view_alone() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "canvas",
            ZoomView::new([(
                point(20.0, 20.0),
                Element::new("button", Button::new("Press", |_| Ok(()))),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let button = ui.children(&root).unwrap()[0].clone();
    let start = origin(&ui, &button);
    ui.focus(&button).unwrap();
    settle(&mut ui);
    for key in [
        KeyCode::Minus,
        KeyCode::ArrowDown,
        KeyCode::Home,
        KeyCode::Digit0,
    ] {
        press(&mut ui, key);
    }
    assert_eq!(origin(&ui, &button), start, "the focused button's keys");

    ui.focus(&root).unwrap();
    settle(&mut ui);
    ui.key_input(primary_key(), KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::Minus);
    ui.key_input(primary_key(), KeyAction::Release).unwrap();
    assert_eq!(
        origin(&ui, &button),
        start,
        "a chord stays an application command"
    );
    press(&mut ui, KeyCode::Minus);
    assert_ne!(origin(&ui, &button), start, "the focused view zooms");
}

#[test]
fn the_reset_key_and_fit_respect_the_limits() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    let ids = two_blocks(&mut ui, |view| {
        view.scale_limits(ScaleLimits::new(scale(2.0), scale(4.0)))
            .controller(&zoom)
    });
    ui.focus(&ids[0]).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Equal);
    press(&mut ui, KeyCode::Digit0);
    assert_eq!(
        zoom.view().get().unwrap().scale(),
        scale(2.0),
        "the nearest limit"
    );
    assert!(zoom.fit().unwrap());
    settle(&mut ui);
    assert_eq!(
        zoom.view().get().unwrap().scale(),
        scale(2.0),
        "a smallest limit above 100% wins"
    );
}

#[test]
fn fitting_content_far_apart_does_not_overflow() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    ui.mount_tree(Element::new(
        "canvas",
        ZoomView::new([
            (point(-2e38, 0.0), block("a", 10.0, 10.0)),
            (point(2e38, 0.0), block("b", 10.0, 10.0)),
        ])
        .controller(&zoom),
    ))
    .unwrap();
    settle(&mut ui);
    assert!(
        zoom.fit().unwrap(),
        "the bounds are not mistaken for no content"
    );
    settle(&mut ui);
    assert_eq!(
        zoom.view().get().unwrap().scale(),
        ScaleLimits::DEFAULT.min()
    );
}

#[test]
fn clamping_handles_infinities_and_nan() {
    let limits = ScaleLimits::DEFAULT;
    assert_eq!(limits.clamp(f32::INFINITY), limits.max());
    assert_eq!(limits.clamp(f32::NEG_INFINITY), limits.min());
    assert_eq!(limits.clamp(f32::NAN), limits.min());
    assert_eq!(limits.clamp(2.0), scale(2.0));
}

#[test]
fn a_pinch_zooms_at_the_pointer_within_the_limits() {
    let mut ui = ui();
    let ids = two_blocks(&mut ui, |view| view);
    let b = &ids[2];
    let pinch = |factor| PointerKind::Pinch(Magnification::new(factor).unwrap());
    // Over an item that ignores it, the pinch reaches the view.
    pointer(&mut ui, 220.0, 120.0, pinch(2.0));
    assert!(close(origin(&ui, b), [220.0, 120.0, 200.0, 100.0]));
    for _ in 0..10 {
        pointer(&mut ui, 220.0, 120.0, pinch(2.0));
    }
    assert!(
        close(origin(&ui, b), [220.0, 120.0, 800.0, 400.0]),
        "held at the largest scale"
    );
    assert!(Magnification::new(-1.0).is_err());
}

#[test]
fn a_pinch_over_built_in_controls_reaches_the_view() {
    let mut ui = ui();
    let (slider, _host) = ui
        .remote_parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 1.0, 0.25).unwrap(),
        )
        .unwrap();
    let controls: Vec<(&str, Element)> = vec![
        ("label", Element::new("label", Label::new("Label"))),
        (
            "button",
            Element::new("button", Button::new("Button", |_| Ok(()))),
        ),
        (
            "checkbox",
            Element::new("checkbox", Checkbox::new("Check", ui.signal(false))),
        ),
        (
            "toggle",
            Element::new("toggle", Toggle::new("Toggle", ui.signal(false))),
        ),
        (
            "segments",
            Element::new(
                "segments",
                SegmentedControl::new(
                    ui.signal("a"),
                    [Choice::new("a", "Alpha"), Choice::new("b", "Beta")],
                )
                .unwrap(),
            ),
        ),
        (
            "radio",
            Element::new(
                "radio",
                RadioGroup::new(
                    ui.signal("a"),
                    [Choice::new("a", "Alpha"), Choice::new("b", "Beta")],
                )
                .unwrap(),
            ),
        ),
        ("slider", Element::new("slider", Slider::new(slider))),
        (
            "field",
            Element::new("field", TextField::new(ui.signal("Text".to_owned()))),
        ),
        (
            "number",
            Element::new("number", IntegerInput::new(ui.signal(3_i64))),
        ),
    ];
    let mut failed = Vec::new();
    for (name, control) in controls {
        let zoom = ZoomController::new(&ui);
        let root = ui
            .mount_tree(Element::new(
                "canvas",
                ZoomView::new([(point(20.0, 20.0), control)]).controller(&zoom),
            ))
            .unwrap();
        settle(&mut ui);
        let child = ui.children(&root).unwrap()[0].clone();
        let [x, y, w, h] = origin(&ui, &child);
        pointer(
            &mut ui,
            x + w / 2.0,
            y + h / 2.0,
            PointerKind::Pinch(Magnification::new(2.0).unwrap()),
        );
        if zoom.view().get().unwrap().scale() != scale(2.0) {
            failed.push(name);
        }
        ui.unmount(&root).unwrap();
        settle(&mut ui);
    }
    assert!(failed.is_empty(), "a pinch stopped at {failed:?}");
}

#[test]
fn an_external_transform_cancels_an_active_pan() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    two_blocks(&mut ui, |v| v.controller(&zoom));
    pointer(
        &mut ui,
        100.0,
        100.0,
        PointerKind::Press(PointerButton::Middle),
    );
    pointer(&mut ui, 120.0, 110.0, PointerKind::Move);
    let external = view(2.0, 70.0, 80.0);
    zoom.set(external).unwrap();
    settle(&mut ui);
    pointer(&mut ui, 180.0, 140.0, PointerKind::Move);
    pointer(
        &mut ui,
        180.0,
        140.0,
        PointerKind::Release(PointerButton::Middle),
    );
    assert_eq!(zoom.view().get().unwrap(), external);
}

#[test]
fn refused_restore_keeps_the_local_transform() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    let mut component = ZoomView::new([]).controller(&zoom);
    let before = component.workspace_state().unwrap();
    let saved = ViewState::new("zoom")
        .unwrap()
        .with("scale", WorkspaceValue::number(2.0).unwrap())
        .unwrap()
        .with("x", WorkspaceValue::number(50.0).unwrap())
        .unwrap()
        .with("y", WorkspaceValue::number(80.0).unwrap())
        .unwrap();
    ui.shutdown();
    assert_eq!(
        component.restore_workspace_state(&saved),
        Err(RestoreRejection::Runtime(RuntimeError::Closed))
    );
    assert_eq!(component.workspace_state().unwrap(), before);
}

#[test]
fn unmounting_the_view_or_replacing_its_controller_forgets_the_arrangement() {
    let mut ui = ui();
    let zoom = ZoomController::new(&ui);
    let canvas = |controller: &ZoomController| {
        Element::new(
            "canvas",
            ZoomView::new([
                (point(20.0, 20.0), block("a", 100.0, 50.0)),
                (point(220.0, 120.0), block("b", 100.0, 50.0)),
            ])
            .controller(controller),
        )
    };
    let root = ui
        .mount_tree(Element::new("root", Column::new([canvas(&zoom)])))
        .unwrap();
    settle(&mut ui);
    assert!(zoom.fit().unwrap(), "a mounted view fits");

    // Another controller takes over the view; the old one has nothing to fit.
    let other = ZoomController::new(&ui);
    ui.reconcile_children(&root, vec![canvas(&other)]).unwrap();
    settle(&mut ui);
    assert!(
        !zoom.fit().unwrap(),
        "the replaced controller forgot the view"
    );
    assert!(other.fit().unwrap(), "the new controller fits the view");

    // Unmounting the view leaves the controller with nothing to fit until a
    // later view is arranged over it.
    ui.reconcile_children(&root, Vec::new()).unwrap();
    settle(&mut ui);
    assert!(!other.fit().unwrap());
    ui.reconcile_children(&root, vec![canvas(&other)]).unwrap();
    settle(&mut ui);
    assert!(other.fit().unwrap(), "the controller serves a later view");
}
