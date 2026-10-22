//! The keyboard focus ring and pressed colors, through the public facade:
//! rendered pixels on a pressed button in both built-in palettes, the
//! independence of the focus color from the `Primary` accent, and pressed
//! colors that follow it by default.
use pert_ui::*;
use std::time::Duration;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn runtime(scale: f64) -> std::result::Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(240.0, 120.0, scale)?)?;
    Ok(ui)
}

/// Mounts a button, focuses it and holds the primary pointer button on it.
fn pressed_focused_button(
    ui: &mut Runtime,
) -> std::result::Result<Rect, Box<dyn std::error::Error>> {
    let id = ui.mount_tree(Element::new("button", Button::new("Apply", |_| Ok(()))))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    let bounds = ui.bounds(&id)?.ok_or("button has no bounds")?;
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )?;
    for kind in [
        PointerKind::Move,
        PointerKind::Press(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(center, kind))?;
    }
    ui.flush(Duration::ZERO)?;
    Ok(bounds)
}

/// Whether any fill or mask in the scene is drawn in `color`.
fn uses(scene: &DisplayList, color: Color) -> bool {
    scene.commands().iter().any(|command| {
        matches!(command,
            DrawCommand::FillRect { color: drawn, .. }
            | DrawCommand::DrawMask { color: drawn, .. }
            if *drawn == color)
    })
}

/// The rendered pixels across the left edge of a focused, pressed button: the
/// ring band and the pressed fill, each sampled at the
/// middle of its logical extent on the button's vertical center.
#[cfg(feature = "cpu")]
#[test]
fn the_rendered_ring_has_no_inner_band_over_a_pressed_fill() -> Result {
    use pert_ui::cpu::CpuRenderer;
    // Distinct opaque values for the identity check, then both built-in
    // palettes for the visibility check.
    let tagged = Palette::dark()
        .with_color(ColorRole::FocusRing, Color::from_srgb8(230, 20, 20, 255))
        .with_color(
            ColorRole::FocusRingContrast,
            Color::from_srgb8(20, 230, 20, 255),
        )
        .with_color(
            ColorRole::ButtonPressed,
            Color::from_srgb8(20, 20, 230, 255),
        );
    let cases = [
        (ThemeMode::Dark, Some(tagged)),
        (ThemeMode::Dark, None),
        (ThemeMode::Light, None),
    ];
    for (mode, palette) in cases {
        let scale = 2.0;
        let mut ui = runtime(scale)?;
        ui.theme().set_mode(mode)?;
        let tagged_case = palette.is_some();
        if let Some(palette) = palette {
            ui.theme().set_palette(mode, palette)?;
        }
        let bounds = pressed_focused_button(&mut ui)?;
        let palette = ui.theme().palette(mode)?;
        let metrics = ui.theme().metrics(Density::Standard)?;
        let ring = metrics.metric(MetricRole::FocusWidth);
        let scene = ui.paint()?;
        if tagged_case {
            assert!(!uses(&scene, Color::from_srgb8(20, 230, 20, 255)));
        }
        let image = CpuRenderer::new().render(&scene)?;
        let y = (f64::from(bounds.y() + bounds.height() / 2.0) * scale) as u32;
        let pixel = |logical_x: f32| {
            let x = (f64::from(logical_x) * scale) as u32;
            let start = ((y * image.width() + x) * 4) as usize;
            let mut rgba = [0; 4];
            rgba.copy_from_slice(&image.pixels()[start..start + 4]);
            rgba
        };
        for (offset, role) in [
            (ring / 2.0, ColorRole::FocusRing),
            (ring + 0.5, ColorRole::ButtonPressed),
            (ring + 2.0, ColorRole::ButtonPressed),
        ] {
            let expected = palette.color(role).to_srgb8();
            let found = pixel(bounds.x() + offset);
            assert!(
                found.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "{mode:?} {role:?} at +{offset}: {found:?} vs {expected:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn a_focused_pressed_button_shows_its_ring_in_both_built_in_palettes() -> Result {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut ui = runtime(1.0)?;
        ui.theme().set_mode(mode)?;
        pressed_focused_button(&mut ui)?;
        let scene = ui.paint()?;
        let palette = ui.theme().palette(mode)?;
        let color = |role| palette.color(role);
        let opaque = |role| OpaqueColor::try_from(color(role));
        for role in [
            ColorRole::FocusRing,
            ColorRole::ButtonPressed,
            ColorRole::OnButtonPressed,
        ] {
            assert!(uses(&scene, color(role)), "{mode:?} {role:?} is drawn");
        }
        let ring = opaque(ColorRole::FocusRing)?;
        let around = opaque(ColorRole::Background)?;
        let ratio = ring.contrast_ratio(around);
        assert!(ratio >= 3.0, "{mode:?} ring on background: {ratio:.2}");
    }
    Ok(())
}

fn accent_palette() -> std::result::Result<(Palette, Color), Box<dyn std::error::Error>> {
    let accent = Color::from_linear(0.7, 0.0, 0.7, 1.0)?;
    let palette = Palette::dark()
        .with_color(ColorRole::Primary, accent)
        .with_color(ColorRole::OnPrimary, Color::WHITE);
    Ok((palette, accent))
}

#[test]
fn overriding_primary_moves_pressed_colors_and_leaves_focus_alone() -> Result {
    let mut ui = runtime(1.0)?;
    let (palette, accent) = accent_palette()?;
    ui.theme().set_palette(ThemeMode::Dark, palette)?;
    pressed_focused_button(&mut ui)?;
    let scene = ui.paint()?;
    assert!(uses(&scene, accent), "the pressed fill follows Primary");
    let defaults = Palette::dark();
    assert!(uses(&scene, defaults.color(ColorRole::FocusRing)));
    // An explicit pressed color is kept although Primary is overridden.
    let mut ui = runtime(1.0)?;
    let pressed = Color::from_linear(0.1, 0.5, 0.1, 1.0)?;
    ui.theme().set_palette(
        ThemeMode::Dark,
        palette.with_color(ColorRole::ButtonPressed, pressed),
    )?;
    pressed_focused_button(&mut ui)?;
    let scene = ui.paint()?;
    assert!(uses(&scene, pressed));
    assert!(!uses(&scene, accent));
    Ok(())
}

#[test]
fn an_accent_matching_the_background_keeps_focus_visible() -> Result {
    for (mode, defaults) in [
        (ThemeMode::Dark, Palette::dark()),
        (ThemeMode::Light, Palette::light()),
    ] {
        let background = defaults.color(ColorRole::Background);
        let palette = defaults.with_color(ColorRole::Primary, background);
        let focus = defaults.color(ColorRole::FocusRing);
        assert_ne!(focus, background);

        let mut ui = runtime(1.0)?;
        ui.theme().set_palette(mode, palette)?;
        ui.theme().set_mode(mode)?;
        pressed_focused_button(&mut ui)?;
        let scene = ui.paint()?;
        assert!(uses(&scene, focus), "{mode:?} button keeps its ring");

        let mut ui = runtime(1.0)?;
        ui.theme().set_palette(mode, palette)?;
        ui.theme().set_mode(mode)?;
        let split = ui.mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("start", Label::new("Start")),
                Element::new("end", Label::new("End")),
            ),
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&split)?;
        ui.flush(Duration::ZERO)?;
        assert!(
            uses(&ui.paint()?, focus),
            "{mode:?} focused divider keeps FocusRing"
        );
    }
    Ok(())
}

#[test]
fn a_focused_divider_keeps_the_focus_color_while_dragged() -> Result {
    let mut ui = runtime(1.0)?;
    let (palette, accent) = accent_palette()?;
    ui.theme().set_palette(ThemeMode::Dark, palette)?;
    let split = ui.mount_tree(Element::new(
        "split",
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("start", Label::new("Start")),
            Element::new("end", Label::new("End")),
        ),
    ))?;
    ui.flush(Duration::ZERO)?;
    let start = ui
        .find_descendant(&split, "start")?
        .ok_or("no start pane")?;
    let before = ui.bounds(&start)?.ok_or("start has no bounds")?;
    ui.focus(&split)?;
    ui.flush(Duration::ZERO)?;
    let focus = Palette::dark().color(ColorRole::FocusRing);
    let scene = ui.paint()?;
    assert!(!uses(&scene, accent));
    assert!(uses(&scene, focus));

    // Drag the divider from just right of the start pane.
    let y = before.y() + before.height() / 2.0;
    let grab = Point::new(before.x() + before.width() + 2.0, y)?;
    let to = Point::new(grab.x() - 30.0, y)?;
    for (point, kind) in [
        (grab, PointerKind::Move),
        (grab, PointerKind::Press(PointerButton::Primary)),
        (to, PointerKind::Move),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind))?;
    }
    ui.flush(Duration::ZERO)?;
    let after = ui.bounds(&start)?.ok_or("start has no bounds")?;
    assert!(after.width() < before.width(), "the drag resized the panes");
    let scene = ui.paint()?;
    assert!(
        !uses(&scene, accent),
        "a dragged focused divider stays FocusRing"
    );
    assert!(uses(&scene, focus));
    Ok(())
}
