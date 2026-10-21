//! Renders the main window on the CPU, with no display or GPU, and writes a
//! PNG capture of it.
mod support;

use pert_ui::cpu::CpuRenderer;
use pert_ui::{CaptureRequest, CaptureTarget};
use std::time::Duration;
use support::*;

#[test]
fn main_window_png_capture() -> TestResult {
    let mut app = Bookmarks::open()?;
    app.add("Docs", "https://docs.rs");
    app.type_into(NAME, "No URL");
    app.click(ADD); // shows the error line
    app.ui.clear_focus();
    app.ui.flush(Duration::ZERO)?;

    let ui = &mut app.ui;
    ui.set_capture_support(true);
    let window = ui.primary_window();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    ui.flush(Duration::ZERO)?;
    assert_eq!(CpuRenderer::new().serve_captures(ui), 1);
    let capture = pending.wait()?.value?;

    let image = &capture.image;
    assert_eq!((image.width(), image.height()), (800, 900));
    // Not a blank frame: the window shows more than one color.
    let first = &image.pixels()[..4];
    assert!(image.pixels().chunks(4).any(|pixel| pixel != first));

    let png = image.encode_png();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bookmarks-main-window.png");
    std::fs::write(&path, &png)?;
    println!("wrote {}", path.display());
    Ok(())
}
