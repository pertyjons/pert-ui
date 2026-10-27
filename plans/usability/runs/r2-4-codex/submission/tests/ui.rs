use pert_ui::SemanticRole;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::{error::Error, path::PathBuf, time::Duration};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

// Only build is called on the app. Actions use host input; assertions inspect
// the mounted controls, never the app's signals or validation/filter helpers.
struct App {
    ui: Runtime,
    root: WidgetId,
}

impl App {
    fn new() -> Result<Self> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 700.0, 1.0)?)?;
        let root = bookmarks::build(&mut ui)?;
        Driver::new(&mut ui).settle()?;
        Ok(Self { ui, root })
    }

    fn find(&self, path: &[&str]) -> Result<WidgetId> {
        self.ui
            .find(&self.root, path)?
            .ok_or_else(|| format!("missing widget: {path:?}").into())
    }

    fn field(&self, field: &str) -> Result<WidgetId> {
        self.find(match field {
            "name" => &["content", "form", "name-field", "name"],
            "url" => &["content", "form", "url-field", "url"],
            "filter" => &["content", "filter-field", "filter"],
            _ => panic!("unknown field: {field}"),
        })
    }

    fn replace(&mut self, field: &str, text: &str) -> Result {
        let id = self.field(field)?;
        Driver::new(&mut self.ui).replace_text(&id, text)?;
        Ok(())
    }

    fn value(&mut self, field: &str) -> Result<String> {
        let id = self.field(field)?;
        match Driver::new(&mut self.ui).semantics(&id)?.value {
            Some(SemanticValue::Text(text)) => Ok(text),
            other => Err(format!("expected text for {field}, got {other:?}").into()),
        }
    }

    fn text(&mut self, path: &[&str]) -> Result<String> {
        let id = self.find(path)?;
        let semantics = Driver::new(&mut self.ui).semantics(&id)?;
        assert_eq!(semantics.role, SemanticRole::Label);
        // Inspection normalizes an empty static name to None.
        Ok(semantics.name.unwrap_or_default())
    }

    fn click(&mut self, path: &[&str]) -> Result {
        let id = self.find(path)?;
        let mut driver = Driver::new(&mut self.ui);
        // Keep repeated Add/Remove clicks separate from double-click gestures.
        driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
        driver.click(&id)?;
        Ok(())
    }

    fn add(&mut self, name: &str, url: &str) -> Result {
        self.replace("name", name)?;
        self.replace("url", url)?;
        self.click(&["content", "add"])
    }

    fn rows(&self) -> Result<Vec<String>> {
        let list = self.find(&["content", "list"])?;
        // Each's documented structural Column is invisible to find paths,
        // but children() includes it and preserves actual display order.
        let containers = self.ui.children(&list)?;
        assert_eq!(containers.len(), 1);
        assert_eq!(self.ui.key(&containers[0])?, "items");
        self.ui
            .children(&containers[0])?
            .iter()
            .map(|id| Ok(self.ui.key(id)?.to_owned()))
            .collect()
    }

    fn assert_row(&mut self, id: &str, name: &str, url: &str) -> Result {
        assert_eq!(self.text(&["content", "list", id, "name"])?, name);
        assert_eq!(self.text(&["content", "list", id, "url"])?, url);
        let remove = self.find(&["content", "list", id, "remove"])?;
        let semantics = Driver::new(&mut self.ui).semantics(&remove)?;
        assert_eq!(semantics.name.as_deref(), Some("Remove"));
        assert_eq!(semantics.role, SemanticRole::Button);
        Ok(())
    }
}

#[test]
fn add_appends_and_trims_the_name() -> Result {
    let mut app = App::new()?;
    app.add("  Guide  ", "https://example.org/guide")?;
    app.add("HTTP", "http://example.org")?;
    assert_eq!(app.rows()?, ["1", "2", "3", "4"]);
    app.assert_row("1", "Rust", "https://www.rust-lang.org")?;
    app.assert_row("2", "Crates", "https://crates.io")?;
    app.assert_row("3", "Guide", "https://example.org/guide")?;
    app.assert_row("4", "HTTP", "http://example.org")
}

#[test]
fn refused_add_keeps_fields_and_success_clears_the_error() -> Result {
    let mut app = App::new()?;
    for (name, url, error) in [
        ("", "bad", "Enter a name."),
        ("   ", "https://example.org", "Enter a name."),
        (
            "Name",
            "ftp://example.org",
            "The URL must start with http:// or https://.",
        ),
        ("Name", "", "The URL must start with http:// or https://."),
    ] {
        app.add(name, url)?;
        assert_eq!(app.rows()?, ["1", "2"]);
        assert_eq!(app.text(&["content", "error"])?, error);
        assert_eq!(app.value("name")?, name);
        assert_eq!(app.value("url")?, url);
    }
    app.add("Valid", "https://example.org")?;
    assert_eq!(app.rows()?, ["1", "2", "3"]);
    assert_eq!(app.text(&["content", "error"])?, "");
    Ok(())
}

#[test]
fn successful_add_clears_both_fields() -> Result {
    let mut app = App::new()?;
    app.add("New", "http://example.org")?;
    assert_eq!(app.rows()?, ["1", "2", "3"]);
    assert_eq!(app.value("name")?, "");
    assert_eq!(app.value("url")?, "");
    Ok(())
}

#[test]
fn list_shows_each_bookmark_and_remove_deletes_only_its_row() -> Result {
    let mut app = App::new()?;
    app.assert_row("1", "Rust", "https://www.rust-lang.org")?;
    app.assert_row("2", "Crates", "https://crates.io")?;
    // Duplicate names must still be independently removable by identity.
    app.add("Rust", "https://example.org/other-rust")?;
    app.assert_row("3", "Rust", "https://example.org/other-rust")?;
    app.click(&["content", "list", "1", "remove"])?;
    assert_eq!(app.rows()?, ["2", "3"]);
    app.assert_row("2", "Crates", "https://crates.io")?;
    app.assert_row("3", "Rust", "https://example.org/other-rust")?;
    app.click(&["content", "list", "3", "remove"])?;
    assert_eq!(app.rows()?, ["2"]);
    Ok(())
}

#[test]
fn filter_matches_substrings_ignoring_case_and_surrounding_spaces() -> Result {
    let mut app = App::new()?;
    for (filter, expected) in [
        ("  uS  ", vec!["1"]),
        ("rA", vec!["2"]),
        ("no match", vec![]),
        ("  ", vec!["1", "2"]),
        ("", vec!["1", "2"]),
    ] {
        app.replace("filter", filter)?;
        assert_eq!(app.rows()?, expected, "filter {filter:?}");
    }
    Ok(())
}

#[test]
fn status_tracks_visible_and_total_counts() -> Result {
    let mut app = App::new()?;
    assert_eq!(app.text(&["content", "status"])?, "2 bookmarks");
    app.replace("filter", "rust")?;
    assert_eq!(app.text(&["content", "status"])?, "1 of 2 bookmarks");
    app.add("Rust guide", "https://example.org")?;
    assert_eq!(app.text(&["content", "status"])?, "2 of 3 bookmarks");
    app.replace("filter", "missing")?;
    assert_eq!(app.text(&["content", "status"])?, "0 of 3 bookmarks");
    app.replace("filter", "  ")?;
    assert_eq!(app.text(&["content", "status"])?, "3 bookmarks");
    app.click(&["content", "list", "1", "remove"])?;
    app.click(&["content", "list", "3", "remove"])?;
    assert_eq!(app.text(&["content", "status"])?, "1 bookmarks");
    app.click(&["content", "list", "2", "remove"])?;
    assert_eq!(app.text(&["content", "status"])?, "0 bookmarks");
    Ok(())
}

#[test]
fn clear_filter_keyboard_command_works_wherever_focus_is() -> Result {
    let mut app = App::new()?;
    // Every focusable control, plus no focus. Build the row button after
    // filtering so its handle always names the current mounted row.
    for target in [
        None,
        Some("filter"),
        Some("name"),
        Some("url"),
        Some("add"),
        Some("remove"),
    ] {
        app.replace("filter", "rust")?;
        assert_eq!(app.rows()?, ["1"]);
        let focus = match target {
            None => None,
            Some("add") => Some(app.find(&["content", "add"])?),
            Some("remove") => Some(app.find(&["content", "list", "1", "remove"])?),
            Some(field) => Some(app.field(field)?),
        };
        let mut driver = Driver::new(&mut app.ui);
        if let Some(id) = focus {
            driver.runtime_mut().focus(&id)?;
        } else {
            driver.runtime_mut().clear_focus();
        }
        driver.chord(KeyChord::primary('l'), KeyCode::KeyL)?;
        assert_eq!(app.value("filter")?, "", "focus {target:?}");
        assert_eq!(app.rows()?, ["1", "2"]);
        assert_eq!(app.text(&["content", "status"])?, "2 bookmarks");
    }
    Ok(())
}

#[test]
fn main_window_cpu_capture_writes_png() -> Result {
    let mut app = App::new()?;
    app.ui.set_capture_support(true);
    let pending = app
        .ui
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &app.ui.primary_window(),
        )));
    let mut driver = Driver::new(&mut app.ui);
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (900, 700));
    // A blank image should not count as rendering the application's UI.
    let pixels = capture.image.pixels();
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    let png = capture.image.encode_png();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("captures/main-window.png");
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, &png)?;
    assert_eq!(std::fs::read(path)?, png);
    Ok(())
}
