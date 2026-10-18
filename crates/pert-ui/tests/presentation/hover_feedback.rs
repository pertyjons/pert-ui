//! Hover feedback on editable surfaces through the public facade.
use pert_ui::*;
use std::time::Duration;

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 160.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn move_to(ui: &mut Runtime, x: f32, y: f32) {
    ui.pointer_input(PointerEvent::new(
        Point::new(x, y).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}

fn count(ui: &mut Runtime, role: ColorRole) -> usize {
    let color = ui.theme().palette(ThemeMode::Dark).unwrap().color(role);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| {
            matches!(command,
                DrawCommand::FillRect { color: fill, .. }
                | DrawCommand::DrawMask { color: fill, .. }
                if *fill == color)
        })
        .count()
}

#[test]
fn slider_track_hover_appears_and_clears_without_moving_the_control() {
    let mut ui = runtime();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    let id = ui
        .mount_tree(Element::new("slider", Slider::new(parameter)))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let bounds = ui.bounds(&id).unwrap().unwrap();
    let rest = count(&mut ui, ColorRole::SurfaceHover);
    move_to(
        &mut ui,
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );
    assert!(count(&mut ui, ColorRole::SurfaceHover) > rest);
    assert_eq!(ui.bounds(&id).unwrap().unwrap(), bounds);
    move_to(&mut ui, 500.0, 200.0);
    assert_eq!(count(&mut ui, ColorRole::SurfaceHover), rest);
}

#[test]
fn disabled_slider_keeps_its_resting_presentation_on_hover() {
    let mut ui = runtime();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 2, "disabled").unwrap(),
            ParameterSpec::new("Disabled", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    ui.mount_tree(Element::new("slider", Slider::new(parameter).disabled()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let resting = ui.paint().unwrap().commands().to_vec();
    move_to(&mut ui, 200.0, 20.0);
    assert_eq!(ui.paint().unwrap().commands(), resting);
}

#[test]
fn text_and_inline_number_fields_share_neutral_hover_feedback() {
    for control in [
        Box::new(|ui: &Runtime| Element::new("field", TextField::new(ui.signal("Name".to_owned()))))
            as Box<dyn Fn(&Runtime) -> Element>,
        Box::new(|ui: &Runtime| {
            Element::new("field", InlineInteger::new(ui.signal(8_i64)).label("Value"))
        }),
    ] {
        let mut ui = runtime();
        let id = ui.mount_tree(control(&ui)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let bounds = ui.bounds(&id).unwrap().unwrap();
        let rest = count(&mut ui, ColorRole::SurfaceHover);
        move_to(
            &mut ui,
            bounds.x() + 8.0,
            bounds.y() + bounds.height() / 2.0,
        );
        assert!(count(&mut ui, ColorRole::SurfaceHover) > rest);
        move_to(&mut ui, 500.0, 200.0);
        assert_eq!(count(&mut ui, ColorRole::SurfaceHover), rest);
    }
}

#[test]
fn toast_buttons_use_action_hover_color() {
    let mut ui = runtime();
    let queue = ToastQueue::new(&ui);
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(
            queue.clone(),
            Element::new("content", Label::new("Content")),
        ),
    ))
    .unwrap();
    let _receipt = queue
        .push(Toast::new("Saved").action("Undo", |_| Ok(())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let button = ui
        .theme()
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::Button);
    let rect = ui
        .paint()
        .unwrap()
        .commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect { rect, color } if *color == button => Some(*rect),
            DrawCommand::DrawMask { rect, color, .. } if *color == button => Some(*rect),
            _ => None,
        })
        .expect("toast action is painted");
    move_to(
        &mut ui,
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    );
    assert!(count(&mut ui, ColorRole::ButtonHover) > 0);
    move_to(&mut ui, 500.0, 200.0);
    assert_eq!(count(&mut ui, ColorRole::ButtonHover), 0);
}
