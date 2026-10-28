//! Renders the editor on the CPU, clean and with refused input, for a visual check.
use std::path::PathBuf;

use pert_ui::prelude::*;
use pert_ui::testing::*;

fn capture(driver: &mut Driver<'_>, file: &str) -> Result<(), Box<dyn std::error::Error>> {
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &driver.runtime().primary_window(),
    )));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(file);
    std::fs::write(&path, capture.image.encode_png())?;
    println!("wrote {}", path.display());
    Ok(())
}

#[test]
fn renders_clean_and_refused_states() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("capture");
    std::fs::create_dir_all(&dir)?;
    let mut app = settings_editor::app(dir.join("settings.json")).headless()?;
    let (ui, editor) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.settle()?;
    capture(&mut driver, "clean.png")?;
    let root = editor.root.clone();
    let prefix = driver.find(&root, &["content", "output", "fields", "prefix-field", "prefix"])?;
    driver.replace_text(&prefix, "my photos!")?;
    driver.key(KeyCode::Enter)?;
    let quality = driver.find(&root, &["content", "quality", "fields", "quality-field", "quality"])?;
    driver.replace_text(&quality, "150")?;
    driver.key(KeyCode::Enter)?;
    let width = driver.find(&root, &["content", "quality", "fields", "max-width-field", "max-width"])?;
    driver.replace_text(&width, "2000")?;
    driver.key(KeyCode::Enter)?;
    let apply = driver.find(&root, &["content", "buttons", "apply"])?;
    println!("width={} apply disabled={}", editor.max_width.get()?, driver.semantics(&apply)?.disabled);
    println!("quality says: {:?}", driver.semantics(&quality)?.description);
    capture(&mut driver, "refused.png")?;
    let revert = driver.find(&root, &["content", "buttons", "revert"])?;
    driver.click(&revert)?;
    driver.replace_text(&quality, "50")?;
    driver.key(KeyCode::Enter)?;
    driver.clear_focus()?;
    for key in ["apply", "revert"] {
        let id = driver.find(&root, &["content", "buttons", key])?;
        let q = driver.runtime().query(&InspectionQuery::components(Selector::id(driver.runtime().inspection_id(&id)?)).detail(Detail::Full));
        println!("{key}: disabled={} props={:?}", driver.semantics(&id)?.disabled, q.components[0].details.properties);
    }
    capture(&mut driver, "dirty.png")?;
    drop(driver);
    app.shutdown()?;
    Ok(())
}
