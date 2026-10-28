//! Independent acceptance checks added by the round 3 verifier (not the
//! tester). They drive the application through pert-ui's public input,
//! inspection and CPU capture facilities and read the picker's value without
//! naming its type: an inspection query by semantic role and name.
//!
//! No production source is changed. These tests use only the crate's public
//! `build` (the same content `app()` mounts) and `PointPicker` constructors.

use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use pert_ui::component::Component;
use pert_ui::cpu::CpuRenderer;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{CaptureRequest, CaptureTarget, FrameImage};
use point_picker::{PickerApp, PointPicker, build};

type R<T = ()> = Result<T, Box<dyn Error>>;

const PRESS: PointerKind = PointerKind::Press(PointerButton::Primary);
const RELEASE: PointerKind = PointerKind::Release(PointerButton::Primary);
const W: f32 = 480.0;
const H: f32 = 320.0;

fn start(mode: ThemeMode) -> R<(Runtime, PickerApp)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(W, H, 1.0)?)?;
    ui.theme().set_mode(mode)?;
    let app = build(&mut ui)?;
    Ok((ui, app))
}

/// The value of the component with semantic role Slider named `name`, found
/// by a library inspection query (no application type or id involved).
fn slider_value(ui: &Runtime, name: &str) -> R<(f64, f64)> {
    let query = InspectionQuery::components(Selector::semantic(SemanticRole::Slider, Some(name)))
        .detail(Detail::Full);
    let result = ui.query(&query);
    assert_eq!(result.components.len(), 1, "exactly one slider named {name}");
    let c = &result.components[0];
    let num = |k: &str| match c.details.properties.get(k) {
        Some(InspectValue::Number(v)) => Ok(*v),
        other => Err(format!("property {k}: {other:?}")),
    };
    Ok((num("x")?, num("y")?))
}

fn point(ui: &Runtime) -> R<(f64, f64)> {
    slider_value(ui, "Point")
}

fn press(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> R {
    ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    Ok(())
}

fn bounds(ui: &Runtime, id: &WidgetId) -> R<[f32; 4]> {
    Ok(ui.bounds(id)?.ok_or("laid out")?.values())
}

fn field_number(driver: &Driver<'_>, id: &WidgetId) -> R<f64> {
    match driver.semantics(id)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("field value {other:?}").into()),
    }
}

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

// ---------------------------------------------------------------- item 1

#[test]
fn acc1_drag_outside_each_edge_clamps_and_continues_until_release() -> R {
    let (mut ui, app) = start(ThemeMode::Dark)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let [l, t, w, h] = bounds(d.runtime(), &app.picker)?;
    assert_eq!(w, h);

    // Press inside: the handle jumps under the pointer.
    press(d.runtime_mut(), l + w * 0.4, t + h * 0.6, PRESS)?;
    d.settle()?;
    assert!(close(point(d.runtime())?, (0.4, 0.4)), "{:?}", point(d.runtime())?);

    // Beyond the top-right corner: clamped to (1, 1).
    press(d.runtime_mut(), l + w + 300.0, t - 300.0, PointerKind::Move)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (1.0, 1.0));
    // Beyond the bottom-left corner, outside the whole window: (0, 0).
    press(d.runtime_mut(), -40.0, H + 40.0, PointerKind::Move)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (0.0, 0.0));
    // Outside on one axis only: that axis clamps, the other follows.
    press(d.runtime_mut(), l + w * 0.75, t + h + 50.0, PointerKind::Move)?;
    d.settle()?;
    assert!(close(point(d.runtime())?, (0.75, 0.0)), "{:?}", point(d.runtime())?);

    // Release outside the area ends the drag at the clamped value.
    press(d.runtime_mut(), l + w + 20.0, t + h * 0.5, PointerKind::Move)?;
    press(d.runtime_mut(), l + w + 20.0, t + h * 0.5, RELEASE)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (1.0, 0.5));
    // A later move with no button changes nothing.
    press(d.runtime_mut(), l + 5.0, t + 5.0, PointerKind::Move)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (1.0, 0.5));
    // The owner's signals and the numeric fields followed the drag.
    assert_eq!((app.x.get()?, app.y.get()?), (1.0, 0.5));
    let xf = d.find(&app.root, &["layout", "fields", "x-field", "x"])?;
    let yf = d.find(&app.root, &["layout", "fields", "y-field", "y"])?;
    assert_eq!((field_number(&d, &xf)?, field_number(&d, &yf)?), (1.0, 0.5));
    Ok(())
}

// ---------------------------------------------------------------- item 2

#[test]
fn acc2_escape_cancels_drag_to_pre_press_value_and_next_drag_works() -> R {
    let (mut ui, app) = start(ThemeMode::Light)?;
    app.x.set(0.33)?;
    app.y.set(0.66)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let [l, t, w, h] = bounds(d.runtime(), &app.picker)?;

    // Press elsewhere (the press itself moves the point), drag outside.
    press(d.runtime_mut(), l + w * 0.9, t + h * 0.9, PRESS)?;
    press(d.runtime_mut(), l - 100.0, t - 100.0, PointerKind::Move)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (0.0, 1.0));
    assert_eq!(
        d.runtime().focused(),
        Some(app.picker.clone()),
        "press focused the picker so Escape reaches it"
    );

    d.key(KeyCode::Escape)?;
    assert_eq!(point(d.runtime())?, (0.33, 0.66), "pre-press value restored");
    assert_eq!((app.x.get()?, app.y.get()?), (0.33, 0.66));
    let xf = d.find(&app.root, &["layout", "fields", "x-field", "x"])?;
    assert_eq!(field_number(&d, &xf)?, 0.33, "field shows the restored value");

    // Moves and the release after the cancel do nothing.
    press(d.runtime_mut(), l + w * 0.5, t + h * 0.5, PointerKind::Move)?;
    press(d.runtime_mut(), l + w * 0.5, t + h * 0.5, RELEASE)?;
    d.settle()?;
    assert_eq!(point(d.runtime())?, (0.33, 0.66));

    // A fresh drag after the cancel works and commits on release.
    press(d.runtime_mut(), l + w * 0.2, t + h * 0.2, PRESS)?;
    press(d.runtime_mut(), l + w * 0.2, t + h * 0.2, RELEASE)?;
    d.settle()?;
    assert!(close(point(d.runtime())?, (0.2, 0.8)), "{:?}", point(d.runtime())?);
    // Escape with no drag open leaves a committed value alone.
    d.key(KeyCode::Escape)?;
    assert!(close(point(d.runtime())?, (0.2, 0.8)));
    Ok(())
}

// ---------------------------------------------------------------- item 3

#[test]
fn acc3_arrow_keys_step_001_and_shift_step_01_on_both_axes() -> R {
    let (mut ui, app) = start(ThemeMode::Dark)?;
    app.x.set(0.2)?;
    app.y.set(0.8)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    // Reach the picker through the library's keyboard focus traversal.
    let mut tabs = 0;
    while d.runtime().focused() != Some(app.picker.clone()) {
        d.key(KeyCode::Tab)?;
        tabs += 1;
        assert!(tabs < 10, "Tab never focuses the picker");
    }

    let cases: [(KeyCode, bool, (f64, f64)); 8] = [
        (KeyCode::ArrowRight, false, (0.21, 0.8)),
        (KeyCode::ArrowUp, false, (0.21, 0.81)),
        (KeyCode::ArrowLeft, false, (0.2, 0.81)),
        (KeyCode::ArrowDown, false, (0.2, 0.8)),
        (KeyCode::ArrowRight, true, (0.3, 0.8)),
        (KeyCode::ArrowUp, true, (0.3, 0.9)),
        (KeyCode::ArrowLeft, true, (0.2, 0.9)),
        (KeyCode::ArrowDown, true, (0.2, 0.8)),
    ];
    for (code, shift, want) in cases {
        if shift {
            d.chord(KeyChord::named(code).shifted(), code)?;
        } else {
            d.key(code)?;
        }
        let got = point(d.runtime())?;
        assert!(close(got, want), "{code:?} shift={shift}: {got:?} != {want:?}");
    }
    // Shift steps clamp at 0 and 1.
    for _ in 0..5 {
        d.chord(KeyChord::named(KeyCode::ArrowLeft).shifted(), KeyCode::ArrowLeft)?;
        d.chord(KeyChord::named(KeyCode::ArrowUp).shifted(), KeyCode::ArrowUp)?;
    }
    assert_eq!(point(d.runtime())?, (0.0, 1.0));

    // Arrows reach the picker only when it is focused.
    let xf = d.find(&app.root, &["layout", "fields", "x-field", "x"])?;
    d.runtime_mut().focus(&xf)?;
    d.settle()?;
    d.key(KeyCode::ArrowRight)?;
    assert_eq!(point(d.runtime())?.1, 1.0);
    Ok(())
}

// ---------------------------------------------------------------- item 4

#[test]
fn acc4_typed_fields_move_handle_both_axes_and_reset_restores_centre() -> R {
    let (mut ui, app) = start(ThemeMode::Dark)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let xf = d.find(&app.root, &["layout", "fields", "x-field", "x"])?;
    let yf = d.find(&app.root, &["layout", "fields", "y-field", "y"])?;

    // Field -> picker, typed through the library's text input.
    d.replace_text(&xf, "0.12")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&yf, "0.87")?;
    // Commit y by moving focus away (Tab), not Enter.
    d.key(KeyCode::Tab)?;
    assert!(close(point(d.runtime())?, (0.12, 0.87)), "{:?}", point(d.runtime())?);

    // The handle is painted at the new point: the CPU capture's pixel at
    // the point is the theme's Primary color.
    let [l, t, w, h] = bounds(d.runtime(), &app.picker)?;
    let img = capture(d.runtime_mut())?;
    let primary = d.runtime().theme().palette(ThemeMode::Dark)?.color(ColorRole::Primary);
    let (hx, hy) = (l + w * 0.12, t + h * (1.0 - 0.87));
    assert!(
        near(pixel(&img, hx as u32 + 2, hy as u32 + 2), primary.to_srgb8(), 3),
        "handle at the typed point: {:?} vs {:?}",
        pixel(&img, hx as u32 + 2, hy as u32 + 2),
        primary.to_srgb8()
    );

    // Picker -> fields, both axes.
    d.runtime_mut().focus(&app.picker)?;
    d.settle()?;
    d.key(KeyCode::ArrowRight)?;
    d.key(KeyCode::ArrowDown)?;
    assert!((field_number(&d, &xf)? - 0.13).abs() < 1e-9);
    assert!((field_number(&d, &yf)? - 0.86).abs() < 1e-9);

    // An out-of-range entry is refused by the field and does not move the
    // picker past 1.
    d.replace_text(&xf, "7")?;
    d.key(KeyCode::Enter)?;
    let p = point(d.runtime())?;
    assert!((0.0..=1.0).contains(&p.0), "{p:?}");

    // Reset through a simulated click.
    let reset = d.find(&app.root, &["layout", "fields", "reset"])?;
    d.click(&reset)?;
    assert_eq!(point(d.runtime())?, (0.5, 0.5));
    assert_eq!((field_number(&d, &xf)?, field_number(&d, &yf)?), (0.5, 0.5));
    assert_eq!((app.x.get()?, app.y.get()?), (0.5, 0.5));
    Ok(())
}

// ---------------------------------------------------------------- item 5

/// A window capture served by the CPU renderer, flushed at the runtime's
/// current time so it also works after a `Driver` has advanced the clock.
fn capture(ui: &mut Runtime) -> R<FrameImage> {
    let now = ui.now();
    ui.flush(now)?;
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
        &ui.primary_window(),
    )));
    ui.flush(now)?;
    CpuRenderer::new().serve_captures(ui);
    Ok(pending.try_reply()?.value?.image)
}

fn pixel(img: &FrameImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width() + x) * 4) as usize;
    let p = img.pixels();
    [p[i], p[i + 1], p[i + 2], p[i + 3]]
}

fn near(a: [u8; 4], b: [u8; 4], tol: i16) -> bool {
    a.iter().zip(b).all(|(x, y)| (*x as i16 - y as i16).abs() <= tol)
}

/// Whether any pixel within `band` px inside the picker's left edge, on the
/// picker's middle row, has `color`.
fn left_edge_has(img: &FrameImage, b: [f32; 4], color: [u8; 4], band: u32) -> bool {
    let y = (b[1] + b[3] * 0.25) as u32;
    let x0 = b[0].floor() as u32;
    (x0.saturating_sub(1)..x0 + band).any(|x| near(pixel(img, x, y), color, 3))
}

#[test]
fn acc5_cpu_captures_dark_and_light_show_theme_focus_ring() -> R {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance-captures");
    std::fs::create_dir_all(&out)?;
    let mut fills = Vec::new();
    for (name, mode) in [("dark", ThemeMode::Dark), ("light", ThemeMode::Light)] {
        // The application's own headless capture writes the PNG under test.
        let app_png = point_picker::headless::capture_png(mode)?;
        assert!(app_png.starts_with(b"\x89PNG\r\n\x1a\n"));
        std::fs::write(out.join(format!("{name}.png")), &app_png)?;

        // The same scene through the same public capture path, kept raw so
        // its pixels can be read; its PNG must equal the application's.
        let (mut ui, app) = start(mode)?;
        app.x.set(0.3)?;
        app.y.set(0.7)?;
        ui.flush(Duration::ZERO)?;
        let b = bounds(&ui, &app.picker)?;
        let unfocused = capture(&mut ui)?;
        ui.focus(&app.picker)?;
        let focused = capture(&mut ui)?;
        assert_eq!(focused.encode_png(), app_png, "{name}: same image as the app's PNG");
        std::fs::write(out.join(format!("{name}-unfocused.png")), unfocused.encode_png())?;

        let palette = ui.theme().palette(mode)?;
        let ring = palette.color(ColorRole::FocusRing).to_srgb8();
        let fill = palette.color(ColorRole::SurfaceContainer).to_srgb8();
        assert!(left_edge_has(&focused, b, ring, 4), "{name}: focus ring in theme color");
        assert!(!left_edge_has(&unfocused, b, ring, 4), "{name}: no ring when unfocused");
        // The area's fill is the theme's SurfaceContainer.
        let inside = pixel(&focused, (b[0] + b[2] * 0.75) as u32, (b[1] + b[3] * 0.75) as u32);
        assert!(near(inside, fill, 3), "{name}: area fill {inside:?} vs {fill:?}");
        fills.push((ring, fill, pixel(&focused, 2, H as u32 - 2)));
    }
    // The two captures differ in colors: ring, area fill and background.
    assert_ne!(fills[0].0, fills[1].0, "ring color differs between themes");
    assert_ne!(fills[0].1, fills[1].1, "area fill differs between themes");
    assert_ne!(fills[0].2, fills[1].2, "window background differs between themes");
    Ok(())
}

#[test]
fn acc5b_focus_ring_follows_a_live_theme_change() -> R {
    let (mut ui, app) = start(ThemeMode::Dark)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&app.picker)?;
    let b = bounds(&ui, &app.picker)?;
    let dark = capture(&mut ui)?;
    ui.theme().set_mode(ThemeMode::Light)?;
    let light = capture(&mut ui)?;
    let dring = ui.theme().palette(ThemeMode::Dark)?.color(ColorRole::FocusRing).to_srgb8();
    let lring = ui.theme().palette(ThemeMode::Light)?.color(ColorRole::FocusRing).to_srgb8();
    assert!(left_edge_has(&dark, b, dring, 4));
    assert!(left_edge_has(&light, b, lring, 4));
    assert!(!left_edge_has(&light, b, dring, 4), "old ring color gone after switch");
    Ok(())
}

// ---------------------------------------------------------------- item 6

#[test]
fn acc6_value_in_inspection_snapshot_follows_input() -> R {
    let (mut ui, app) = start(ThemeMode::Dark)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    d.runtime_mut().focus(&app.picker)?;
    d.chord(KeyChord::named(KeyCode::ArrowUp).shifted(), KeyCode::ArrowUp)?;
    d.key(KeyCode::ArrowLeft)?;

    let snapshot = d.runtime().inspect();
    let found: Vec<_> = snapshot
        .components
        .iter()
        .filter(|c| {
            c.details
                .semantics
                .as_ref()
                .is_some_and(|s| s.role == SemanticRole::Slider && s.name.as_deref() == Some("Point"))
        })
        .collect();
    assert_eq!(found.len(), 1);
    let c = found[0];
    assert_eq!(c.key, "picker");
    let s = c.details.semantics.as_ref().ok_or("semantics")?;
    assert_eq!(s.value, Some(SemanticValue::Text("x 0.49, y 0.6".into())));
    assert_eq!(c.details.properties.get("x"), Some(&InspectValue::Number(0.49)));
    assert_eq!(c.details.properties.get("y"), Some(&InspectValue::Number(0.6)));
    Ok(())
}

// ------------------------------------------------- reusability / contract

fn is_component<C: Component + 'static>(c: C) -> C {
    c
}

#[test]
fn acc_reusable_two_independent_pickers_report_and_accept_owner_values() -> R {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0)?)?;
    let (ax, ay, bx, by) = (ui.signal(0.1), ui.signal(0.2), ui.signal(0.7), ui.signal(0.8));
    let reports = ui.signal(Vec::<(f64, f64)>::new());
    let log = reports.clone();
    let a = is_component(
        PointPicker::new(&ax, &ay)
            .name("A")
            .on_change(move |_, p| log.update(|v| v.push(p))),
    );
    let b = is_component(PointPicker::new(&bx, &by).name("B"));
    let root = ui.mount_tree(Row::new([Element::new("a", a), Element::new("b", b)]).key("row"))?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let a_id = d.find(&root, &["a"])?;
    assert_eq!(slider_value(d.runtime(), "A")?, (0.1, 0.2));
    assert_eq!(slider_value(d.runtime(), "B")?, (0.7, 0.8));

    d.runtime_mut().focus(&a_id)?;
    d.key(KeyCode::ArrowRight)?;
    assert!(close(slider_value(d.runtime(), "A")?, (0.11, 0.2)));
    assert_eq!(slider_value(d.runtime(), "B")?, (0.7, 0.8), "B unaffected");
    assert_eq!(reports.get()?.len(), 1, "user change reported to owner");

    // The owner sets the value; it shows, without a user-change report.
    bx.set(0.25)?;
    ax.set(0.9)?;
    d.settle()?;
    assert_eq!(slider_value(d.runtime(), "B")?, (0.25, 0.8));
    assert_eq!(slider_value(d.runtime(), "A")?.0, 0.9);
    assert_eq!(reports.get()?.len(), 1);
    Ok(())
}
