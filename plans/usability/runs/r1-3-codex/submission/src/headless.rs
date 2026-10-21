use crate::*;
use std::{path::Path, time::Duration};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn flush(ui: &mut Runtime) -> Result<()> {
    ui.flush(Duration::ZERO)?;
    Ok(())
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<()> {
    ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
    flush(ui)
}
fn key(ui: &mut Runtime, code: KeyCode) -> Result<()> {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))?;
    }
    flush(ui)
}
/// Read only public inspection data: this deliberately does not downcast a widget.
fn observed(ui: &Runtime) -> (f64, f64) {
    let snapshot = ui.inspect();
    let entry = snapshot
        .components
        .iter()
        .find(|c| c.key == "picker")
        .unwrap();
    let InspectValue::Object(value) = &entry.details.properties["value"] else {
        panic!("object value")
    };
    let InspectValue::Number(x) = value["x"] else {
        panic!("numeric x")
    };
    let InspectValue::Number(y) = value["y"] else {
        panic!("numeric y")
    };
    (x, y)
}
fn expect(ui: &Runtime, x: f64, y: f64) {
    let value = observed(ui);
    assert!(
        (value.0 - x).abs() < 1e-6 && (value.1 - y).abs() < 1e-6,
        "expected ({x},{y}), got {value:?}"
    );
}
pub fn check(output: &Path) -> Result<()> {
    std::fs::create_dir_all(output)?;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    let app = mount(&mut ui)?;
    flush(&mut ui)?;
    let picker = ui.find_descendant(&app.root, "picker")?.unwrap();
    let b = ui.bounds(&picker)?.unwrap();
    assert_eq!(b.width(), b.height());
    let press = PointerKind::Press(PointerButton::Primary);
    let release = PointerKind::Release(PointerButton::Primary);
    pointer(
        &mut ui,
        b.x() + b.width() * 0.5,
        b.y() + b.height() * 0.5,
        press,
    )?;
    pointer(
        &mut ui,
        b.x() + b.width() * 0.75,
        b.y() + b.height() * 0.25,
        PointerKind::Move,
    )?;
    expect(&ui, 0.75, 0.25);
    pointer(
        &mut ui,
        b.x() + b.width() + 80.0,
        b.y() - 40.0,
        PointerKind::Move,
    )?;
    expect(&ui, 1.0, 0.0);
    assert_eq!((app.x.get()?, app.y.get()?), (1.0, 0.0));
    pointer(&mut ui, b.x() + b.width() + 80.0, b.y() - 40.0, release)?;
    // Capture ended: an unrelated move cannot change the point.
    pointer(&mut ui, b.x() + 10.0, b.y() + 10.0, PointerKind::Move)?;
    expect(&ui, 1.0, 0.0);
    pointer(&mut ui, b.x() + 30.0, b.y() + 70.0, press)?;
    pointer(
        &mut ui,
        b.x() - 100.0,
        b.y() + b.height() + 50.0,
        PointerKind::Move,
    )?;
    expect(&ui, 0.0, 1.0);
    key(&mut ui, KeyCode::Escape)?;
    expect(&ui, 1.0, 0.0);
    assert_eq!((app.x.get()?, app.y.get()?), (1.0, 0.0));
    pointer(&mut ui, b.x() - 100.0, b.y() + b.height() + 50.0, release)?;
    ui.focus(&picker)?;
    key(&mut ui, KeyCode::ArrowLeft)?;
    key(&mut ui, KeyCode::ArrowDown)?;
    expect(&ui, 0.99, 0.01);
    ui.native_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    })?;
    key(&mut ui, KeyCode::ArrowLeft)?;
    key(&mut ui, KeyCode::ArrowDown)?;
    expect(&ui, 0.89, 0.11);
    ui.native_modifiers(Modifiers::default())?;
    // Real committed text input into each numeric field.
    for (name, text) in [("x", "0.23"), ("y", "0.78")] {
        let field = ui.find_descendant(&app.root, name)?.unwrap();
        ui.focus(&field)?;
        ui.native_modifiers(Modifiers::PRIMARY)?;
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::KeyA,
                action,
                CommandKey::Character('a'),
            ))?;
        }
        ui.native_modifiers(Modifiers::default())?;
        ui.text_input(text)?;
        key(&mut ui, KeyCode::Enter)?;
    }
    expect(&ui, 0.23, 0.78);
    let reset = ui.find_descendant(&app.root, "reset")?.unwrap();
    let r = ui.bounds(&reset)?.unwrap();
    pointer(
        &mut ui,
        r.x() + r.width() / 2.0,
        r.y() + r.height() / 2.0,
        press,
    )?;
    pointer(
        &mut ui,
        r.x() + r.width() / 2.0,
        r.y() + r.height() / 2.0,
        release,
    )?;
    expect(&ui, 0.5, 0.5);
    assert_eq!((app.x.get()?, app.y.get()?), (0.5, 0.5));
    ui.focus(&picker)?;
    ui.set_capture_support(true);
    for (mode, name) in [
        (ThemeMode::Dark, "dark.png"),
        (ThemeMode::Light, "light.png"),
    ] {
        ui.theme().set_mode(mode)?;
        flush(&mut ui)?;
        let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
            &ui.primary_window(),
        )));
        flush(&mut ui)?;
        pert_ui::cpu::CpuRenderer::new().serve_captures(&mut ui);
        let capture = pending.wait()?.value?;
        std::fs::write(output.join(name), capture.image.encode_png())?;
        assert!(
            ui.inspect()
                .components
                .iter()
                .any(|c| c.key == "picker" && c.focused)
        );
    }
    std::fs::write(
        output.join("inspection.json"),
        serde_json::to_string_pretty(&ui.inspect())?,
    )?;
    println!(
        "All six acceptance checks passed; captures and inspection: {}",
        output.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn application_acceptance() {
        super::check(std::path::Path::new("artifacts")).unwrap();
    }
}
