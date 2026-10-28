//! Independent acceptance checks added by the verifier (not part of the
//! submission). They drive the unchanged application only through
//! `testing::Driver` input and read it back through semantics and a CPU capture.
use pert_ui::prelude::*;
use pert_ui::testing::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NAME: &[&str] = &["content", "form", "name-field", "name"];
const URL: &[&str] = &["content", "form", "url-field", "url"];
const ADD: &[&str] = &["content", "add"];
const FILTER: &[&str] = &["content", "filter-field", "filter"];
const STATUS: &[&str] = &["content", "status"];
const LIST: &[&str] = &["content", "list"];

fn start() -> Result<(Runtime, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    let root = bookmarks::build(&mut ui)?;
    Ok((ui, root))
}

fn status(driver: &Driver<'_>, root: &WidgetId) -> Result<Option<String>, TestError> {
    Ok(driver.semantics(&driver.find(root, STATUS)?)?.name)
}

fn filter_text(driver: &Driver<'_>, root: &WidgetId) -> Result<String, TestError> {
    Ok(match driver.semantics(&driver.find(root, FILTER)?)?.value {
        Some(SemanticValue::Text(text)) => text.to_string(),
        _ => String::new(),
    })
}

fn row_names(driver: &Driver<'_>, root: &WidgetId) -> Result<Vec<(String, String)>, TestError> {
    let list = driver.find(root, LIST)?;
    driver
        .address_children(&list)?
        .into_iter()
        .map(|row| {
            let name = driver.semantics(&driver.find(&row.id, &["name"])?)?.name;
            Ok((row.segment, name.unwrap_or_default()))
        })
        .collect()
}

fn add(driver: &mut Driver<'_>, root: &WidgetId, name: &str, url: &str) -> Result<(), TestError> {
    let field = driver.find(root, NAME)?;
    driver.replace_text(&field, name)?;
    let field = driver.find(root, URL)?;
    driver.replace_text(&field, url)?;
    let button = driver.find(root, ADD)?;
    driver.click(&button)
}

/// Remove pressed in a filtered view deletes only that bookmark from the
/// whole list; the status follows, and Ctrl+L with a button focused clears
/// the filter and shows the remaining bookmarks again.
#[test]
fn verifier_remove_in_filtered_view_and_clear_filter_from_button_focus() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    add(&mut driver, &root, "Rust Book", "https://doc.rust-lang.org/book")?;
    let field = driver.find(&root, FILTER)?;
    driver.replace_text(&field, " rUsT ")?;
    assert_eq!(
        row_names(&driver, &root)?,
        [("1".to_owned(), "Rust".to_owned()), ("3".to_owned(), "Rust Book".to_owned())]
    );
    assert_eq!(status(&driver, &root)?.as_deref(), Some("2 of 3 bookmarks"));

    let remove = driver.find(&root, &["content", "list", "3", "remove"])?;
    driver.click(&remove)?;
    assert_eq!(row_names(&driver, &root)?, [("1".to_owned(), "Rust".to_owned())]);
    assert_eq!(status(&driver, &root)?.as_deref(), Some("1 of 2 bookmarks"));

    // Focus on the Add button (not an editor), then the chord.
    let add_button = driver.find(&root, ADD)?;
    driver.runtime_mut().focus(&add_button)?;
    driver.settle()?;
    assert_eq!(driver.runtime().focused(), Some(add_button));
    driver.chord(KeyChord::primary('l'), KeyCode::KeyL)?;
    assert_eq!(filter_text(&driver, &root)?, "");
    assert_eq!(status(&driver, &root)?.as_deref(), Some("2 bookmarks"));
    assert_eq!(
        row_names(&driver, &root)?,
        [("1".to_owned(), "Rust".to_owned()), ("2".to_owned(), "Crates".to_owned())]
    );
    Ok(())
}

/// An independent CPU-renderer capture of the main window after driven input,
/// written as a PNG and checked to differ from the untouched start.
#[test]
fn verifier_cpu_capture_png_of_main_window() -> TestResult {
    fn capture(driver: &mut Driver<'_>) -> Result<FrameImage, Box<dyn std::error::Error>> {
        driver.runtime().set_capture_support(true);
        let window = driver.runtime().primary_window();
        let pending = driver
            .runtime()
            .request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
        driver.settle()?;
        CpuRenderer::new().serve_captures(driver.runtime_mut());
        Ok(pending.try_reply()?.value?.image)
    }
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let before = capture(&mut driver)?;
    add(&mut driver, &root, "Verifier", "https://example.org")?;
    let field = driver.find(&root, FILTER)?;
    driver.replace_text(&field, "ver")?;
    driver.clear_focus()?;
    assert_eq!(status(&driver, &root)?.as_deref(), Some("1 of 3 bookmarks"));
    let after = capture(&mut driver)?;
    assert_eq!((after.width(), after.height()), (640, 480));
    assert_ne!(before.pixels(), after.pixels());

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    for (name, image) in [("verifier-start.png", &before), ("verifier-filtered.png", &after)] {
        let file = dir.join(name);
        std::fs::write(&file, image.encode_png())?;
        assert!(std::fs::read(&file)?.starts_with(b"\x89PNG\r\n\x1a\n"));
        println!("capture written to {}", file.display());
    }
    Ok(())
}
