//! Acceptance 5: a CPU capture of the window with two notes open, one of
//! them modified, written to `capture.png` in the package directory.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::fs;

#[test]
fn capture_two_notes_one_modified() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("notes-capture-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("groceries.txt"), "milk\neggs\nbread\n")?;
    fs::write(dir.join("ideas.txt"), "A notes editor\nbuilt with pert-ui.\n")?;
    fs::write(dir.join("todo.txt"), "write tests\n")?;
    fs::write(dir.join("photo.jpg"), "not a note")?;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
    let root = notes::build(&mut ui, dir)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let list = driver.runtime().find_descendant(&root, "notes")?.ok_or("no list")?;
    for note in ["groceries.txt", "ideas.txt"] {
        let row = driver.find(&list, &[note])?;
        driver.click(&row)?;
    }
    driver.type_text("Draft: ")?;

    let tabs = driver.runtime().find_descendant(&root, "tabs")?.ok_or("no tabs")?;
    let titles: Vec<_> = driver.semantics(&tabs)?.items.into_iter().filter_map(|i| i.name).collect();
    assert_eq!(titles, ["groceries.txt", "ideas.txt *"]);

    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (960, 640));
    let png = capture.image.encode_png();
    fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/capture.png"), &png)?;
    Ok(())
}
