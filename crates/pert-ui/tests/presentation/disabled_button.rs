//! Disabled framed actions have a neutral boundary, without changing admission.
use pert_ui::testing::Driver;
use pert_ui::*;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime() -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 240.0, 1.0)?)?;
    Ok(ui)
}

fn uses(scene: &DisplayList, color: Color) -> bool {
    scene.commands().iter().any(|command| {
        matches!(command,
            DrawCommand::FillRect { color: found, .. }
            | DrawCommand::DrawMask { color: found, .. }
            if *found == color)
    })
}

#[test]
fn revocation_repaints_a_held_button_and_reenable_does_not_replay_it() -> TestResult {
    let mut ui = runtime()?;
    let enabled = ui.signal(true);
    let count = ui.signal(0_u32);
    let seen = count.clone();
    let button = ui.mount_tree(
        Button::new("Apply", move |_| seen.update(|value| *value += 1))
            .enabled(&enabled)
            .key("button"),
    )?;
    ui.flush(Duration::ZERO)?;
    let bounds = ui.bounds(&button)?.ok_or("button bounds")?;
    let point = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )?;
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.flush(ui.now())?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    assert!(uses(&ui.paint()?, palette.color(ColorRole::ButtonPressed)));

    enabled.set(false)?;
    ui.flush(ui.now())?;
    let disabled = ui.paint()?;
    assert!(uses(&disabled, palette.color(ColorRole::Surface)));
    assert!(uses(&disabled, palette.color(ColorRole::Outline)));
    assert!(uses(&disabled, palette.color(ColorRole::Muted)));
    assert!(!uses(&disabled, palette.color(ColorRole::ButtonPressed)));
    assert_eq!(count.get()?, 0);
    assert_eq!(ui.bounds(&button)?, Some(bounds));

    enabled.set(true)?;
    ui.flush(ui.now())?;
    ui.pointer_input(PointerEvent::new(Point::new(901.0, 241.0)?, PointerKind::Move).outside())?;
    ui.flush(ui.now())?;
    let enabled_again = ui.paint()?;
    assert!(uses(&enabled_again, palette.color(ColorRole::Button)));
    assert!(!uses(&enabled_again, palette.color(ColorRole::Outline)));
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert_eq!(count.get()?, 0, "a revoked hold never replays on reenable");
    Driver::new(&mut ui).click(&button)?;
    assert_eq!(count.get()?, 1, "a fresh gesture still works");
    Ok(())
}

#[test]
fn disabled_role_and_border_overrides_repaint_without_layout() -> TestResult {
    let mut ui = runtime()?;
    ui.set_profiling(true)?;
    let button = ui.mount_tree(Button::new("Apply", |_| Ok(())).disabled().key("button"))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let bounds = ui.bounds(&button)?;
    for (role, color) in [
        (ColorRole::Surface, Color::from_srgb8(23, 51, 91, 255)),
        (ColorRole::Outline, Color::from_srgb8(201, 52, 29, 255)),
        (ColorRole::Muted, Color::from_srgb8(182, 220, 52, 255)),
    ] {
        ui.clear_profile()?;
        ui.theme().set_color(ThemeMode::Dark, role, color)?;
        ui.flush(ui.now())?;
        assert!(uses(&ui.paint()?, color), "{role:?} is adaptive");
        assert_eq!(ui.bounds(&button)?, bounds);
        assert!(
            ui.profile()
                .stats
                .iter()
                .all(|stat| stat.operation != ProfileOperation::Layout)
        );
    }
    for width in [3_u16, 0] {
        ui.clear_profile()?;
        let metrics = ui
            .theme()
            .metrics(Density::Standard)?
            .with_metric(MetricRole::BorderWidth, Length::from(width))?;
        ui.theme().set_metrics(Density::Standard, metrics)?;
        ui.flush(ui.now())?;
        let scene = ui.paint()?;
        let outline = ui
            .theme()
            .palette(ThemeMode::Dark)?
            .color(ColorRole::Outline);
        assert_eq!(uses(&scene, outline), width != 0);
        assert_eq!(ui.bounds(&button)?, bounds);
        assert!(
            ui.profile()
                .stats
                .iter()
                .all(|stat| stat.operation != ProfileOperation::Layout)
        );
    }
    Ok(())
}

#[test]
fn disabled_frameless_and_contextual_command_buttons_keep_their_contracts() -> TestResult {
    use pert_ui::testing::HostIntegration;
    let mut ui = runtime()?;
    let frameless = ui.mount_tree(
        Button::new("Inactive", |_| Ok(()))
            .disabled()
            .frameless()
            .key("frameless"),
    )?;
    ui.flush(Duration::ZERO)?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    let scene = ui.paint()?;
    for role in [ColorRole::Button, ColorRole::Surface, ColorRole::Outline] {
        assert!(!uses(&scene, palette.color(role)), "no {role:?} frame");
    }
    assert!(uses(&scene, palette.color(ColorRole::Muted)));
    assert!(Driver::new(&mut ui).semantics(&frameless)?.disabled);
    ui.theme().set_color(
        ThemeMode::Dark,
        ColorRole::Surface,
        Color::from_srgb8(23, 51, 91, 255),
    )?;
    let metrics = ui
        .theme()
        .metrics(Density::Standard)?
        .with_metric(MetricRole::CornerRadius, Length::from(9))?
        .with_metric(MetricRole::BorderWidth, Length::from(3))?;
    ui.theme().set_metrics(Density::Standard, metrics)?;
    assert!(!ui.needs_paint(), "undrawn frame roles schedule no paint");
    assert_eq!(ui.next_deadline(), None);
    assert!(
        ui.inspect()
            .components
            .iter()
            .all(|entry| entry.dirty.is_none())
    );

    let mut ui = runtime()?;
    let command =
        ui.register_command("Contextual", CommandScope::Application, vec![], |_, _| {
            Ok(())
        })?;
    ui.command_enabled(&command, |_, _| Ok(false))?;
    let contextual =
        ui.mount_tree(Button::command(&ui, &command, |_, _| Ok(()))?.key("contextual"))?;
    ui.flush(Duration::ZERO)?;
    let mut driver = Driver::new(&mut ui);
    assert!(
        !driver.semantics(&contextual)?.disabled,
        "a contextual predicate is admission, not enablement"
    );
    let palette = driver.runtime().theme().palette(ThemeMode::Dark)?;
    let scene = driver.runtime_mut().paint()?;
    assert!(uses(&scene, palette.color(ColorRole::Button)));
    assert!(!uses(&scene, palette.color(ColorRole::Surface)));
    assert!(!uses(&scene, palette.color(ColorRole::Outline)));
    Ok(())
}

#[test]
fn disabled_menu_triggers_inherit_framed_or_icon_presentation() -> TestResult {
    for icon in [false, true] {
        let mut ui = runtime()?;
        let menu = if icon {
            Menu::icon(
                &ui,
                Icon::MoreHorizontal,
                "Unavailable",
                Vec::<MenuItem>::new(),
            )?
        } else {
            Menu::new(&ui, "Unavailable", Vec::<MenuItem>::new())?
        };
        let menu = ui.mount_tree(menu.key("menu"))?;
        let mut driver = Driver::new(&mut ui);
        driver.settle()?;
        let trigger = driver.exposed_part(&menu, "trigger")?;
        assert!(driver.semantics(&trigger)?.disabled);
        let palette = driver.runtime().theme().palette(ThemeMode::Dark)?;
        let scene = driver.runtime_mut().paint()?;
        assert_eq!(uses(&scene, palette.color(ColorRole::Surface)), !icon);
        assert_eq!(uses(&scene, palette.color(ColorRole::Outline)), !icon);
        assert!(!uses(&scene, palette.color(ColorRole::Button)));
        assert!(uses(&scene, palette.color(ColorRole::Muted)));
    }
    Ok(())
}

#[cfg(feature = "cpu")]
#[test]
fn cpu_pixels_and_adjacent_fields_show_the_shared_inactive_frame() -> TestResult {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut ui = runtime()?;
        ui.theme().set_mode(mode)?;
        let root = ui.mount_tree(
            Column::new([
                Row::new([
                    Column::new([
                        Label::new("Enabled Button").key("label"),
                        Button::new("Apply", |_| Ok(())).key("button"),
                    ])
                    .gap(8)
                    .key("enabled"),
                    Column::new([
                        Label::new("Disabled Button").key("label"),
                        Button::new("Apply", |_| Ok(())).disabled().key("button"),
                    ])
                    .gap(8)
                    .key("disabled"),
                    Column::new([
                        Label::new("Empty TextField").key("label"),
                        TextField::new(ui.signal(String::new()))
                            .placeholder("Empty editor")
                            .key("field"),
                    ])
                    .gap(8)
                    .key("empty"),
                    Column::new([
                        Label::new("Disabled TextField").key("label"),
                        TextField::new(ui.signal("Inactive text".to_owned()))
                            .disabled()
                            .key("field"),
                    ])
                    .gap(8)
                    .key("field"),
                ])
                .gap(24)
                .key("comparison"),
                Button::icon(Icon::Save, "Disabled icon", |_| Ok(()))
                    .disabled()
                    .key("icon"),
                Button::new("No frame", |_| Ok(()))
                    .disabled()
                    .frameless()
                    .key("frameless"),
            ])
            .gap(16)
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        let mut driver = Driver::new(&mut ui);
        driver.settle()?;
        let enabled = driver.find(&root, &["comparison", "enabled", "button"])?;
        let disabled = driver.find(&root, &["comparison", "disabled", "button"])?;
        let icon = driver.find(&root, &["icon"])?;
        let field = driver.find(&root, &["comparison", "field", "field"])?;
        let frameless = driver.find(&root, &["frameless"])?;
        let palette = driver.runtime().theme().palette(mode)?;
        let scene = driver.runtime_mut().paint()?;
        let image = pert_ui::cpu::CpuRenderer::new().render(&scene)?;
        let pixel = |x: f32, y: f32| {
            let index = ((y as u32 * image.width() + x as u32) * 4) as usize;
            <[u8; 4]>::try_from(&image.pixels()[index..index + 4]).unwrap()
        };
        for (id, fill_role, boundary_role) in [
            (&enabled, ColorRole::Button, ColorRole::Button),
            (&disabled, ColorRole::Surface, ColorRole::Outline),
            (&icon, ColorRole::Surface, ColorRole::Outline),
            (&field, ColorRole::Surface, ColorRole::Outline),
            (&frameless, ColorRole::Background, ColorRole::Background),
        ] {
            let bounds = driver.runtime().bounds(id)?.ok_or("control bounds")?;
            for (x, role) in [
                (bounds.x() + 3.0, fill_role),
                (bounds.x() + 0.5, boundary_role),
            ] {
                let actual = pixel(x, bounds.y() + bounds.height() / 2.0);
                let expected = palette.color(role).to_srgb8();
                assert!(
                    actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                    "{mode:?} {bounds:?} {role:?}: {actual:?} vs {expected:?}"
                );
            }
        }
        let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("disabled-button");
        std::fs::create_dir_all(&out)?;
        std::fs::write(out.join(format!("{mode:?}.png")), image.encode_png())?;
    }
    Ok(())
}

#[cfg(feature = "cpu")]
#[test]
fn cpu_border_width_changes_coverage_inward_and_zero_leaves_the_fill() -> TestResult {
    let mut ui = runtime()?;
    let button = ui.mount_tree(Button::new("Apply", |_| Ok(())).disabled().key("button"))?;
    ui.flush(Duration::ZERO)?;
    let bounds = ui.bounds(&button)?.ok_or("button bounds")?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    for (width, samples) in [
        (
            3_u16,
            [(1.5, ColorRole::Outline), (4.0, ColorRole::Surface)],
        ),
        (0, [(1.5, ColorRole::Surface), (4.0, ColorRole::Surface)]),
        (240, [(1.5, ColorRole::Outline), (4.0, ColorRole::Outline)]),
    ] {
        let metrics = ui
            .theme()
            .metrics(Density::Standard)?
            .with_metric(MetricRole::BorderWidth, Length::from(width))?;
        ui.theme().set_metrics(Density::Standard, metrics)?;
        ui.flush(ui.now())?;
        assert_eq!(ui.bounds(&button)?, Some(bounds));
        let image = pert_ui::cpu::CpuRenderer::new().render(&ui.paint()?)?;
        for (offset, role) in samples {
            let x = (bounds.x() + offset) as u32;
            let y = (bounds.y() + bounds.height() / 2.0) as u32;
            let start = ((y * image.width() + x) * 4) as usize;
            let actual = &image.pixels()[start..start + 4];
            assert!(
                actual
                    .iter()
                    .zip(palette.color(role).to_srgb8())
                    .all(|(a, b)| a.abs_diff(b) <= 1),
                "width {width} at {offset}: {actual:?}"
            );
        }
    }
    Ok(())
}
