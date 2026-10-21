//! Point picker application.
//!
//! `cargo run` renders the application headlessly on the CPU: it focuses the
//! picker, moves the point, and writes `capture-dark.png` and
//! `capture-light.png` (or into the directory given as the first argument),
//! then prints the picker's inspection entry. `cargo run -- --window` opens
//! the native window instead.

use pert_ui::cpu::CpuRenderer;
use pert_ui::{
    App, CaptureRequest, CaptureTarget, KeyAction, KeyCode, Runtime, ThemeMode, Viewport,
};
use point_picker::app;
use std::path::PathBuf;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--window") {
        App::new("Point picker", |ui| {
            app::mount(ui)?;
            Ok(())
        })
        .run()?;
        return Ok(());
    }
    let out = args.first().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    headless(&out)
}

fn headless(out: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(520.0, 300.0, 2.0)?)?;
    let (state, root) = app::mount(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    let picker = ui.find_descendant(&root, "picker")?.ok_or("no picker")?;
    ui.focus(&picker)?;
    // Move the point off centre so the crosshair is visible: (0.70, 0.30).
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    for code in [KeyCode::ArrowRight, KeyCode::ArrowRight, KeyCode::ArrowDown, KeyCode::ArrowDown] {
        ui.key_input(code, KeyAction::Press)?;
        ui.key_input(code, KeyAction::Release)?;
    }
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    println!("point: ({}, {})", state.x.get()?, state.y.get()?);

    ui.set_capture_support(true);
    let mut renderer = CpuRenderer::new();
    for (mode, name) in [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")] {
        ui.theme().set_mode(mode)?;
        ui.flush(Duration::ZERO)?;
        let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(&ui.primary_window())));
        ui.flush(Duration::ZERO)?;
        renderer.serve_captures(&mut ui);
        let capture = pending.wait()?.value?;
        let path = out.join(format!("capture-{name}.png"));
        std::fs::write(&path, capture.image.encode_png())?;
        println!("wrote {}", path.display());
    }

    let id = ui.inspection_id(&picker)?;
    let snapshot = ui.inspect();
    let entry = snapshot.components.iter().find(|c| c.id == id).ok_or("no entry")?;
    println!("focused: {}", ui.focused() == Some(picker));
    println!("semantics: {:?}", entry.details.semantics);
    println!("properties: {:?}", entry.details.properties);
    Ok(())
}
