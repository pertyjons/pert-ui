use pert_ui::{prelude::*, testing::*};
use std::error::Error;

fn read_point(ui: &Runtime) -> [f64; 2] {
    let snapshot = ui.inspect();
    let picker = snapshot
        .components
        .iter()
        .find(|c| c.key == "picker")
        .unwrap();
    ["x", "y"].map(|name| match picker.details.properties.get(name) {
        Some(InspectValue::Number(v)) => *v,
        other => panic!("missing coordinate: {other:?}"),
    })
}
fn close(actual: [f64; 2], expected: [f64; 2]) {
    for i in 0..2 {
        assert!(
            (actual[i] - expected[i]).abs() < 1e-6,
            "{actual:?} != {expected:?}"
        );
    }
}
fn pointer(d: &mut Driver<'_>, x: f32, y: f32, kind: PointerKind) -> Result<(), Box<dyn Error>> {
    d.runtime_mut()
        .pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    d.settle()?;
    Ok(())
}

#[test]
fn application_headless_acceptance() -> Result<(), Box<dyn Error>> {
    let mut app = point_picker::app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    ui.set_viewport(Viewport::new(720.0, 360.0, 1.0)?)?;
    let mut d = Driver::new(ui);
    d.settle()?;
    let picker = d.find(&state.root, &["body", "picker"])?;
    let xf = d.find(&state.root, &["body", "fields", "x"])?;
    let yf = d.find(&state.root, &["body", "fields", "y"])?;
    let reset = d.find(&state.root, &["body", "fields", "reset"])?;
    let [x, y, w, h] = d
        .runtime()
        .bounds(&picker)?
        .ok_or("picker layout")?
        .values();
    assert_eq!(w, h, "square picker");
    pointer(
        &mut d,
        x + w / 2.0,
        y + h / 2.0,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(&mut d, x + w * 0.8, y + h * 0.2, PointerKind::Move)?;
    close(read_point(d.runtime()), [0.8, 0.2]);
    pointer(&mut d, x + w + 60.0, y - 40.0, PointerKind::Move)?;
    close(read_point(d.runtime()), [1.0, 0.0]);
    pointer(
        &mut d,
        x + w + 60.0,
        y - 40.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    close(read_point(d.runtime()), [1.0, 0.0]);
    assert!(state.changes.get()? >= 2);
    // The ordinary numeric controls follow the drag's live writes.
    assert!(matches!(
        d.semantics(&xf)?.value,
        Some(SemanticValue::Number { value: 1.0, .. })
    ));
    assert!(matches!(
        d.semantics(&yf)?.value,
        Some(SemanticValue::Number { value: 0.0, .. })
    ));

    pointer(
        &mut d,
        x + w * 0.3,
        y + h * 0.7,
        PointerKind::Press(PointerButton::Primary),
    )?;
    pointer(&mut d, x - 50.0, y + h + 20.0, PointerKind::Move)?;
    close(read_point(d.runtime()), [0.0, 1.0]);
    d.key(KeyCode::Escape)?;
    close(read_point(d.runtime()), [1.0, 0.0]);
    // A move after cancellation must no longer drag.
    pointer(&mut d, x + w / 2.0, y + h / 2.0, PointerKind::Move)?;
    close(read_point(d.runtime()), [1.0, 0.0]);
    pointer(
        &mut d,
        x + w / 2.0,
        y + h / 2.0,
        PointerKind::Release(PointerButton::Primary),
    )?;
    d.key(KeyCode::ArrowLeft)?;
    d.key(KeyCode::ArrowDown)?;
    close(read_point(d.runtime()), [0.99, 0.01]);
    d.chord(
        KeyChord::named(KeyCode::ArrowLeft).shifted(),
        KeyCode::ArrowLeft,
    )?;
    d.chord(
        KeyChord::named(KeyCode::ArrowDown).shifted(),
        KeyCode::ArrowDown,
    )?;
    close(read_point(d.runtime()), [0.89, 0.11]);
    d.key(KeyCode::ArrowRight)?;
    d.key(KeyCode::ArrowUp)?;
    close(read_point(d.runtime()), [0.90, 0.10]);

    d.replace_text(&xf, "0.25")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&yf, "0.75")?;
    d.key(KeyCode::Enter)?;
    close(read_point(d.runtime()), [0.25, 0.75]);
    d.click(&reset)?;
    close(read_point(d.runtime()), [0.5, 0.5]);
    // Owner writes are observed without calling the control's callback.
    state.x.set(0.35)?;
    state.y.set(0.65)?;
    d.settle()?;
    close(read_point(d.runtime()), [0.35, 0.65]);
    d.runtime_mut().focus(&picker)?;
    d.settle()?;
    std::fs::create_dir_all("artifacts")?;
    d.runtime().set_capture_support(true);
    for (mode, name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        d.runtime().theme().set_mode(mode)?;
        d.settle()?;
        assert_eq!(d.runtime().focused(), Some(picker.clone()));
        let pending = d
            .runtime()
            .request_capture(CaptureRequest::new(CaptureTarget::window(
                &d.runtime().primary_window(),
            )));
        d.settle()?;
        CpuRenderer::new().serve_captures(d.runtime_mut());
        let capture = pending.try_reply()?.value?;
        let png = capture.image.encode_png();
        assert!(png.starts_with(b"\x89PNG"));
        std::fs::write(format!("artifacts/{name}.png"), png)?;
    }
    let snapshot = d.runtime().inspect();
    std::fs::write(
        "artifacts/inspection.json",
        serde_json::to_string_pretty(&snapshot)?,
    )?;
    assert!(
        snapshot.components.iter().any(|c| c.key == "picker"
            && c.details.properties.get("x") == Some(&InspectValue::Number(0.35)))
    );
    #[cfg(debug_assertions)]
    assert!(
        d.runtime().validation_report().issues.is_empty(),
        "{:?}",
        d.runtime().validation_report()
    );
    Ok(())
}
