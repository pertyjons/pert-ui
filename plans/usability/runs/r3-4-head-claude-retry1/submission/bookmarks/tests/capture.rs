//! Renders the main window with the CPU renderer and writes a PNG capture.
use pert_ui::prelude::*;
use pert_ui::testing::*;

#[test]
fn main_window_renders_to_a_png_capture() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    let root = bookmarks::build(&mut ui)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    // Show a refused add, so the capture holds every part of the window.
    let name = driver.find(&root, &["content", "form", "name-field", "name"])?;
    driver.replace_text(&name, "Docs")?;
    let url = driver.find(&root, &["content", "form", "url-field", "url"])?;
    driver.replace_text(&url, "docs.rs")?;
    let add = driver.find(&root, &["content", "add"])?;
    driver.click(&add)?;
    driver.clear_focus()?;

    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &driver.runtime().primary_window(),
    )));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let image: &FrameImage = &capture.image;
    assert_eq!((image.width(), image.height()), (640, 480));
    // Something is drawn: the frame is not one flat color.
    let pixels = image.pixels();
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));

    let file = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bookmarks.png");
    std::fs::write(&file, image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    println!("capture written to {}", file.display());
    Ok(())
}
