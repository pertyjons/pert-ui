use pert_ui::{InspectValue, ThemeMode, prelude::*, testing::*};
use std::{error::Error, path::Path};

type CheckResult = Result<(), Box<dyn Error>>;

fn value(driver: &Driver<'_>, picker: &WidgetId) -> Result<(f64, f64), Box<dyn Error>> {
    let id = driver.runtime().inspection_id(picker)?;
    let result = driver
        .runtime()
        .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full));
    let props = &result.components[0].details.properties;
    let number = |key: &str| match props.get(key) {
        Some(InspectValue::Number(n)) => Ok(*n),
        _ => Err(format!("missing numeric inspection property {key}")),
    };
    Ok((number("x")?, number("y")?))
}
fn assert_point(driver: &Driver<'_>, picker: &WidgetId, expected: (f64, f64)) -> CheckResult {
    let actual = value(driver, picker)?;
    assert!(
        (actual.0 - expected.0).abs() < 1e-6 && (actual.1 - expected.1).abs() < 1e-6,
        "expected {expected:?}, got {actual:?}"
    );
    Ok(())
}
fn pointer(driver: &mut Driver<'_>, x: f32, y: f32, kind: PointerKind) -> CheckResult {
    driver
        .runtime_mut()
        .pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    driver.settle()?;
    Ok(())
}
/// The same tree as the native app, driven only through pert-ui input and inspection.
pub fn check(output: &Path) -> CheckResult {
    std::fs::create_dir_all(output)?;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640., 420., 1.)?)?;
    let root = super::mount(&mut ui)?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let picker = d.find(&root, &["content", "body", "picker"])?;
    let x = d.find(&root, &["content", "body", "fields", "x"])?;
    let y = d.find(&root, &["content", "body", "fields", "y"])?;
    let reset = d.find(&root, &["content", "body", "fields", "reset"])?;
    let bounds = d.runtime().bounds(&picker)?.ok_or("picker has no bounds")?;
    assert_eq!((bounds.width(), bounds.height()), (260., 260.));
    let (left, top) = (bounds.x(), bounds.y());
    let press = PointerKind::Press(PointerButton::Primary);
    let release = PointerKind::Release(PointerButton::Primary);

    pointer(&mut d, left + 130., top + 130., press)?;
    pointer(
        &mut d,
        left + 12. + 0.75 * 236.,
        top + 12. + 0.25 * 236.,
        PointerKind::Move,
    )?;
    assert_point(&d, &picker, (0.75, 0.25))?;
    // Capture must keep delivering even far outside both axes.
    pointer(&mut d, left + 400., top - 100., PointerKind::Move)?;
    assert_point(&d, &picker, (1., 0.))?;
    pointer(&mut d, left + 400., top - 100., release)?;
    assert_point(&d, &picker, (1., 0.))?;
    assert!(matches!(
        d.semantics(&x)?.value,
        Some(SemanticValue::Number { value: 1.0, .. })
    ));
    assert!(matches!(
        d.semantics(&y)?.value,
        Some(SemanticValue::Number { value: 0.0, .. })
    ));

    pointer(&mut d, left + 130., top + 130., press)?;
    pointer(&mut d, left - 80., top + 400., PointerKind::Move)?;
    assert_point(&d, &picker, (0., 1.))?;
    d.key(KeyCode::Escape)?;
    assert_point(&d, &picker, (1., 0.))?;
    pointer(&mut d, left - 80., top + 400., release)?;
    pointer(&mut d, left + 130., top + 130., PointerKind::Move)?;
    assert_point(&d, &picker, (1., 0.))?;

    d.click(&reset)?;
    assert_point(&d, &picker, (0.5, 0.5))?;
    d.runtime_mut().focus(&picker)?;
    d.key(KeyCode::ArrowRight)?;
    d.key(KeyCode::ArrowUp)?;
    assert_point(&d, &picker, (0.51, 0.49))?;
    d.chord(
        KeyChord::named(KeyCode::ArrowLeft).shifted(),
        KeyCode::ArrowLeft,
    )?;
    d.chord(
        KeyChord::named(KeyCode::ArrowDown).shifted(),
        KeyCode::ArrowDown,
    )?;
    assert_point(&d, &picker, (0.41, 0.59))?;

    d.replace_text(&x, "0.23")?;
    d.key(KeyCode::Enter)?;
    d.replace_text(&y, "0.78")?;
    d.key(KeyCode::Enter)?;
    assert_point(&d, &picker, (0.23, 0.78))?;
    d.click(&reset)?;
    assert_point(&d, &picker, (0.5, 0.5))?;

    // Context/focus loss restores the starting point as well.
    pointer(&mut d, left + 50., top + 200., press)?;
    d.runtime_mut().focus(&x)?;
    d.settle()?;
    assert_point(&d, &picker, (0.5, 0.5))?;
    pointer(&mut d, left + 50., top + 200., release)?;

    d.runtime_mut().focus(&picker)?;
    d.settle()?;
    assert_eq!(d.runtime().focused(), Some(picker.clone()));
    let snapshot = d.runtime().inspect();
    std::fs::write(
        output.join("inspection.json"),
        serde_json::to_vec_pretty(&snapshot)?,
    )?;
    let picker_id = d.runtime().inspection_id(&picker)?;
    let inspected = snapshot
        .components
        .iter()
        .find(|c| c.id == picker_id)
        .ok_or("picker missing from snapshot")?;
    assert_eq!(
        inspected.details.properties.get("x"),
        Some(&InspectValue::Number(0.5))
    );
    assert_eq!(
        inspected.details.properties.get("y"),
        Some(&InspectValue::Number(0.5))
    );

    d.runtime().set_capture_support(true);
    for (mode, file) in [
        (ThemeMode::Dark, "dark.png"),
        (ThemeMode::Light, "light.png"),
    ] {
        d.runtime().theme().set_mode(mode)?;
        d.settle()?;
        let pending = d
            .runtime()
            .request_capture(CaptureRequest::new(CaptureTarget::window(
                &d.runtime().primary_window(),
            )));
        d.settle()?;
        CpuRenderer::new().serve_captures(d.runtime_mut());
        let capture = pending.wait()?.value?;
        assert_eq!((capture.image.width(), capture.image.height()), (640, 420));
        std::fs::write(output.join(file), capture.image.encode_png())?;
    }
    println!(
        "All six acceptance checks passed; captures and inspection saved to {}",
        output.display()
    );
    Ok(())
}
#[test]
fn acceptance() -> CheckResult {
    check(Path::new("artifacts"))
}
