//! Headless tests of the behaviors listed in the README. Each test mounts the
//! application with `bookmarks::build`, drives it through
//! `pert_ui::testing::Driver` (clicks, typing, chords) and reads what it shows
//! through inspection.
use pert_ui::prelude::*;
use pert_ui::testing::*;

type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;

const NAME: &[&str] = &["content", "form", "name-field", "name"];
const URL: &[&str] = &["content", "form", "url-field", "url"];
const ADD: &[&str] = &["content", "add"];
const ERROR: &[&str] = &["content", "error"];
const FILTER: &[&str] = &["content", "filter-field", "filter"];
const STATUS: &[&str] = &["content", "status"];
const LIST: &[&str] = &["content", "list"];

/// What one row of the list shows.
#[derive(Debug, PartialEq)]
struct Shown {
    key: String,
    name: String,
    url: String,
    remove: String,
}

impl Shown {
    fn new(key: &str, name: &str, url: &str) -> Self {
        Self {
            key: key.to_owned(),
            name: name.to_owned(),
            url: url.to_owned(),
            remove: "Remove".to_owned(),
        }
    }
}

/// The mounted application and its root component.
struct Harness {
    ui: Runtime,
    root: WidgetId,
}

impl Harness {
    fn new() -> Result<Self, Error> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 640.0, 1.0)?)?;
        let root = bookmarks::build(&mut ui)?;
        Driver::new(&mut ui).settle()?;
        Ok(Self { ui, root })
    }

    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }

    /// The component at `path` below the root scroll area `bookmarks`.
    fn find(&mut self, path: &[&str]) -> Result<WidgetId, Error> {
        let root = self.root.clone();
        Ok(self.driver().find(&root, path)?)
    }

    /// Static text (a label's or a button's): the semantic name.
    fn name(&mut self, path: &[&str]) -> Result<String, Error> {
        let id = self.find(path)?;
        Ok(self.driver().semantics(&id)?.name.unwrap_or_default())
    }

    /// An editor's text: the semantic value.
    fn text(&mut self, path: &[&str]) -> Result<String, Error> {
        let id = self.find(path)?;
        match self.driver().semantics(&id)?.value {
            Some(SemanticValue::Text(text)) => Ok(text),
            None => Ok(String::new()),
            other => Err(format!("{path:?} has no text value: {other:?}").into()),
        }
    }

    fn type_into(&mut self, path: &[&str], text: &str) -> TestResult {
        let id = self.find(path)?;
        Ok(self.driver().replace_text(&id, text)?)
    }

    fn click(&mut self, path: &[&str]) -> TestResult {
        let id = self.find(path)?;
        let mut driver = self.driver();
        driver.click(&id)?;
        // Keeps two clicks at one point from counting as a double click.
        driver.advance(ClickCounter::DEFAULT_INTERVAL + std::time::Duration::from_millis(1))?;
        Ok(())
    }

    /// Fills the form and presses Add.
    fn add(&mut self, name: &str, url: &str) -> TestResult {
        self.type_into(NAME, name)?;
        self.type_into(URL, url)?;
        self.click(ADD)
    }

    /// The rows of the list, top to bottom as laid out.
    fn rows(&mut self) -> Result<Vec<Shown>, Error> {
        let list = self.find(LIST)?;
        let mut rows = Vec::new();
        for id in self.descendant_rows(&list)? {
            let key = self.ui.key(&id)?.to_owned();
            let top = self.ui.bounds(&id)?.ok_or("row not laid out")?.y();
            let path = [LIST, &[key.as_str()]].concat();
            let row = Shown {
                name: self.name(&[path.as_slice(), &["name"]].concat())?,
                url: self.name(&[path.as_slice(), &["url"]].concat())?,
                remove: self.name(&[path.as_slice(), &["remove"]].concat())?,
                key,
            };
            rows.push((top, row));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|(_, row)| row).collect())
    }

    /// The row components below the `Each`, through the internal container
    /// that holds them: every descendant whose own address is `list/<key>`.
    fn descendant_rows(&mut self, list: &WidgetId) -> Result<Vec<WidgetId>, Error> {
        let mut rows = Vec::new();
        let mut pending = self.ui.children(list)?;
        while let Some(id) = pending.pop() {
            let key = self.ui.key(&id)?.to_owned();
            match self.ui.find(list, &[key.as_str()])? {
                Some(found) if found == id => rows.push(id),
                _ => pending.extend(self.ui.children(&id)?),
            }
        }
        Ok(rows)
    }
}

fn initial_rows() -> Vec<Shown> {
    vec![
        Shown::new("1", "Rust", "https://www.rust-lang.org"),
        Shown::new("2", "Crates", "https://crates.io"),
    ]
}

/// Behavior 1: Add appends a bookmark at the end with a trimmed name.
#[test]
fn add_appends_bookmark_with_trimmed_name() -> TestResult {
    let mut app = Harness::new()?;
    app.add("  Docs  ", "https://docs.rs")?;
    let mut expected = initial_rows();
    expected.push(Shown::new("3", "Docs", "https://docs.rs"));
    assert_eq!(app.rows()?, expected);

    app.add("Example", "http://example.com")?;
    expected.push(Shown::new("4", "Example", "http://example.com"));
    assert_eq!(app.rows()?, expected);
    Ok(())
}

/// Behavior 2: invalid input is refused with a message, the fields keep their
/// text, and the next successful add clears the message.
#[test]
fn refused_add_shows_error_and_keeps_fields() -> TestResult {
    let mut app = Harness::new()?;
    assert_eq!(app.name(ERROR)?, "");

    // Empty name, valid URL.
    app.add("", "https://docs.rs")?;
    assert_eq!(app.name(ERROR)?, "Enter a name.");
    assert_eq!(app.text(NAME)?, "");
    assert_eq!(app.text(URL)?, "https://docs.rs");
    assert_eq!(app.rows()?, initial_rows());

    // A name of spaces only, with an invalid URL too: the name is checked first.
    app.add("   ", "docs.rs")?;
    assert_eq!(app.name(ERROR)?, "Enter a name.");
    assert_eq!(app.text(NAME)?, "   ");
    assert_eq!(app.text(URL)?, "docs.rs");
    assert_eq!(app.rows()?, initial_rows());

    // Valid name, URL without a scheme.
    app.add("Docs", "ftp://docs.rs")?;
    assert_eq!(app.name(ERROR)?, "The URL must start with http:// or https://.");
    assert_eq!(app.text(NAME)?, "Docs");
    assert_eq!(app.text(URL)?, "ftp://docs.rs");
    assert_eq!(app.rows()?, initial_rows());

    // The next successful add clears the message.
    app.type_into(URL, "https://docs.rs")?;
    app.click(ADD)?;
    assert_eq!(app.name(ERROR)?, "");
    assert_eq!(app.rows()?.len(), 3);
    Ok(())
}

/// Behavior 3: a successful add empties both fields.
#[test]
fn successful_add_clears_fields() -> TestResult {
    let mut app = Harness::new()?;
    app.add("Docs", "https://docs.rs")?;
    assert_eq!(app.text(NAME)?, "");
    assert_eq!(app.text(URL)?, "");
    Ok(())
}

/// Behavior 4: every bookmark is a row with name, URL and Remove; Remove
/// deletes that bookmark only.
#[test]
fn rows_show_bookmarks_and_remove_deletes_one() -> TestResult {
    let mut app = Harness::new()?;
    assert_eq!(app.rows()?, initial_rows());

    app.add("Docs", "https://docs.rs")?;
    app.click(&[LIST, &["1", "remove"]].concat())?;
    assert_eq!(
        app.rows()?,
        [
            Shown::new("2", "Crates", "https://crates.io"),
            Shown::new("3", "Docs", "https://docs.rs"),
        ]
    );

    app.click(&[LIST, &["3", "remove"]].concat())?;
    assert_eq!(app.rows()?, [Shown::new("2", "Crates", "https://crates.io")]);
    Ok(())
}

/// Behavior 5: the filter hides non-matching names, ignoring case and
/// surrounding spaces; an empty filter shows everything.
#[test]
fn filter_narrows_list_by_name() -> TestResult {
    let mut app = Harness::new()?;
    app.add("Rustdoc", "https://doc.rust-lang.org")?;

    app.type_into(FILTER, "rust")?;
    let keys = |rows: Vec<Shown>| rows.into_iter().map(|row| row.key).collect::<Vec<_>>();
    assert_eq!(keys(app.rows()?), ["1", "3"]);

    app.type_into(FILTER, "  CRATES ")?;
    assert_eq!(app.rows()?, [Shown::new("2", "Crates", "https://crates.io")]);

    app.type_into(FILTER, "nothing")?;
    assert_eq!(app.rows()?, []);

    app.type_into(FILTER, "")?;
    assert_eq!(keys(app.rows()?), ["1", "2", "3"]);
    Ok(())
}

/// Behavior 6: the status line counts all bookmarks, or shown of total while
/// filtering.
#[test]
fn status_counts_bookmarks() -> TestResult {
    let mut app = Harness::new()?;
    assert_eq!(app.name(STATUS)?, "2 bookmarks");

    app.add("Docs", "https://docs.rs")?;
    assert_eq!(app.name(STATUS)?, "3 bookmarks");

    app.type_into(FILTER, "Rust")?;
    assert_eq!(app.name(STATUS)?, "1 of 3 bookmarks");
    app.type_into(FILTER, "zzz")?;
    assert_eq!(app.name(STATUS)?, "0 of 3 bookmarks");
    app.type_into(FILTER, "")?;
    assert_eq!(app.name(STATUS)?, "3 bookmarks");

    app.click(&[LIST, &["1", "remove"]].concat())?;
    app.click(&[LIST, &["2", "remove"]].concat())?;
    assert_eq!(app.name(STATUS)?, "1 bookmarks");
    Ok(())
}

/// Behavior 7: Ctrl+L (Cmd+L on macOS) empties the filter wherever focus is.
#[test]
fn clear_filter_command_empties_filter() -> TestResult {
    let mut app = Harness::new()?;

    // Focus in the filter field itself.
    app.type_into(FILTER, "rust")?;
    assert_eq!(app.rows()?.len(), 1);
    app.driver().chord(KeyChord::primary('l'), KeyCode::KeyL)?;
    assert_eq!(app.text(FILTER)?, "");
    assert_eq!(app.rows()?.len(), 2);
    assert_eq!(app.name(STATUS)?, "2 bookmarks");

    // Focus in another field.
    app.type_into(FILTER, "crates")?;
    app.type_into(NAME, "Docs")?;
    app.driver().chord(KeyChord::primary('l'), KeyCode::KeyL)?;
    assert_eq!(app.text(FILTER)?, "");
    assert_eq!(app.text(NAME)?, "Docs");
    assert_eq!(app.rows()?.len(), 2);

    // Focus on a button.
    app.type_into(FILTER, "crates")?;
    let add = app.find(ADD)?;
    app.driver().runtime_mut().focus(&add)?;
    app.driver().chord(KeyChord::primary('l'), KeyCode::KeyL)?;
    assert_eq!(app.ui.focused(), Some(add));
    assert_eq!(app.text(FILTER)?, "");
    assert_eq!(app.rows()?.len(), 2);
    Ok(())
}
