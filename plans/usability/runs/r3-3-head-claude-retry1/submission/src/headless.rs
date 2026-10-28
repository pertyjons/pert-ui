//! A headless mode: the application rendered on the CPU, without a window.

use std::error::Error;

use pert_ui::cpu::CpuRenderer;
use pert_ui::prelude::*;
use pert_ui::{CaptureRequest, CaptureTarget, Viewport};

use crate::app::build;

/// The logical size of the captured window.
pub const SIZE: (f32, f32) = (480.0, 320.0);

/// Renders the application in `mode` with the picker focused and returns the
/// window as PNG bytes.
pub fn capture_png(mode: ThemeMode) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(SIZE.0, SIZE.1, 1.0)?)?;
    ui.theme().set_mode(mode)?;
    let app = build(&mut ui)?;
    app.x.set(0.3)?;
    app.y.set(0.7)?;
    ui.flush(std::time::Duration::ZERO)?;
    ui.focus(&app.picker)?;
    ui.flush(std::time::Duration::ZERO)?;
    #[cfg(debug_assertions)]
    {
        let report = ui.validation_report();
        if !report.issues.is_empty() {
            return Err(format!("development validation: {:?}", report.issues).into());
        }
    }
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
        &ui.primary_window(),
    )));
    ui.flush(std::time::Duration::ZERO)?;
    CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.try_reply()?.value?;
    Ok(capture.image.encode_png())
}
