//! Renders the main window with the CPU renderer and writes a PNG capture.
use pert_ui::prelude::*;
use pert_ui::testing::*;

#[test]
fn main_window_capture_is_written_as_png() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 640.0, 1.0)?)?;
    bookmarks::build(&mut ui)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;

    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &driver.runtime().primary_window(),
    )));
    driver.settle()?;
    assert_eq!(CpuRenderer::new().serve_captures(driver.runtime_mut()), 1);
    let capture = pending.wait()?.value?;
    let image = &capture.image;
    assert_eq!((image.width(), image.height()), (480, 640));

    // Something other than the background was drawn.
    let first = &image.pixels()[..4];
    assert!(image.pixels().chunks(4).any(|pixel| pixel != first));

    let png = image.encode_png();
    assert!(png.starts_with(b"\x89PNG"));
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bookmarks.png");
    std::fs::write(&path, &png)?;
    eprintln!("capture written to {}", path.display());
    Ok(())
}
