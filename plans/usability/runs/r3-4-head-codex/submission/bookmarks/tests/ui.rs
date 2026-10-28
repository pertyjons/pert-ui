//! Public-input regression tests for the seven behaviors in README.md.
use pert_ui::prelude::*;
use pert_ui::testing::*;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Screen {
    ui: Runtime,
    root: WidgetId,
}

impl Screen {
    fn new() -> Result<Self> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 700.0, 1.0)?)?;
        let root = bookmarks::build(&mut ui)?;
        Driver::new(&mut ui).settle()?;
        Ok(Self { ui, root })
    }

    fn find(&mut self, path: &[&str]) -> Result<WidgetId> {
        Ok(Driver::new(&mut self.ui).find(&self.root, path)?)
    }

    fn semantics(&mut self, path: &[&str]) -> Result<Semantics> {
        let id = self.find(path)?;
        Ok(Driver::new(&mut self.ui).semantics(&id)?)
    }

    fn label(&mut self, path: &[&str]) -> Result<String> {
        Ok(self.semantics(path)?.name.unwrap_or_default())
    }

    fn text(&mut self, path: &[&str]) -> Result<String> {
        match self.semantics(path)?.value {
            Some(SemanticValue::Text(value)) => Ok(value),
            other => Err(format!("expected text at {path:?}, got {other:?}").into()),
        }
    }

    fn replace(&mut self, path: &[&str], text: &str) -> Result {
        let id = self.find(path)?;
        Driver::new(&mut self.ui).replace_text(&id, text)?;
        Ok(())
    }

    fn click(&mut self, path: &[&str]) -> Result {
        let id = self.find(path)?;
        Driver::new(&mut self.ui).click(&id)?;
        Ok(())
    }

    fn fields(&mut self, name: &str, url: &str) -> Result {
        self.replace(NAME, name)?;
        self.replace(URL, url)
    }

    fn add(&mut self, name: &str, url: &str) -> Result {
        self.fields(name, url)?;
        self.click(ADD)
    }

    fn rows(&mut self) -> Result<Vec<String>> {
        let list = self.find(&["content", "list"])?;
        Ok(Driver::new(&mut self.ui)
            .address_children(&list)?
            .into_iter()
            .map(|row| row.segment)
            .collect())
    }

    fn row_label(&mut self, row: &str, key: &str) -> Result<String> {
        self.label(&["content", "list", row, key])
    }
}

const NAME: &[&str] = &["content", "form", "name-field", "name"];
const URL: &[&str] = &["content", "form", "url-field", "url"];
const ADD: &[&str] = &["content", "add"];
const FILTER: &[&str] = &["content", "filter-field", "filter"];
const ERROR: &[&str] = &["content", "error"];
const STATUS: &[&str] = &["content", "status"];

#[test]
fn add_appends_and_trims_name() -> Result {
    let mut app = Screen::new()?;
    for (id, name, url) in [
        ("3", "  Docs  ", "https://docs.rs"),
        ("4", "  Local  ", "http://localhost"),
    ] {
        app.add(name, url)?;
        assert_eq!(app.row_label(id, "name")?, name.trim());
        assert_eq!(app.row_label(id, "url")?, url);
    }
    assert_eq!(app.rows()?, ["1", "2", "3", "4"]);
    assert_eq!(app.row_label("1", "name")?, "Rust");
    assert_eq!(app.row_label("2", "name")?, "Crates");
    Ok(())
}

#[test]
fn refused_add_preserves_fields_and_success_clears_error() -> Result {
    let mut app = Screen::new()?;
    for (name, url, message) in [
        ("", "invalid", "Enter a name."),
        ("   ", "https://docs.rs", "Enter a name."),
        (
            "Docs",
            "ftp://docs.rs",
            "The URL must start with http:// or https://.",
        ),
        ("Docs", "", "The URL must start with http:// or https://."),
    ] {
        app.add(name, url)?;
        assert_eq!(app.label(ERROR)?, message);
        assert_eq!(app.text(NAME)?, name);
        assert_eq!(app.text(URL)?, url);
        assert_eq!(app.rows()?, ["1", "2"]);
    }
    app.add("Docs", "https://docs.rs")?;
    assert_eq!(app.rows()?, ["1", "2", "3"]);
    assert_eq!(app.semantics(ERROR)?.name, None);
    Ok(())
}

#[test]
fn successful_add_clears_both_fields() -> Result {
    let mut app = Screen::new()?;
    app.add("Docs", "https://docs.rs")?;
    assert_eq!(app.rows()?, ["1", "2", "3"]);
    assert_eq!(app.text(NAME)?, "");
    assert_eq!(app.text(URL)?, "");
    Ok(())
}

#[test]
fn list_shows_rows_and_remove_deletes_only_its_bookmark() -> Result {
    let mut app = Screen::new()?;
    assert_eq!(app.rows()?, ["1", "2"]);
    for (id, name, url) in [
        ("1", "Rust", "https://www.rust-lang.org"),
        ("2", "Crates", "https://crates.io"),
    ] {
        assert_eq!(app.row_label(id, "name")?, name);
        assert_eq!(app.row_label(id, "url")?, url);
        let remove = app.semantics(&["content", "list", id, "remove"])?;
        assert_eq!(remove.role, SemanticRole::Button);
        assert_eq!(remove.name.as_deref(), Some("Remove"));
    }
    // A duplicate name must remain distinguishable by row identity.
    app.add("Rust", "http://another.example")?;
    app.click(&["content", "list", "1", "remove"])?;
    assert_eq!(app.rows()?, ["2", "3"]);
    assert_eq!(app.row_label("2", "url")?, "https://crates.io");
    assert_eq!(app.row_label("3", "name")?, "Rust");
    assert_eq!(app.row_label("3", "url")?, "http://another.example");
    Ok(())
}

#[test]
fn filter_matches_name_substrings_ignoring_case_and_spaces() -> Result {
    let mut app = Screen::new()?;
    for (filter, expected) in [
        ("  uS  ", vec!["1"]),
        ("R", vec!["1", "2"]),
        ("aTe", vec!["2"]),
        ("https", vec![]),
        ("missing", vec![]),
        ("   ", vec!["1", "2"]),
        ("", vec!["1", "2"]),
    ] {
        app.replace(FILTER, filter)?;
        assert_eq!(app.rows()?, expected, "filter {filter:?}");
    }
    Ok(())
}

#[test]
fn status_counts_visible_and_total_bookmarks() -> Result {
    let mut app = Screen::new()?;
    assert_eq!(app.label(STATUS)?, "2 bookmarks");
    app.add("Docs", "https://docs.rs")?;
    assert_eq!(app.label(STATUS)?, "3 bookmarks");
    app.replace(FILTER, " r ")?;
    assert_eq!(app.label(STATUS)?, "2 of 3 bookmarks");
    app.click(&["content", "list", "1", "remove"])?;
    assert_eq!(app.label(STATUS)?, "1 of 2 bookmarks");
    app.replace(FILTER, "absent")?;
    assert_eq!(app.label(STATUS)?, "0 of 2 bookmarks");
    app.replace(FILTER, "   ")?;
    assert_eq!(app.label(STATUS)?, "2 bookmarks");
    app.click(&["content", "list", "2", "remove"])?;
    assert_eq!(app.label(STATUS)?, "1 bookmarks");
    app.click(&["content", "list", "3", "remove"])?;
    assert_eq!(app.label(STATUS)?, "0 bookmarks");
    Ok(())
}

#[test]
fn clear_filter_keyboard_command_works_wherever_focus_is() -> Result {
    let mut app = Screen::new()?;
    for focus in [Some(FILTER), Some(NAME), Some(URL), Some(ADD), None] {
        app.replace(FILTER, "rust")?;
        assert_eq!(app.rows()?, ["1"]);
        match focus {
            Some(path) => app.click(path)?,
            None => {
                Driver::new(&mut app.ui).clear_focus()?;
            }
        }
        Driver::new(&mut app.ui).chord(KeyChord::primary('l'), KeyCode::KeyL)?;
        assert_eq!(app.text(FILTER)?, "");
        assert_eq!(app.rows()?, ["1", "2"]);
        assert_eq!(app.label(STATUS)?, "2 bookmarks");
    }
    Ok(())
}

#[test]
fn cpu_renderer_captures_main_window_as_png() -> Result {
    let mut app = Screen::new()?;
    app.ui.set_capture_support(true);
    let pending = app
        .ui
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &app.ui.primary_window(),
        )));
    Driver::new(&mut app.ui).settle()?;
    CpuRenderer::new().serve_captures(&mut app.ui);
    let capture = pending.try_reply()?.value?;
    let image = &capture.image;
    assert_eq!((image.width(), image.height()), (900, 700));
    let png = image.encode_png();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures/bookmarks.png");
    std::fs::create_dir_all(file.parent().unwrap())?;
    std::fs::write(&file, &png)?;
    assert_eq!(std::fs::read(&file)?, png);
    Ok(())
}
