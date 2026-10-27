//! sRGB conversion, hexadecimal notation, color arithmetic, contrast and the
//! color swatch, through the public facade.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use support::{frame, press, settle, widget};

// The IEC 61966-2-1 decoding written independently of the library, in f64.
fn reference_decode(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[test]
fn every_byte_round_trips_and_decodes_by_the_standard_transfer() {
    for byte in 0..=u8::MAX {
        let color = Color::from_srgb8(byte, byte, byte, byte);
        assert_eq!(color.to_srgb8(), [byte; 4], "byte {byte}");
        let expected = reference_decode(f64::from(byte) / 255.0);
        assert!(
            (f64::from(color.red()) - expected).abs() < 1e-7,
            "byte {byte}: {} vs {expected}",
            color.red()
        );
        // Alpha is a straight fraction, never gamma-decoded.
        assert_eq!(color.alpha(), (f64::from(byte) / 255.0) as f32);
        let hex = color.to_hex();
        assert_eq!(Color::from_hex(&hex).unwrap().to_srgb8(), [byte; 4]);
    }
}

#[test]
fn float_channels_round_trip_and_reject_invalid_input() {
    let color = Color::from_srgb(0.25, 0.5, 0.75, 0.4).unwrap();
    let [r, g, b, a] = color.to_srgb();
    for (value, expected) in [(r, 0.25), (g, 0.5), (b, 0.75)] {
        assert!((value - expected).abs() < 1e-6, "{value} vs {expected}");
    }
    assert_eq!(a, 0.4);
    assert_eq!(color.alpha(), 0.4);
    // The linear segment near black and the ends of the range.
    assert_eq!(Color::from_srgb(0.0, 1.0, 0.04, 1.0).unwrap().red(), 0.0);
    assert_eq!(Color::from_srgb(0.0, 1.0, 0.04, 1.0).unwrap().green(), 1.0);
    let dark = Color::from_srgb(0.0, 0.0, 0.04, 1.0).unwrap();
    assert!((f64::from(dark.blue()) - 0.04 / 12.92).abs() < 1e-9);
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        assert_eq!(
            Color::from_srgb(bad, 0.0, 0.0, 1.0),
            Err(DrawError::InvalidColor)
        );
        assert_eq!(
            Color::from_srgb(0.0, 0.0, 0.0, bad),
            Err(DrawError::InvalidColor)
        );
    }
    // Linear constants convert to sRGB within range.
    for channel in Color::TEXT.to_srgb() {
        assert!((0.0..=1.0).contains(&channel));
    }
}

#[test]
fn hex_notation_accepts_four_forms_and_reports_the_first_problem() {
    let full = Color::from_srgb8(0xff, 0x88, 0x00, 0xff);
    for text in [
        "#ff8800",
        "ff8800",
        "#FF8800",
        "#f80",
        "F80",
        "#ff8800ff",
        "#f80f",
    ] {
        assert_eq!(Color::from_hex(text), Ok(full), "{text}");
        assert_eq!(text.parse::<Color>(), Ok(full), "{text}");
    }
    let translucent = Color::from_hex("#11223344").unwrap();
    assert_eq!(translucent.to_srgb8(), [0x11, 0x22, 0x33, 0x44]);
    assert_eq!(translucent.to_hex(), "#11223344");
    assert_eq!(Color::from_hex("#1234").unwrap().to_hex(), "#11223344");
    assert_eq!(full.to_hex(), "#ff8800");

    for (text, length) in [
        ("", 0),
        ("#", 0),
        ("#12", 2),
        ("#12345", 5),
        ("#1234567", 7),
        ("#123456789", 9),
        ("##123", 0),
    ] {
        let expected = if text == "##123" {
            Err(HexColorError::Digit {
                index: 1,
                character: '#',
            })
        } else {
            Err(HexColorError::Length(length))
        };
        assert_eq!(Color::from_hex(text), expected, "{text:?}");
    }
    for (text, index, character) in [
        (" #fff", 0, ' '),
        ("#fff ", 4, ' '),
        ("0xfff", 1, 'x'),
        ("#ffé", 3, 'é'),
        ("#12345678z", 9, 'z'),
        ("+fff", 0, '+'),
    ] {
        assert_eq!(
            Color::from_hex(text),
            Err(HexColorError::Digit { index, character }),
            "{text:?}"
        );
    }
    assert!(!HexColorError::Length(5).to_string().is_empty());
}

fn opaque(hex: &str) -> OpaqueColor {
    OpaqueColor::try_from(Color::from_hex(hex).unwrap()).unwrap()
}

#[test]
fn contrast_matches_wcag_reference_values() {
    let (white, black) = (opaque("#fff"), opaque("#000"));
    assert_eq!(black.relative_luminance(), 0.0);
    assert_eq!(white.relative_luminance(), 1.0);
    assert_eq!(white.contrast_ratio(black), 21.0);
    assert_eq!(white.contrast_ratio(white), 1.0);
    // #767676 is the lightest gray that reaches 4.5:1 on white: 4.54, while
    // #777777 reaches only 4.48. Both agree with published WCAG calculators.
    let pass = opaque("#767676").contrast_ratio(white);
    let fail = opaque("#777777").contrast_ratio(white);
    assert!((pass - 4.54).abs() < 0.005 && pass >= 4.5, "{pass}");
    assert!((fail - 4.48).abs() < 0.005 && fail < 4.5, "{fail}");
    // Pure primaries weigh by the WCAG coefficients.
    let green = opaque("#00ff00").relative_luminance();
    assert!((green - 0.7152).abs() < 1e-6, "{green}");
    for (a, b) in [("#123456", "#fedcba"), ("#808080", "#202020")] {
        let (a, b) = (opaque(a), opaque(b));
        assert_eq!(a.contrast_ratio(b), b.contrast_ratio(a));
        assert!((1.0..=21.0).contains(&a.contrast_ratio(b)));
    }
}

#[test]
fn only_an_exactly_opaque_color_is_measurable() {
    let almost = Color::from_linear(1.0, 1.0, 1.0, 0.999).unwrap();
    assert_eq!(OpaqueColor::try_from(almost), Err(TranslucentColorError));
    assert!(!TranslucentColorError.to_string().is_empty());
    let opaque = OpaqueColor::try_from(Color::TEXT).unwrap();
    assert_eq!(Color::from(opaque), Color::TEXT);
}

#[test]
fn compositing_over_an_opaque_background_follows_source_over() {
    let back = opaque("#204060");
    let source = Color::from_linear(0.8, 0.6, 0.4, 0.25).unwrap();
    let [r, g, b, a] = Color::from(source.over(back)).channels();
    let [br, bg, bb, _] = Color::from(back).channels();
    for (value, expected) in [
        (r, 0.8 * 0.25 + br * 0.75),
        (g, 0.6 * 0.25 + bg * 0.75),
        (b, 0.4 * 0.25 + bb * 0.75),
    ] {
        assert!((value - expected).abs() < 1e-6, "{value} vs {expected}");
    }
    assert_eq!(a, 1.0);
    // The alpha extremes reproduce one side bit for bit.
    let bits = |color: Color| color.channels().map(f32::to_bits);
    let signed = Color::from_linear(-0.0, 0.5, 0.25, 1.0).unwrap();
    let signed_back = OpaqueColor::try_from(signed).unwrap();
    assert_eq!(
        bits(source.with_alpha(0.0).unwrap().over(signed_back).into()),
        bits(signed)
    );
    assert_eq!(bits(signed.over(back).into()), bits(signed));
    // Channels stay valid at the extremes of every input.
    for value in [0.0, 1.0] {
        for alpha in [0.0, 0.3, 1.0] {
            let source = Color::from_linear(value, value, value, alpha).unwrap();
            for back in ["#000", "#fff"] {
                let out = Color::from(source.over(opaque(back)));
                assert!(out.channels().iter().all(|c| (0.0..=1.0).contains(c)));
            }
        }
    }
}

#[test]
fn alpha_replacement_and_scaling_validate_their_inputs() {
    let color = Color::from_linear(0.2, 0.4, 0.6, 0.5).unwrap();
    assert_eq!(
        color.with_alpha(0.75).unwrap().channels(),
        [0.2, 0.4, 0.6, 0.75]
    );
    assert_eq!(
        color.scale_alpha(0.5).unwrap().channels(),
        [0.2, 0.4, 0.6, 0.25]
    );
    assert_eq!(color.scale_alpha(1.0).unwrap(), color);
    assert_eq!(color.scale_alpha(0.0).unwrap().alpha(), 0.0);
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
        assert_eq!(color.with_alpha(invalid), Err(DrawError::InvalidColor));
        assert_eq!(color.scale_alpha(invalid), Err(DrawError::InvalidColor));
    }
}

#[test]
fn mixing_interpolates_premultiplied_channels() {
    let red = Color::from_linear(1.0, 0.0, 0.0, 1.0).unwrap();
    let faint_blue = Color::from_linear(0.0, 0.0, 1.0, 0.2).unwrap();
    let clear = Color::from_linear(0.0, 0.0, 0.0, 0.0).unwrap();
    // Endpoints are exact.
    assert_eq!(red.mix(faint_blue, 0.0).unwrap(), red);
    assert_eq!(red.mix(faint_blue, 1.0).unwrap(), faint_blue);
    // Halfway, alpha averages and each color counts by its own opacity: red
    // carries five times the weight of the faint blue.
    let [r, g, b, a] = red.mix(faint_blue, 0.5).unwrap().channels();
    assert!((a - 0.6).abs() < 1e-6, "{a}");
    assert!((r - 0.5 / 0.6).abs() < 1e-6 && g == 0.0, "{r}");
    assert!((b - 0.1 / 0.6).abs() < 1e-6, "{b}");
    // Fading toward transparent keeps the hue instead of darkening it.
    assert_eq!(
        red.mix(clear, 0.75).unwrap().channels(),
        [1.0, 0.0, 0.0, 0.25]
    );
    // Zero alpha has no meaningful RGB and is canonical transparent black.
    let invisible = Color::from_linear(0.3, 0.6, 0.9, 0.0).unwrap();
    assert_eq!(invisible.mix(red, 0.0).unwrap(), clear);
    assert_eq!(invisible.mix(invisible, 0.5).unwrap(), clear);
    // A positive alpha that rounds to zero when narrowed is transparent black
    // too, not red at zero alpha.
    let trace = Color::from_linear(1.0, 0.0, 0.0, f32::from_bits(1)).unwrap();
    let rounded = trace.mix(clear, 1.0 - f32::EPSILON / 2.0).unwrap();
    assert_eq!(rounded.channels().map(f32::to_bits), [0; 4]);
    // Endpoints are bitwise exact, including a negative zero channel, which
    // `assert_eq!` on floats alone would not distinguish.
    let signed = Color::from_linear(-0.0, 0.5, 0.25, 0.5).unwrap();
    let bits = |color: Color| color.channels().map(f32::to_bits);
    assert_eq!(bits(signed.mix(red, 0.0).unwrap()), bits(signed));
    assert_eq!(bits(red.mix(signed, 1.0).unwrap()), bits(signed));
    for invalid in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        assert_eq!(red.mix(clear, invalid), Err(DrawError::InvalidColor));
    }
}

fn swatch_ui(swatch: ColorSwatch) -> (Runtime, WidgetId) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("swatch", swatch),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, root)
}
fn inspection(ui: &Runtime) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == "swatch")
        .unwrap()
}
fn fills(ui: &mut Runtime) -> Vec<(Rect, Color)> {
    let scene = ui.paint().unwrap();
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}
fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    Point::new(x + w / 2.0, y + h / 2.0).unwrap()
}

#[test]
fn a_static_swatch_is_an_image_with_no_tab_stop() {
    let color = Color::from_hex("#2090a0").unwrap();
    let (mut ui, root) = swatch_ui(ColorSwatch::new(color));
    let swatch = widget(&ui, &root, "swatch");
    let details = inspection(&ui).details;
    let semantics = details.semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::Image);
    assert_eq!(semantics.name.as_deref(), Some("#2090a0"));
    assert_eq!(semantics.value, Some(SemanticValue::Text("#2090a0".into())));
    assert!(semantics.actions.is_empty());
    assert_eq!(
        details.properties["color"],
        InspectValue::Text("#2090a0".into())
    );
    assert_eq!(details.box_model.background, Some(color.channels()));
    let [_, _, width, height] = ui.bounds(&swatch).unwrap().unwrap().values();
    assert_eq!(width, height, "the default swatch is square");

    // Tab skips it; a click does nothing and is not consumed as an action.
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(widget(&ui, &root, "other")));
    let point = center(&ui, &swatch);
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    settle(&mut ui);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn only_a_translucent_color_draws_the_checkerboard() {
    let opaque = Color::from_hex("#2090a0").unwrap();
    let (mut ui, _) = swatch_ui(ColorSwatch::new(opaque).name("Track"));
    let colors: Vec<Color> = fills(&mut ui).into_iter().map(|(_, c)| c).collect();
    assert!(colors.contains(&opaque));
    let neutral = |c: &Color| c.red() == c.green() && c.green() == c.blue() && c.alpha() == 1.0;
    assert!(
        !colors.iter().any(|c| neutral(c) && *c != Color::WHITE),
        "{colors:?}"
    );
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().name.as_deref(),
        Some("Track")
    );

    let translucent = Color::from_hex("#2090a080").unwrap();
    let (mut ui, root) = swatch_ui(ColorSwatch::new(translucent));
    let swatch = ui.bounds(&widget(&ui, &root, "swatch")).unwrap().unwrap();
    let painted = fills(&mut ui);
    let checker: Vec<&(Rect, Color)> = painted.iter().filter(|(_, c)| neutral(c)).collect();
    let shades: std::collections::BTreeSet<[u32; 4]> = checker
        .iter()
        .map(|(_, c)| c.channels().map(f32::to_bits))
        .collect();
    assert_eq!(shades.len(), 2, "two checker shades");
    assert!(checker.len() > 2, "light cells over a dark base");
    for (rect, _) in &checker {
        let [x, y, w, h] = rect.values();
        let [sx, sy, sw, sh] = swatch.values();
        assert!(x >= sx && y >= sy && x + w <= sx + sw + 1e-3 && y + h <= sy + sh + 1e-3);
    }
    // The color is drawn last over the checkerboard.
    let color_index = painted.iter().position(|(_, c)| *c == translucent).unwrap();
    assert!(
        painted
            .iter()
            .take(color_index)
            .filter(|(_, c)| neutral(c))
            .count()
            == checker.len()
    );
}

#[test]
fn an_activatable_swatch_is_a_button_that_activates_once_and_cancels() {
    let count = Rc::new(Cell::new(0));
    let probe = count.clone();
    let (mut ui, root) = swatch_ui(
        ColorSwatch::new(Color::from_hex("#808080").unwrap())
            .name("Fill")
            .on_activate(move |_| {
                probe.set(probe.get() + 1);
                Ok(())
            }),
    );
    let swatch = widget(&ui, &root, "swatch");
    let semantics = inspection(&ui).details.semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::Button);
    assert_eq!(semantics.name.as_deref(), Some("Fill"));
    assert_eq!(semantics.actions, [SemanticAction::Activate]);

    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(swatch.clone()));
    press(&mut ui, KeyCode::Space);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(count.get(), 2);

    // Escape between press and release cancels.
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::Escape);
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(count.get(), 2);

    // A pointer press released outside cancels; inside activates.
    let inside = center(&ui, &swatch);
    let outside = Point::new(290.0, 190.0).unwrap();
    for (release, expected) in [(outside, 2), (inside, 3)] {
        ui.pointer_input(PointerEvent::new(
            inside,
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        ui.pointer_input(PointerEvent::new(release, PointerKind::Move))
            .unwrap();
        ui.pointer_input(PointerEvent::new(
            release,
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        settle(&mut ui);
        assert_eq!(count.get(), expected);
    }
    // Focus loss while armed cancels.
    ui.focus(&swatch).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.clear_focus();
    settle(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(count.get(), 3);
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn a_disabled_activatable_swatch_neither_focuses_nor_activates() {
    let count = Rc::new(Cell::new(0));
    let probe = count.clone();
    let (mut ui, root) = swatch_ui(
        ColorSwatch::new(Color::WHITE)
            .on_activate(move |_| {
                probe.set(probe.get() + 1);
                Ok(())
            })
            .disabled(),
    );
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(widget(&ui, &root, "other")));
    let swatch = widget(&ui, &root, "swatch");
    let point = center(&ui, &swatch);
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    settle(&mut ui);
    assert_eq!(count.get(), 0);
    let semantics = inspection(&ui).details.semantics.unwrap();
    assert!(semantics.disabled && semantics.actions.is_empty());
}

#[test]
fn a_bound_swatch_repaints_with_its_binding() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let first = Color::from_hex("#ff0000").unwrap();
    let second = Color::from_hex("#00ff0080").unwrap();
    let color = ui.signal(first);
    ui.mount_tree(Element::new(
        "swatch",
        ColorSwatch::bound(color.read_only()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(fills(&mut ui).iter().any(|(_, c)| *c == first));
    color.set(second).unwrap();
    settle(&mut ui);
    let painted = fills(&mut ui);
    assert!(painted.iter().any(|(_, c)| *c == second));
    assert!(!painted.iter().any(|(_, c)| *c == first));
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().value,
        Some(SemanticValue::Text("#00ff0080".into()))
    );
}

#[test]
fn a_swatch_that_gains_an_action_on_reconcile_becomes_focusable() {
    let (mut ui, root) = swatch_ui(ColorSwatch::new(Color::WHITE));
    let row = root.clone();
    ui.reconcile_children(
        &row,
        vec![
            Element::new(
                "swatch",
                ColorSwatch::new(Color::WHITE).on_activate(|_| Ok(())),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(widget(&ui, &root, "swatch")));
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().role,
        SemanticRole::Button
    );
}

#[test]
fn a_disabled_static_swatch_mutes_its_outline_like_a_disabled_action() {
    // The built-in palettes give Muted and Outline the same color, so tell
    // them apart with an override.
    let muted = Color::from_hex("#ff00ff").unwrap();
    let colors = |swatch: ColorSwatch| {
        let (mut ui, _) = swatch_ui(swatch);
        let mode = ui.theme().mode().unwrap();
        ui.theme().set_color(mode, ColorRole::Muted, muted).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { color, .. }
                | DrawCommand::DrawVector { color, .. }
                | DrawCommand::DrawMask { color, .. } => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let color = Color::from_hex("#2090a0").unwrap();
    let enabled = colors(ColorSwatch::new(color));
    let disabled = colors(ColorSwatch::new(color).disabled());
    let disabled_action = colors(ColorSwatch::new(color).on_activate(|_| Ok(())).disabled());
    assert!(!enabled.contains(&muted));
    assert!(
        disabled.contains(&muted),
        "the outline changes when disabled"
    );
    assert_eq!(disabled, disabled_action, "both use the muted outline");
}

#[test]
fn role_named_swatch_preserves_resolved_name_across_identical_description() {
    let (mut ui, root) = swatch_ui(ColorSwatch::new(Color::WHITE).name(StringRole::ColorName));
    let table = UiStrings::english()
        .with_text(StringRole::ColorName, "Track tint")
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    settle(&mut ui);
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().name.as_deref(),
        Some("Track tint")
    );
    ui.reconcile_children(
        &root,
        vec![
            Element::new(
                "swatch",
                ColorSwatch::new(Color::WHITE).name(StringRole::ColorName),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().name.as_deref(),
        Some("Track tint")
    );
    let table = UiStrings::english()
        .with_text(StringRole::ColorName, "Updated tint")
        .unwrap();
    ui.ui_settings().set_strings(table).unwrap();
    settle(&mut ui);
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().name.as_deref(),
        Some("Updated tint")
    );
    ui.reconcile_children(
        &root,
        vec![
            Element::new(
                "swatch",
                ColorSwatch::new(Color::WHITE).name(StringRole::ColorPickerName),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(
        inspection(&ui).details.semantics.unwrap().name.as_deref(),
        Some(UiStrings::english().text(StringRole::ColorPickerName))
    );
}

#[test]
fn the_checkerboard_stays_bounded_for_any_valid_icon_size() {
    let translucent = Color::from_hex("#2090a080").unwrap();
    for icon in [0.0, f32::MIN_POSITIVE, 1.0e-3] {
        let (mut ui, _) = swatch_ui(ColorSwatch::new(translucent));
        let density = ui.theme().density().unwrap();
        let metrics = ThemeMetrics::for_density(density)
            .with_metric(MetricRole::IconSize, Length::new(icon).unwrap())
            .unwrap();
        ui.theme().set_metrics(density, metrics).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let neutral = |c: &Color| c.red() == c.green() && c.green() == c.blue() && c.alpha() == 1.0;
        let cells = fills(&mut ui).iter().filter(|(_, c)| neutral(c)).count();
        assert!(cells > 2, "{icon}: the checkerboard is still drawn");
        assert!(cells <= 1 + 64 * 64 / 2 + 64, "{icon}: {cells} cells");
    }
}
