//! Headless tests of the behaviors listed in the README. Each test mounts the
//! application with `bookmarks::build`, drives it through `testing::Driver`
//! (clicks, text, chords) and reads what it shows through semantics.
use pert_ui::prelude::*;
use pert_ui::testing::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Mounts the application on a fresh headless runtime.
fn start() -> Result<(Runtime, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 800.0, 1.0)?)?;
    let root = bookmarks::build(&mut ui)?;
    Ok((ui, root))
}

const NAME: &[&str] = &["content", "form", "name-field", "name"];
const URL: &[&str] = &["content", "form", "url-field", "url"];
const ADD: &[&str] = &["content", "add"];
const ERROR: &[&str] = &["content", "error"];
const FILTER: &[&str] = &["content", "filter-field", "filter"];
const STATUS: &[&str] = &["content", "status"];
const LIST: &[&str] = &["content", "list"];

/// The static text (semantic name) of the component at `path`; `None` when empty.
fn name_at(driver: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<Option<String>, TestError> {
    Ok(driver.semantics(&driver.find(root, path)?)?.name)
}

/// The text an editor at `path` holds, empty when it holds none.
fn text_at(driver: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<String, TestError> {
    Ok(match driver.semantics(&driver.find(root, path)?)?.value {
        Some(SemanticValue::Text(text)) => text.to_string(),
        None => String::new(),
        Some(other) => panic!("{path:?} is not a text editor: {other:?}"),
    })
}

/// One shown row: its key, the `name` and `url` labels and the Remove button's text.
#[derive(Debug, PartialEq)]
struct Shown {
    key: String,
    name: Option<String>,
    url: Option<String>,
    remove: Option<String>,
}

/// The rows of the list, in list order.
fn rows(driver: &Driver<'_>, root: &WidgetId) -> Result<Vec<Shown>, TestError> {
    let list = driver.find(root, LIST)?;
    driver
        .address_children(&list)?
        .into_iter()
        .map(|row| {
            Ok(Shown {
                name: name_at(driver, &row.id, &["name"])?,
                url: name_at(driver, &row.id, &["url"])?,
                remove: name_at(driver, &row.id, &["remove"])?,
                key: row.segment,
            })
        })
        .collect()
}

/// The (key, name, url) of every shown row.
fn entries(driver: &Driver<'_>, root: &WidgetId) -> Result<Vec<(String, String, String)>, TestError> {
    Ok(rows(driver, root)?
        .into_iter()
        .map(|row| (row.key, row.name.unwrap_or_default(), row.url.unwrap_or_default()))
        .collect())
}

fn entry(key: &str, name: &str, url: &str) -> (String, String, String) {
    (key.to_owned(), name.to_owned(), url.to_owned())
}

/// Types `name` and `url` into the form and presses Add.
fn add(driver: &mut Driver<'_>, root: &WidgetId, name: &str, url: &str) -> Result<(), TestError> {
    let name_field = driver.find(root, NAME)?;
    driver.replace_text(&name_field, name)?;
    let url_field = driver.find(root, URL)?;
    driver.replace_text(&url_field, url)?;
    let button = driver.find(root, ADD)?;
    driver.click(&button)
}

/// Types `text` into the filter field.
fn filter(driver: &mut Driver<'_>, root: &WidgetId, text: &str) -> Result<(), TestError> {
    let field = driver.find(root, FILTER)?;
    driver.replace_text(&field, text)
}

#[test]
fn add_appends_a_trimmed_bookmark_at_the_end() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(
        entries(&driver, &root)?,
        [
            entry("1", "Rust", "https://www.rust-lang.org"),
            entry("2", "Crates", "https://crates.io"),
        ]
    );

    add(&mut driver, &root, "  Docs  ", "https://docs.rs")?;
    add(&mut driver, &root, "Example", "http://example.com")?;
    assert_eq!(
        entries(&driver, &root)?,
        [
            entry("1", "Rust", "https://www.rust-lang.org"),
            entry("2", "Crates", "https://crates.io"),
            entry("3", "Docs", "https://docs.rs"),
            entry("4", "Example", "http://example.com"),
        ]
    );
    Ok(())
}

#[test]
fn add_refuses_a_blank_name_or_a_bad_url_and_keeps_the_fields() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(name_at(&driver, &root, ERROR)?, None);
    let initial = entries(&driver, &root)?;

    // Empty name; the name is checked before the (also bad) URL.
    add(&mut driver, &root, "", "ftp://example.com")?;
    assert_eq!(name_at(&driver, &root, ERROR)?.as_deref(), Some("Enter a name."));
    assert_eq!(entries(&driver, &root)?, initial);
    assert_eq!(text_at(&driver, &root, NAME)?, "");
    assert_eq!(text_at(&driver, &root, URL)?, "ftp://example.com");

    // A name of spaces only.
    add(&mut driver, &root, "   ", "https://docs.rs")?;
    assert_eq!(name_at(&driver, &root, ERROR)?.as_deref(), Some("Enter a name."));
    assert_eq!(entries(&driver, &root)?, initial);
    assert_eq!(text_at(&driver, &root, NAME)?, "   ");
    assert_eq!(text_at(&driver, &root, URL)?, "https://docs.rs");

    // A URL without http:// or https://.
    for url in ["docs.rs", "ftp://docs.rs", "", " https://docs.rs"] {
        add(&mut driver, &root, "Docs", url)?;
        assert_eq!(
            name_at(&driver, &root, ERROR)?.as_deref(),
            Some("The URL must start with http:// or https://."),
            "url {url:?}"
        );
        assert_eq!(entries(&driver, &root)?, initial);
        assert_eq!(text_at(&driver, &root, NAME)?, "Docs");
        assert_eq!(text_at(&driver, &root, URL)?, url);
    }

    // The next successful add clears the message.
    add(&mut driver, &root, "Docs", "https://docs.rs")?;
    assert_eq!(name_at(&driver, &root, ERROR)?, None);
    assert_eq!(entries(&driver, &root)?.len(), 3);
    Ok(())
}

#[test]
fn a_successful_add_clears_both_fields() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    add(&mut driver, &root, "Docs", "https://docs.rs")?;
    assert_eq!(entries(&driver, &root)?.len(), 3);
    assert_eq!(text_at(&driver, &root, NAME)?, "");
    assert_eq!(text_at(&driver, &root, URL)?, "");
    Ok(())
}

#[test]
fn each_row_shows_name_url_and_remove_which_deletes_only_that_bookmark() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    add(&mut driver, &root, "Docs", "https://docs.rs")?;
    // A duplicate name: rows are told apart by id.
    add(&mut driver, &root, "Rust", "https://doc.rust-lang.org")?;
    let shown = rows(&driver, &root)?;
    assert_eq!(shown.len(), 4);
    for row in &shown {
        assert_eq!(row.remove.as_deref(), Some("Remove"), "row {}", row.key);
        assert!(row.name.is_some() && row.url.is_some(), "row {row:?}");
    }

    let remove = driver.find(&root, &["content", "list", "2", "remove"])?;
    driver.click(&remove)?;
    assert_eq!(
        entries(&driver, &root)?,
        [
            entry("1", "Rust", "https://www.rust-lang.org"),
            entry("3", "Docs", "https://docs.rs"),
            entry("4", "Rust", "https://doc.rust-lang.org"),
        ]
    );

    let remove = driver.find(&root, &["content", "list", "4", "remove"])?;
    driver.click(&remove)?;
    assert_eq!(
        entries(&driver, &root)?,
        [
            entry("1", "Rust", "https://www.rust-lang.org"),
            entry("3", "Docs", "https://docs.rs"),
        ]
    );
    Ok(())
}

#[test]
fn filter_hides_names_that_do_not_contain_it_ignoring_case_and_spaces() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    add(&mut driver, &root, "Rust Book", "https://doc.rust-lang.org/book")?;
    let keys = |driver: &Driver<'_>| -> Result<Vec<String>, TestError> {
        Ok(rows(driver, &root)?.into_iter().map(|row| row.key).collect())
    };
    assert_eq!(keys(&driver)?, ["1", "2", "3"]);

    filter(&mut driver, &root, "rust")?;
    assert_eq!(keys(&driver)?, ["1", "3"]);
    filter(&mut driver, &root, "  RUST  ")?;
    assert_eq!(keys(&driver)?, ["1", "3"]);
    filter(&mut driver, &root, "aTe")?;
    assert_eq!(keys(&driver)?, ["2"]);
    // Only the name counts, not the URL.
    filter(&mut driver, &root, "crates.io")?;
    assert_eq!(keys(&driver)?, Vec::<String>::new());
    filter(&mut driver, &root, "   ")?;
    assert_eq!(keys(&driver)?, ["1", "2", "3"]);
    filter(&mut driver, &root, "")?;
    assert_eq!(keys(&driver)?, ["1", "2", "3"]);
    Ok(())
}

#[test]
fn status_counts_all_or_shown_of_total() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let status = |driver: &Driver<'_>| name_at(driver, &root, STATUS);
    assert_eq!(status(&driver)?.as_deref(), Some("2 bookmarks"));

    add(&mut driver, &root, "Docs", "https://docs.rs")?;
    assert_eq!(status(&driver)?.as_deref(), Some("3 bookmarks"));

    filter(&mut driver, &root, "rust")?;
    assert_eq!(status(&driver)?.as_deref(), Some("1 of 3 bookmarks"));
    filter(&mut driver, &root, "nothing")?;
    assert_eq!(status(&driver)?.as_deref(), Some("0 of 3 bookmarks"));
    filter(&mut driver, &root, "s")?;
    assert_eq!(status(&driver)?.as_deref(), Some("3 of 3 bookmarks"));
    filter(&mut driver, &root, "  ")?;
    assert_eq!(status(&driver)?.as_deref(), Some("3 bookmarks"));

    // "N bookmarks" also for one.
    for key in ["1", "2"] {
        let remove = driver.find(&root, &["content", "list", key, "remove"])?;
        driver.click(&remove)?;
    }
    assert_eq!(status(&driver)?.as_deref(), Some("1 bookmarks"));
    Ok(())
}

#[test]
fn ctrl_l_clears_the_filter_wherever_focus_is() -> TestResult {
    let (mut ui, root) = start()?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    // Ctrl+L, or Cmd+L on macOS: `primary` picks the platform's modifier.
    let clear = |driver: &mut Driver<'_>| driver.chord(KeyChord::primary('l'), KeyCode::KeyL);

    // Focus in the filter field itself.
    filter(&mut driver, &root, "rust")?;
    assert_eq!(rows(&driver, &root)?.len(), 1);
    clear(&mut driver)?;
    assert_eq!(text_at(&driver, &root, FILTER)?, "");
    assert_eq!(rows(&driver, &root)?.len(), 2);
    assert_eq!(name_at(&driver, &root, STATUS)?.as_deref(), Some("2 bookmarks"));

    // Focus in the name field.
    filter(&mut driver, &root, "crates")?;
    let name_field = driver.find(&root, NAME)?;
    driver.replace_text(&name_field, "Docs")?;
    assert_eq!(driver.runtime().focused(), Some(name_field));
    clear(&mut driver)?;
    assert_eq!(text_at(&driver, &root, FILTER)?, "");
    assert_eq!(text_at(&driver, &root, NAME)?, "Docs");

    // Nothing focused.
    filter(&mut driver, &root, "crates")?;
    driver.clear_focus()?;
    assert_eq!(driver.runtime().focused(), None);
    clear(&mut driver)?;
    assert_eq!(text_at(&driver, &root, FILTER)?, "");
    assert_eq!(rows(&driver, &root)?.len(), 2);
    Ok(())
}
