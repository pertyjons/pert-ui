//! Disabled framed actions have a neutral boundary, without changing admission.
use crate::support;
use pert_ui::testing::Driver;
use pert_ui::*;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime() -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 240.0, 1.0)?)?;
    Ok(ui)
}

use support::uses;

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

/// A background, foreground and border color, each as reported.
type Colors = (Option<[f32; 4]>, Option<[f32; 4]>, Option<[f32; 4]>);

/// The background, foreground and border color a component reports as last
/// painted.
fn painted(ui: &Runtime, id: &WidgetId) -> TestResult<Colors> {
    let inspection = ui.inspection_id(id)?;
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.id == inspection)
        .ok_or("inspected")?;
    let model = component.details.box_model;
    Ok((
        model.background,
        model.foreground,
        model.border.map(|border| border.color),
    ))
}

#[test]
fn a_button_reports_the_colors_it_last_painted() -> TestResult {
    let mut ui = runtime()?;
    let enabled = ui.signal(true);
    let framed = ui.mount_tree(
        Button::new("Apply", |_| Ok(()))
            .enabled(&enabled)
            .key("framed"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        painted(&ui, &framed)?,
        (None, None, None),
        "nothing before a paint"
    );
    ui.paint()?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    let channels = |role| Some(palette.color(role).channels());
    assert_eq!(
        painted(&ui, &framed)?,
        (
            channels(ColorRole::Button),
            channels(ColorRole::OnButton),
            None
        )
    );
    enabled.set(false)?;
    ui.flush(ui.now())?;
    ui.paint()?;
    assert_eq!(
        painted(&ui, &framed)?,
        (
            channels(ColorRole::Surface),
            channels(ColorRole::Muted),
            channels(ColorRole::Outline)
        ),
        "the neutral inactive frame"
    );
    enabled.set(true)?;
    ui.flush(ui.now())?;
    ui.paint()?;
    assert_eq!(
        painted(&ui, &framed)?.2,
        None,
        "enabling removes the outline"
    );
    // A theme change shows after the next paint.
    let custom = Color::from_srgb8(10, 120, 200, 255);
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Button, custom)?;
    ui.flush(ui.now())?;
    ui.paint()?;
    assert_eq!(painted(&ui, &framed)?.0, Some(custom.channels()));
    // Inspecting again changes nothing and needs no work.
    let before = painted(&ui, &framed)?;
    assert_eq!(painted(&ui, &framed)?, before);
    Ok(())
}

#[test]
fn a_frameless_button_reports_a_frame_only_while_it_paints_one() -> TestResult {
    let mut ui = runtime()?;
    let button = ui.mount_tree(Button::new("Tool", |_| Ok(())).frameless().key("tool"))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(painted(&ui, &button)?.0, None, "no frame at rest");
    let bounds = ui.bounds(&button)?.ok_or("laid out")?;
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )?;
    ui.pointer_input(PointerEvent::new(center, PointerKind::Move))?;
    ui.flush(ui.now())?;
    ui.paint()?;
    let hover = ui
        .theme()
        .palette(ThemeMode::Dark)?
        .color(ColorRole::ButtonHover);
    assert_eq!(painted(&ui, &button)?.0, Some(hover.channels()), "hovered");
    Ok(())
}

#[test]
fn disabled_dropdown_anchors_use_the_inactive_button_frame() -> TestResult {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut ui = runtime()?;
        ui.theme().set_mode(mode)?;
        let enabled = ui.signal(false);
        let single = ui.signal(0_u32);
        let several = ui.signal(Vec::<u32>::new());
        let choices = || [Choice::new(0, "Low"), Choice::new(1, "High")];
        let root = ui.mount_tree(
            Column::new([
                Select::new(&ui, single.clone(), choices())?
                    .enabled(&enabled)
                    .key("single-choice"),
                MultiSelect::new(&ui, several.clone(), choices())?
                    .enabled(&enabled)
                    .key("multi-choice"),
            ])
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        let palette = ui.theme().palette(mode)?;
        let channels = |role| Some(palette.color(role).channels());
        // Each control's anchor is the descendant that reports a frame.
        let anchors: Vec<WidgetId> = ["single-choice", "multi-choice"]
            .into_iter()
            .map(|key| {
                let control = ui.find_descendant(&root, key).unwrap().unwrap();
                let mut pending = vec![control];
                let mut found = None;
                while let Some(id) = pending.pop() {
                    if painted(&ui, &id).is_ok_and(|colors| colors.0.is_some()) {
                        found = Some(id);
                        break;
                    }
                    pending.extend(ui.children(&id).unwrap());
                }
                found.expect("a framed anchor")
            })
            .collect();
        for anchor in &anchors {
            assert_eq!(
                painted(&ui, anchor)?,
                (
                    channels(ColorRole::Surface),
                    channels(ColorRole::Muted),
                    channels(ColorRole::Outline)
                ),
                "{mode:?}: a disabled anchor"
            );
        }
        enabled.set(true)?;
        ui.flush(ui.now())?;
        ui.paint()?;
        for anchor in &anchors {
            let (background, foreground, border) = painted(&ui, anchor)?;
            assert_eq!(background, channels(ColorRole::Button), "{mode:?}: enabled");
            assert_eq!(foreground, channels(ColorRole::OnButton));
            assert_eq!(border, None);
        }
    }
    Ok(())
}

/// The colors a painted scene draws inside `bounds`: fills, vector shapes
/// whose origin lies there and text masks inside it.
fn drawn_in(scene: &DisplayList, bounds: Rect) -> Vec<Color> {
    let inside = |rect: &Rect| {
        let (x, y) = (
            rect.x() + rect.width() / 2.0,
            rect.y() + rect.height() / 2.0,
        );
        x >= bounds.x()
            && x <= bounds.x() + bounds.width()
            && y >= bounds.y()
            && y <= bounds.y() + bounds.height()
    };
    scene
        .commands()
        .iter()
        .flat_map(|command| match command {
            DrawCommand::DrawVector { origin, color, .. }
                if origin.x() >= bounds.x() - 0.5
                    && origin.x() <= bounds.x() + bounds.width()
                    && origin.y() >= bounds.y() - 0.5
                    && origin.y() <= bounds.y() + bounds.height() =>
            {
                vec![*color]
            }
            _ if support::extent(command).is_some_and(|rect| inside(&rect)) => {
                support::paints(command)
            }
            _ => Vec::new(),
        })
        .collect()
}

/// The descendant of `control` (or itself) that reports a painted frame.
fn framed_anchor(ui: &Runtime, control: WidgetId) -> TestResult<WidgetId> {
    let mut pending = vec![control];
    while let Some(id) = pending.pop() {
        if painted(ui, &id)?.0.is_some() {
            return Ok(id);
        }
        pending.extend(ui.children(&id)?);
    }
    Err("a framed anchor".into())
}

#[test]
fn disabled_dropdown_anchors_draw_the_inactive_frame_and_follow_roles_and_border_width()
-> TestResult {
    let mut ui = runtime()?;
    let several = ui.signal(vec![1_u32]);
    let single = ui.signal(0_u32);
    let choices = || [Choice::new(0, "Low"), Choice::new(1, "High")];
    let root = ui.mount_tree(
        Column::new([
            Select::new(&ui, single.clone(), choices())?
                .disabled()
                .key("single-choice"),
            // A selected option shows as a chip on the disabled trigger.
            MultiSelect::new(&ui, several.clone(), choices())?
                .disabled()
                .key("multi-choice"),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    // The anchors report their frame once painted.
    let scene = ui.paint()?;
    let anchors = ["single-choice", "multi-choice"]
        .into_iter()
        .map(|key| framed_anchor(&ui, ui.find_descendant(&root, key).unwrap().unwrap()))
        .collect::<TestResult<Vec<_>>>()?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    for anchor in &anchors {
        let drawn = drawn_in(&scene, ui.bounds(anchor)?.ok_or("laid out")?);
        for role in [ColorRole::Surface, ColorRole::Outline, ColorRole::Muted] {
            assert!(
                drawn.contains(&palette.color(role)),
                "{role:?} drawn: {drawn:?}"
            );
        }
        assert!(
            !drawn.contains(&palette.color(ColorRole::Button)),
            "no enabled fill"
        );
    }
    // The light palette's inactive frame is drawn the same way.
    ui.theme().set_mode(ThemeMode::Light)?;
    ui.flush(ui.now())?;
    let scene = ui.paint()?;
    let light = ui.theme().palette(ThemeMode::Light)?;
    for anchor in &anchors {
        let drawn = drawn_in(&scene, ui.bounds(anchor)?.ok_or("laid out")?);
        for role in [ColorRole::Surface, ColorRole::Outline, ColorRole::Muted] {
            assert!(
                drawn.contains(&light.color(role)),
                "light {role:?}: {drawn:?}"
            );
        }
        assert!(!drawn.contains(&light.color(ColorRole::Button)));
    }
    ui.theme().set_mode(ThemeMode::Dark)?;
    ui.flush(ui.now())?;
    // An adaptive override of the inactive fill reaches both anchors.
    let surface = Color::from_srgb8(23, 51, 91, 255);
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Surface, surface)?;
    ui.flush(ui.now())?;
    let scene = ui.paint()?;
    for anchor in &anchors {
        assert_eq!(painted(&ui, anchor)?.0, Some(surface.channels()));
        assert!(drawn_in(&scene, ui.bounds(anchor)?.ok_or("laid out")?).contains(&surface));
    }
    // A zero border width paints and reports no outline.
    let metrics = ui
        .theme()
        .metrics(Density::Standard)?
        .with_metric(MetricRole::BorderWidth, Length::from(0))?;
    ui.theme().set_metrics(Density::Standard, metrics)?;
    ui.flush(ui.now())?;
    let scene = ui.paint()?;
    for anchor in &anchors {
        assert_eq!(painted(&ui, anchor)?.2, None, "no border reported");
        let drawn = drawn_in(&scene, ui.bounds(anchor)?.ok_or("laid out")?);
        assert!(
            !drawn.contains(&palette.color(ColorRole::Outline)),
            "none drawn"
        );
    }
    Ok(())
}

#[test]
fn a_pressed_frameless_button_reports_its_pressed_frame_and_inspection_schedules_nothing()
-> TestResult {
    let mut ui = runtime()?;
    let button = ui.mount_tree(Button::new("Tool", |_| Ok(())).frameless().key("tool"))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let bounds = ui.bounds(&button)?.ok_or("laid out")?;
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )?;
    ui.pointer_input(PointerEvent::new(
        center,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.flush(ui.now())?;
    ui.paint()?;
    let pressed = ui
        .theme()
        .palette(ThemeMode::Dark)?
        .color(ColorRole::ButtonPressed);
    assert_eq!(painted(&ui, &button)?.0, Some(pressed.channels()));
    // Inspecting reads the record: no deadline, no pending work, the same
    // answer twice, and no layout or paint work follows from it.
    let deadline = ui.next_deadline();
    ui.set_profiling(true)?;
    ui.clear_profile()?;
    let first = painted(&ui, &button)?;
    assert_eq!(painted(&ui, &button)?, first);
    assert_eq!(ui.next_deadline(), deadline);
    assert_eq!(
        ui.flush(ui.now())?.components_updated,
        0,
        "nothing to update"
    );
    ui.paint()?;
    let work: Vec<_> = ui
        .profile()
        .stats
        .into_iter()
        .filter(|stat| {
            matches!(
                stat.operation,
                ProfileOperation::Layout
                    | ProfileOperation::Measure
                    | ProfileOperation::Arrange
                    | ProfileOperation::Paint
            )
        })
        .collect();
    assert!(work.is_empty(), "the retained paint is reused: {work:?}");
    Ok(())
}

/// The masks drawn wholly inside `bounds`, as mask and color.
fn masks_in(scene: &DisplayList, bounds: Rect) -> Vec<(MaskId, Color)> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, mask, color, ..
            } if rect.x() >= bounds.x()
                && rect.x() + rect.width() <= bounds.x() + bounds.width()
                && rect.y() >= bounds.y()
                && rect.y() + rect.height() <= bounds.y() + bounds.height() =>
            {
                Some((*mask, *color))
            }
            _ => None,
        })
        .collect()
}

/// The colors of the masks inside `bounds` that a reference control inside
/// `reference` does not draw. Masks are shared within a scene by content,
/// so two controls that differ in one part (an icon, a label) differ in
/// exactly that part's masks: this identifies a part's coverage without
/// guessing it from its shape.
fn differing(scene: &DisplayList, bounds: Rect, reference: Rect) -> Vec<Color> {
    let shared: Vec<MaskId> = masks_in(scene, reference)
        .into_iter()
        .map(|(mask, _)| mask)
        .collect();
    masks_in(scene, bounds)
        .into_iter()
        .filter(|(mask, _)| !shared.contains(mask))
        .map(|(_, color)| color)
        .collect()
}

fn bounds_of(ui: &Runtime, id: &WidgetId) -> TestResult<Rect> {
    Ok(ui.bounds(id)?.ok_or("laid out")?)
}

/// Paints and checks the disabled anchors' muted parts in `muted`: the
/// anchor's icon against its icon-less reference, the chip's label against
/// a chip with another label, and both anchors' reported foreground.
fn check_muted_parts(ui: &mut Runtime, parts: &[WidgetId; 5], muted: Color) -> TestResult {
    let [anchor, reference, trigger, chip, other_chip] = parts;
    let scene = ui.paint()?;
    let palette = ui.theme().palette(ThemeMode::Dark)?;
    // The icon is the anchor's only part the reference lacks: drawn, and
    // muted.
    let icon = differing(&scene, bounds_of(ui, anchor)?, bounds_of(ui, reference)?);
    assert!(!icon.is_empty(), "the icon is drawn");
    assert!(icon.iter().all(|color| *color == muted), "{icon:?}");
    // The chip's label is its only part the other chip lacks: drawn, and
    // muted. Its container keeps its color, and nothing in the chip uses the
    // active label color.
    let label = differing(&scene, bounds_of(ui, chip)?, bounds_of(ui, other_chip)?);
    assert!(!label.is_empty(), "the label is drawn");
    assert!(label.iter().all(|color| *color == muted), "{label:?}");
    let own: Vec<Color> = masks_in(&scene, bounds_of(ui, chip)?)
        .into_iter()
        .map(|(_, color)| color)
        .collect();
    assert!(
        !own.contains(&palette.color(ColorRole::OnSecondaryContainer)),
        "{own:?}"
    );
    let fills = drawn_in(&scene, bounds_of(ui, chip)?);
    assert!(
        fills.contains(&palette.color(ColorRole::SecondaryContainer)),
        "{fills:?}"
    );
    // Both anchors report the muted foreground they painted.
    for framed in [anchor, trigger] {
        assert_eq!(painted(ui, framed)?.1, Some(muted.channels()));
    }
    Ok(())
}

#[test]
fn disabled_dropdown_anchors_mute_icon_captions_and_chips_and_follow_overridden_roles() -> TestResult
{
    let mut ui = runtime()?;
    // Each control has a reference that differs in one part: the same
    // caption without its icon, and a chip with another label.
    let iconic = || {
        [
            Choice::new(0, "Low").leading_icon(Icon::Info),
            Choice::new(1, "High"),
        ]
    };
    let plain = || [Choice::new(0, "Low"), Choice::new(1, "High")];
    let longer = || [Choice::new(0, "Low"), Choice::new(1, "Highest")];
    let root = ui.mount_tree(
        Column::new([
            Select::new(&ui, ui.signal(0_u32), iconic())?
                .disabled()
                .key("single-choice"),
            Select::new(&ui, ui.signal(0_u32), plain())?
                .disabled()
                .key("single-plain"),
            MultiSelect::new(&ui, ui.signal(vec![1_u32]), plain())?
                .disabled()
                .key("multi-choice"),
            MultiSelect::new(&ui, ui.signal(vec![1_u32]), longer())?
                .disabled()
                .key("multi-longer"),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let find = |key: &str| -> TestResult<WidgetId> {
        Ok(ui.find_descendant(&root, key)?.ok_or("mounted")?)
    };
    let anchor = framed_anchor(&ui, find("single-choice")?)?;
    let reference = framed_anchor(&ui, find("single-plain")?)?;
    let trigger = framed_anchor(&ui, find("multi-choice")?)?;
    let chip_of = |control: WidgetId| -> TestResult<WidgetId> {
        let chips = ui.find_descendant(&control, "chips")?.ok_or("chips")?;
        Ok(ui.children(&chips)?.into_iter().next().ok_or("a chip")?)
    };
    let chip = chip_of(find("multi-choice")?)?;
    let other_chip = chip_of(find("multi-longer")?)?;
    let parts = [anchor.clone(), reference, trigger.clone(), chip, other_chip];
    let muted = ui.theme().palette(ThemeMode::Dark)?.color(ColorRole::Muted);
    check_muted_parts(&mut ui, &parts, muted)?;
    // Overridden outline and muted roles reach both anchors and the chip,
    // in what they draw and in what they report.
    let outline = Color::from_srgb8(91, 51, 23, 255);
    let override_muted = Color::from_srgb8(51, 91, 23, 255);
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Outline, outline)?;
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Muted, override_muted)?;
    ui.flush(ui.now())?;
    check_muted_parts(&mut ui, &parts, override_muted)?;
    let scene = ui.paint()?;
    for framed in [&anchor, &trigger] {
        let drawn = drawn_in(&scene, bounds_of(&ui, framed)?);
        assert!(drawn.contains(&outline), "{drawn:?}");
        assert_eq!(painted(&ui, framed)?.2, Some(outline.channels()));
    }
    Ok(())
}
