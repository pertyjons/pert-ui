//! Independent acceptance checks added by the verifier (acceptance copy only).
//! They drive the application through the library's runtime pointer input,
//! `Driver` key/text/click input, inspection snapshots and CPU captures.
//! No production code is changed or called directly except mounting the
//! application (`point_picker::app()`) and, for reusability, the public
//! `PointPicker` type inside a separate library-mounted tree.
use pert_ui::{prelude::*, testing::*};
use std::error::Error;

type R<T = ()> = Result<T, Box<dyn Error>>;

const OUT: &str = "acceptance-captures";

/// The picker's point read only through an inspection snapshot serialized to
/// JSON, as a tool that does not know the application's types would.
fn json_point(ui: &Runtime, key: &str) -> R<[f64; 2]> {
    let json = serde_json::to_value(ui.inspect())?;
    let comps = json["components"].as_array().ok_or("no components")?;
    let c = comps
        .iter()
        .find(|c| c["key"] == key)
        .ok_or("picker not in snapshot")?;
    let props = &c["details"]["properties"];
    let num = |name: &str| -> R<f64> {
        let v = &props[name];
        // InspectValue::Number may serialize tagged or untagged; accept both.
        v.as_f64()
            .or_else(|| v["Number"].as_f64())
            .ok_or_else(|| format!("property {name} not numeric: {v}").into())
    };
    Ok([num("x")?, num("y")?])
}

fn close(actual: [f64; 2], expected: [f64; 2]) {
    for i in 0..2 {
        assert!(
            (actual[i] - expected[i]).abs() < 1e-6,
            "{actual:?} != {expected:?}"
        );
    }
}

fn field_number(d: &Driver<'_>, id: &WidgetId) -> R<f64> {
    match d.semantics(id)?.value {
        Some(SemanticValue::Number { value, .. }) => Ok(value),
        other => Err(format!("field value: {other:?}").into()),
    }
}

fn pointer(d: &mut Driver<'_>, x: f32, y: f32, kind: PointerKind) -> R {
    d.runtime_mut()
        .pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    d.settle()?;
    Ok(())
}

struct Ids {
    picker: WidgetId,
    xf: WidgetId,
    yf: WidgetId,
    reset: WidgetId,
    rect: [f32; 4],
}

fn ids(d: &Driver<'_>, root: &WidgetId) -> R<Ids> {
    let picker = d.find(root, &["body", "picker"])?;
    let rect = d
        .runtime()
        .bounds(&picker)?
        .ok_or("picker layout")?
        .values();
    Ok(Ids {
        xf: d.find(root, &["body", "fields", "x"])?,
        yf: d.find(root, &["body", "fields", "y"])?,
        reset: d.find(root, &["body", "fields", "reset"])?,
        picker,
        rect,
    })
}

fn capture(d: &mut Driver<'_>) -> R<FrameImage> {
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    Ok(pending.try_reply()?.value?.image)
}

fn px(img: &FrameImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width() + x) * 4) as usize;
    let p = img.pixels();
    [p[i], p[i + 1], p[i + 2], p[i + 3]]
}

/// Pixels equal to `rgb` within the picker rectangle grown by `pad`.
fn count_in(img: &FrameImage, rect: [f32; 4], pad: f32, rgb: [u8; 3]) -> usize {
    let [x, y, w, h] = rect;
    let x0 = (x - pad).max(0.0) as u32;
    let y0 = (y - pad).max(0.0) as u32;
    let x1 = ((x + w + pad) as u32).min(img.width());
    let y1 = ((y + h + pad) as u32).min(img.height());
    let mut n = 0;
    for yy in y0..y1 {
        for xx in x0..x1 {
            let p = px(img, xx, yy);
            if p[0] == rgb[0] && p[1] == rgb[1] && p[2] == rgb[2] {
                n += 1;
            }
        }
    }
    n
}

fn whole(img: &FrameImage, rgb: [u8; 3]) -> usize {
    count_in(
        img,
        [0.0, 0.0, img.width() as f32, img.height() as f32],
        0.0,
        rgb,
    )
}

fn save(img: &FrameImage, name: &str) -> R {
    std::fs::create_dir_all(OUT)?;
    std::fs::write(format!("{OUT}/{name}.png"), img.encode_png())?;
    Ok(())
}

/// Items 1 and 2 plus the press rule: pressing inside moves the handle there;
/// the drag continues outside on every side (clamped) until release; Escape
/// restores the pre-press point; the fields follow the drag.
#[test]
fn v1_v2_drag_clamp_and_cancel() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    let [x, y, w, h] = i.rect;
    close(json_point(d.runtime(), "picker")?, [0.5, 0.5]);

    // Press inside moves the handle to the press point immediately.
    pointer(&mut d, x + w * 0.3, y + h * 0.7, PointerKind::Press(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [0.3, 0.7]);
    assert_eq!(d.runtime().focused(), Some(i.picker.clone()), "press focuses");
    // Drag outside left/top, then right/bottom: both clamp.
    pointer(&mut d, x - 100.0, y - 100.0, PointerKind::Move)?;
    close(json_point(d.runtime(), "picker")?, [0.0, 0.0]);
    pointer(&mut d, x + w + 300.0, y + h + 300.0, PointerKind::Move)?;
    close(json_point(d.runtime(), "picker")?, [1.0, 1.0]);
    // Back inside, still dragging.
    pointer(&mut d, x + w * 0.6, y + h * 0.4, PointerKind::Move)?;
    close(json_point(d.runtime(), "picker")?, [0.6, 0.4]);
    // Release outside: clamped value is kept.
    pointer(&mut d, x + w + 50.0, y + h * 0.25, PointerKind::Move)?;
    pointer(&mut d, x + w + 50.0, y + h * 0.25, PointerKind::Release(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.25]);
    assert!((field_number(&d, &i.xf)? - 1.0).abs() < 1e-6);
    assert!((field_number(&d, &i.yf)? - 0.25).abs() < 1e-6);
    // After release, moves no longer drag.
    pointer(&mut d, x + w * 0.1, y + h * 0.1, PointerKind::Move)?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.25]);

    // Cancel: press elsewhere, drag outside, Escape restores (1.0, 0.25).
    pointer(&mut d, x + w * 0.2, y + h * 0.2, PointerKind::Press(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [0.2, 0.2]);
    pointer(&mut d, x - 30.0, y + h + 30.0, PointerKind::Move)?;
    close(json_point(d.runtime(), "picker")?, [0.0, 1.0]);
    d.key(KeyCode::Escape)?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.25]);
    assert!((field_number(&d, &i.xf)? - 1.0).abs() < 1e-6, "field restored");
    assert!((field_number(&d, &i.yf)? - 0.25).abs() < 1e-6, "field restored");
    // Further moves and the release do not change the restored value.
    pointer(&mut d, x + w * 0.5, y + h * 0.5, PointerKind::Move)?;
    pointer(&mut d, x + w * 0.5, y + h * 0.5, PointerKind::Release(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.25]);

    // A press outside the area does not move the point.
    pointer(&mut d, x + w + 400.0, y + h + 50.0, PointerKind::Press(PointerButton::Primary))?;
    pointer(&mut d, x + w + 400.0, y + h + 50.0, PointerKind::Release(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.25]);
    Ok(())
}

/// Item 3: every arrow with and without Shift, clamping at the edges, and
/// arrows only act while the picker is focused. Fields follow.
#[test]
fn v3_arrow_keys_with_and_without_shift() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    d.runtime_mut().focus(&i.picker)?;
    d.settle()?;
    let shift = |d: &mut Driver<'_>, code| d.chord(KeyChord::named(code).shifted(), code);

    d.key(KeyCode::ArrowRight)?;
    close(json_point(d.runtime(), "picker")?, [0.51, 0.5]);
    shift(&mut d, KeyCode::ArrowRight)?;
    close(json_point(d.runtime(), "picker")?, [0.61, 0.5]);
    d.key(KeyCode::ArrowLeft)?;
    close(json_point(d.runtime(), "picker")?, [0.60, 0.5]);
    shift(&mut d, KeyCode::ArrowLeft)?;
    close(json_point(d.runtime(), "picker")?, [0.50, 0.5]);
    d.key(KeyCode::ArrowDown)?;
    close(json_point(d.runtime(), "picker")?, [0.50, 0.51]);
    shift(&mut d, KeyCode::ArrowDown)?;
    close(json_point(d.runtime(), "picker")?, [0.50, 0.61]);
    d.key(KeyCode::ArrowUp)?;
    close(json_point(d.runtime(), "picker")?, [0.50, 0.60]);
    shift(&mut d, KeyCode::ArrowUp)?;
    close(json_point(d.runtime(), "picker")?, [0.50, 0.50]);
    assert!((field_number(&d, &i.xf)? - 0.5).abs() < 1e-6);
    assert!((field_number(&d, &i.yf)? - 0.5).abs() < 1e-6);

    // Clamping at the edges.
    for _ in 0..6 {
        shift(&mut d, KeyCode::ArrowRight)?;
        shift(&mut d, KeyCode::ArrowUp)?;
    }
    close(json_point(d.runtime(), "picker")?, [1.0, 0.0]);
    d.key(KeyCode::ArrowRight)?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.0]);
    assert!((field_number(&d, &i.xf)? - 1.0).abs() < 1e-6);
    assert!((field_number(&d, &i.yf)? - 0.0).abs() < 1e-6);

    // Without focus, arrows do not move the picker.
    d.clear_focus()?;
    d.key(KeyCode::ArrowLeft)?;
    shift(&mut d, KeyCode::ArrowDown)?;
    close(json_point(d.runtime(), "picker")?, [1.0, 0.0]);
    Ok(())
}

/// Item 4 and the two-way sync check: editing each field moves the picker
/// (inspection and the painted handle), picker input updates the fields,
/// owner writes reach both, and Reset restores (0.5, 0.5) everywhere.
#[test]
fn v4_fields_sync_and_reset() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    // Distinctive theme Primary so the handle can be located in captures.
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Primary, Color::from_linear(0.0, 1.0, 0.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    let [x, y, w, h] = i.rect;
    let green = [0u8, 255, 0];
    let handle_at = |img: &FrameImage, fx: f32, fy: f32| {
        // The 12x12 handle centered on the point: sample a 3x3 block.
        let (cx, cy) = ((x + w * fx) as u32, (y + h * fy) as u32);
        let mut all = true;
        for dy in 0..3 {
            for dx in 0..3 {
                let p = px(img, cx - 1 + dx, cy - 1 + dy);
                all &= p[0] == 0 && p[1] == 255 && p[2] == 0;
            }
        }
        all
    };

    d.replace_text(&i.xf, "0.25")?;
    d.key(KeyCode::Enter)?;
    close(json_point(d.runtime(), "picker")?, [0.25, 0.5]);
    d.replace_text(&i.yf, "0.8")?;
    d.key(KeyCode::Enter)?;
    close(json_point(d.runtime(), "picker")?, [0.25, 0.8]);
    let img = capture(&mut d)?;
    save(&img, "v4-after-field-edit")?;
    assert!(whole(&img, green) > 0, "theme Primary used for the handle");
    assert!(handle_at(&img, 0.25, 0.8), "handle painted at the edited point");
    assert!(!handle_at(&img, 0.5, 0.5));

    // Field commit by focus loss (Tab) rather than Enter.
    d.replace_text(&i.xf, "0.4")?;
    d.key(KeyCode::Tab)?;
    close(json_point(d.runtime(), "picker")?, [0.4, 0.8]);

    // Picker -> fields through keyboard.
    d.runtime_mut().focus(&i.picker)?;
    d.settle()?;
    d.key(KeyCode::ArrowRight)?;
    assert!((field_number(&d, &i.xf)? - 0.41).abs() < 1e-6);

    // Reset through a simulated click.
    d.click(&i.reset)?;
    close(json_point(d.runtime(), "picker")?, [0.5, 0.5]);
    assert!((field_number(&d, &i.xf)? - 0.5).abs() < 1e-6);
    assert!((field_number(&d, &i.yf)? - 0.5).abs() < 1e-6);
    let img = capture(&mut d)?;
    save(&img, "v4-after-reset")?;
    assert!(handle_at(&img, 0.5, 0.5), "handle painted at center after Reset");
    assert!(!handle_at(&img, 0.25, 0.8));

    // Owner sets the value; picker and fields follow, without a change report.
    let before = state.changes.get()?;
    state.x.set(0.1)?;
    state.y.set(0.9)?;
    d.settle()?;
    close(json_point(d.runtime(), "picker")?, [0.1, 0.9]);
    assert!((field_number(&d, &i.xf)? - 0.1).abs() < 1e-6);
    assert_eq!(state.changes.get()?, before, "owner writes not reported as changes");
    // User changes are reported to the owner.
    d.runtime_mut().focus(&i.picker)?;
    d.settle()?;
    d.key(KeyCode::ArrowUp)?;
    assert!(state.changes.get()? > before, "user change reported");
    Ok(())
}

/// Item 5 and the theme/focus checks: CPU captures in dark and light with the
/// picker focused; the two differ; the focus ring uses the theme's FocusRing
/// role (overridden to a distinctive color) only while focused, and the
/// picker's surface uses the theme's Surface role.
#[test]
fn v5_captures_theme_and_focus_ring() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    let [x, y, w, h] = i.rect;

    // Default palettes, focused, both modes.
    d.runtime_mut().focus(&i.picker)?;
    let mut shots = Vec::new();
    for (mode, name) in [(ThemeMode::Dark, "v5-dark-focused"), (ThemeMode::Light, "v5-light-focused")] {
        d.runtime().theme().set_mode(mode)?;
        d.settle()?;
        assert_eq!(d.runtime().focused(), Some(i.picker.clone()));
        let img = capture(&mut d)?;
        assert!(img.encode_png().starts_with(b"\x89PNG"));
        save(&img, name)?;
        shots.push(img);
    }
    let inner = |img: &FrameImage| px(img, (x + w * 0.25) as u32, (y + h * 0.25) as u32);
    assert_ne!(shots[0].pixels(), shots[1].pixels(), "dark and light differ");
    assert_ne!(inner(&shots[0]), inner(&shots[1]), "picker surface differs by theme");

    // Distinctive overrides in both modes: FocusRing red, Surface blue.
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let t = d.runtime().theme();
        t.set_color(mode, ColorRole::FocusRing, Color::from_linear(1.0, 0.0, 0.0, 1.0)?)?;
        t.set_color(mode, ColorRole::Surface, Color::from_linear(0.0, 0.0, 1.0, 1.0)?)?;
    }
    for (mode, tag) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        d.runtime().theme().set_mode(mode)?;
        d.runtime_mut().focus(&i.picker)?;
        d.settle()?;
        let focused = capture(&mut d)?;
        save(&focused, &format!("v5-{tag}-focused-overridden"))?;
        let ring_focused = count_in(&focused, i.rect, 4.0, [255, 0, 0]);
        assert!(ring_focused >= (2.0 * (w + h)) as usize, "focus ring drawn: {ring_focused}");
        assert_eq!(inner(&focused)[..3], [0, 0, 255], "picker surface is theme Surface");

        d.clear_focus()?;
        d.settle()?;
        assert_eq!(d.runtime().focused(), None);
        let unfocused = capture(&mut d)?;
        save(&unfocused, &format!("v5-{tag}-unfocused-overridden"))?;
        assert_eq!(whole(&unfocused, [255, 0, 0]), 0, "no focus ring when unfocused");
    }
    Ok(())
}

/// Item 6: the value appears in an inspection snapshot (properties and the
/// semantic value), readable by key without the application's types.
#[test]
fn v6_value_in_inspection_snapshot() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    d.runtime_mut().focus(&i.picker)?;
    d.settle()?;
    d.chord(KeyChord::named(KeyCode::ArrowRight).shifted(), KeyCode::ArrowRight)?;
    d.key(KeyCode::ArrowDown)?;
    close(json_point(d.runtime(), "picker")?, [0.6, 0.51]);
    let snap = d.runtime().inspect();
    std::fs::create_dir_all(OUT)?;
    std::fs::write(format!("{OUT}/v6-inspection.json"), serde_json::to_string_pretty(&snap)?)?;
    let sem = d.semantics(&i.picker)?;
    let text = format!("{:?}", sem.value);
    assert!(text.contains("0.6") && text.contains("0.51"), "semantic value: {text}");
    Ok(())
}

/// The control is a reusable type: two independent instances mounted by the
/// library in another tree, each driven by input, each with its own value.
#[test]
fn v7_reusable_control_two_instances() -> R {
    use point_picker::picker::PointPicker;
    struct S {
        root: WidgetId,
        a: [Signal<f64>; 2],
        b: [Signal<f64>; 2],
    }
    let app = App::new("two pickers", |ui: &mut Runtime| {
        let a = [ui.signal(0.5), ui.signal(0.5)];
        let b = [ui.signal(0.2), ui.signal(0.2)];
        let root = ui.mount_tree(
            Row::new([
                PointPicker::new(a[0].clone(), a[1].clone()).key("a"),
                PointPicker::new(b[0].clone(), b[1].clone()).key("b"),
            ])
            .key("root"),
        )?;
        Ok(S { root, a, b })
    });
    let mut app = app.headless()?;
    let (ui, s) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let a = d.find(&s.root, &["a"])?;
    let b = d.find(&s.root, &["b"])?;
    let [bx, by, bw, bh] = d.runtime().bounds(&b)?.ok_or("b layout")?.values();
    pointer(&mut d, bx + bw * 0.9, by + bh * 0.1, PointerKind::Press(PointerButton::Primary))?;
    pointer(&mut d, bx + bw * 0.9, by + bh * 0.1, PointerKind::Release(PointerButton::Primary))?;
    close(json_point(d.runtime(), "b")?, [0.9, 0.1]);
    close(json_point(d.runtime(), "a")?, [0.5, 0.5]);
    d.runtime_mut().focus(&a)?;
    d.settle()?;
    d.key(KeyCode::ArrowLeft)?;
    close(json_point(d.runtime(), "a")?, [0.49, 0.5]);
    close(json_point(d.runtime(), "b")?, [0.9, 0.1]);
    assert!((s.a[0].get()? - 0.49).abs() < 1e-6 && (s.b[1].get()? - 0.1).abs() < 1e-6);
    Ok(())
}

/// Informational (not required by the task): focus loss during a drag, which
/// the README says restores the pre-press point via `cancel_input`.
#[test]
fn v8_focus_loss_cancels_drag() -> R {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let i = ids(&d, &state.root)?;
    let [x, y, w, h] = i.rect;
    pointer(&mut d, x + w * 0.1, y + h * 0.9, PointerKind::Press(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [0.1, 0.9]);
    d.runtime_mut().focus(&i.xf)?;
    d.settle()?;
    close(json_point(d.runtime(), "picker")?, [0.5, 0.5]);
    pointer(&mut d, x + w * 0.7, y + h * 0.7, PointerKind::Move)?;
    pointer(&mut d, x + w * 0.7, y + h * 0.7, PointerKind::Release(PointerButton::Primary))?;
    close(json_point(d.runtime(), "picker")?, [0.5, 0.5]);
    Ok(())
}
