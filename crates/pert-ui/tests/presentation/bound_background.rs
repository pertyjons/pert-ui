//! `Container::background_role_signal` follows a bound role with repaint
//! alone, replaces and is replaced by the literal builders, and rejects a
//! binding of another runtime.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn background(ui: &Runtime) -> Option<[f32; 4]> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == "strip")
        .and_then(|component| component.details.box_model.background)
}

/// The current palette's color for `role`.
fn role_color(ui: &Runtime, role: ColorRole) -> Result<[f32; 4], RuntimeError> {
    let theme = ui.theme();
    Ok(theme.palette(theme.mode()?)?.color(role).channels())
}

fn strip(container: Container) -> Element {
    Element::new("strip", container)
}

fn frame(ui: &mut Runtime, at: u64) -> TestResult {
    ui.flush(Duration::from_millis(at))?;
    ui.paint()?;
    Ok(())
}

#[test]
fn a_bound_role_repaints_without_layout() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let role = ui.signal(ColorRole::Surface);
    let surface = role_color(&ui, ColorRole::Surface)?;
    let selected = role_color(&ui, ColorRole::PrimaryContainer)?;
    ui.mount_tree(strip(
        Container::new(Element::new("name", Label::new("Strip")))
            .background_role_signal(role.clone()),
    ))?;
    frame(&mut ui, 0)?;
    assert_eq!(background(&ui), Some(surface));
    ui.set_profiling(true)?;
    ui.clear_profile()?;
    role.set(ColorRole::PrimaryContainer)?;
    frame(&mut ui, 1)?;
    assert_eq!(background(&ui), Some(selected));
    assert!(ui.profile().stats.iter().all(|stat| !matches!(
        stat.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    Ok(())
}

#[test]
fn builders_replace_each_other_and_release_the_old_binding() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let role = ui.signal(ColorRole::Surface);
    let error = role_color(&ui, ColorRole::Error)?;
    let literal = Color::from_linear(0.25, 0.5, 0.75, 1.0)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([strip(
            Container::new(Element::new("name", Label::new("Strip")))
                .background_role_signal(role.clone()),
        )]),
    ))?;
    frame(&mut ui, 0)?;
    // A literal color described later replaces the binding.
    ui.reconcile_children(
        &root,
        vec![strip(
            Container::new(Element::new("name", Label::new("Strip")))
                .background_role_signal(role.clone())
                .background(literal),
        )],
    )?;
    frame(&mut ui, 1)?;
    assert_eq!(background(&ui), Some(literal.channels()));
    role.set(ColorRole::Error)?;
    frame(&mut ui, 2)?;
    assert_eq!(
        background(&ui),
        Some(literal.channels()),
        "the old binding is released"
    );
    // A binding described later replaces a fixed role.
    ui.reconcile_children(
        &root,
        vec![strip(
            Container::new(Element::new("name", Label::new("Strip")))
                .background_role(ColorRole::Surface)
                .background_role_signal(role.clone()),
        )],
    )?;
    frame(&mut ui, 3)?;
    assert_eq!(background(&ui), Some(error));
    // Unmounting releases the binding: writing it schedules nothing.
    ui.unmount(&root)?;
    role.set(ColorRole::Surface)?;
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

#[test]
fn a_binding_of_another_runtime_fails_the_update() -> TestResult {
    let other = Runtime::new();
    let foreign = other.signal(ColorRole::Surface);
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.mount_tree(strip(
        Container::new(Element::new("name", Label::new("Strip"))).background_role_signal(foreign),
    ))?;
    assert_eq!(
        ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::ForeignRuntime
    );
    Ok(())
}

#[test]
fn the_pause_icon_draws_and_can_be_replaced() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "pause",
        Button::icon(Icon::Pause, "Pause", |_| Ok(())),
    ))?;
    frame(&mut ui, 0)?;
    // Like every role it can be replaced and restored.
    let icons = ui.icons();
    icons.set(Icon::Pause, ui.fonts().material_icon("pause_circle")?)?;
    frame(&mut ui, 1)?;
    icons.reset(Icon::Pause)?;
    frame(&mut ui, 2)?;
    Ok(())
}
