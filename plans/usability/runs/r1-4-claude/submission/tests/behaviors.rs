//! One test per behavior listed in the README, driven headlessly through
//! pointer, text and key input and checked through inspection.
mod support;

use pert_ui::{KeyCode, Modifiers};
use support::*;

fn initial() -> Vec<(String, String)> {
    vec![
        pair("Rust", "https://www.rust-lang.org"),
        pair("Crates", "https://crates.io"),
    ]
}

/// 1. Add appends at the end, with the name trimmed.
#[test]
fn add_appends_trimmed_bookmark() -> TestResult {
    let mut app = Bookmarks::open()?;
    assert_eq!(app.shown(), initial());

    app.add("  Docs  ", "https://docs.rs");
    app.add("Example", "http://example.com");

    let mut expected = initial();
    expected.push(pair("Docs", "https://docs.rs"));
    expected.push(pair("Example", "http://example.com"));
    assert_eq!(app.shown(), expected);
    assert_eq!(app.rows(), ["1", "2", "3", "4"]);
    assert_eq!(app.label(ERROR), "");
    Ok(())
}

/// 2. Add refuses a blank name and a URL without http(s)://, shows the
/// message (name first), keeps both fields, and the next add clears it.
#[test]
fn refused_add_shows_message_and_keeps_fields() -> TestResult {
    let mut app = Bookmarks::open()?;

    // Empty name, with an invalid URL too: the name is checked first.
    app.type_into(URL, "ftp://example.com");
    app.click(ADD);
    assert_eq!(app.label(ERROR), "Enter a name.");
    assert_eq!(app.shown(), initial());
    assert_eq!(app.field(NAME), "");
    assert_eq!(app.field(URL), "ftp://example.com");

    // A name of spaces only.
    app.type_into(NAME, "   ");
    app.click(ADD);
    assert_eq!(app.label(ERROR), "Enter a name.");
    assert_eq!(app.shown(), initial());
    assert_eq!(app.field(NAME), "   ");

    // A valid name with a URL lacking the scheme.
    for url in ["ftp://example.com", "example.com", "https:/example.com", ""] {
        app.type_into(NAME, "Example");
        app.type_into(URL, url);
        app.click(ADD);
        assert_eq!(
            app.label(ERROR),
            "The URL must start with http:// or https://.",
            "url {url:?}"
        );
        assert_eq!(app.shown(), initial());
        assert_eq!(app.field(NAME), "Example");
        assert_eq!(app.field(URL), url);
    }

    // The next successful add clears the message.
    app.type_into(URL, "https://example.com");
    app.click(ADD);
    assert_eq!(app.label(ERROR), "");
    assert_eq!(app.shown().len(), 3);
    Ok(())
}

/// 3. After a successful add, both fields are empty.
#[test]
fn successful_add_clears_fields() -> TestResult {
    let mut app = Bookmarks::open()?;
    app.type_into(NAME, "Docs");
    app.type_into(URL, "https://docs.rs");
    assert_eq!(app.field(NAME), "Docs");
    assert_eq!(app.field(URL), "https://docs.rs");

    app.click(ADD);
    assert_eq!(app.field(NAME), "");
    assert_eq!(app.field(URL), "");
    Ok(())
}

/// 4. Each bookmark shows its name, URL and a Remove button; Remove deletes
/// that bookmark only.
#[test]
fn list_rows_and_remove() -> TestResult {
    let mut app = Bookmarks::open()?;
    app.add("Docs", "https://docs.rs");
    // Two bookmarks with the same name stay distinct.
    app.add("Rust", "https://doc.rust-lang.org");
    assert_eq!(app.rows(), ["1", "2", "3", "4"]);
    for row in app.rows() {
        let remove = app.semantics(&["content", "list", "items", &row, "remove"]);
        assert_eq!(remove.name.as_deref(), Some("Remove"));
        assert_eq!(remove.role, pert_ui::SemanticRole::Button);
    }

    app.click(&["content", "list", "items", "1", "remove"]);
    assert_eq!(app.rows(), ["2", "3", "4"]);
    assert_eq!(
        app.shown(),
        [
            pair("Crates", "https://crates.io"),
            pair("Docs", "https://docs.rs"),
            pair("Rust", "https://doc.rust-lang.org"),
        ]
    );

    app.click(&["content", "list", "items", "3", "remove"]);
    assert_eq!(
        app.shown(),
        [
            pair("Crates", "https://crates.io"),
            pair("Rust", "https://doc.rust-lang.org"),
        ]
    );
    Ok(())
}

/// 5. The filter hides names that do not contain it, ignoring case and
/// surrounding spaces; an empty filter shows everything.
#[test]
fn filter_by_name() -> TestResult {
    let mut app = Bookmarks::open()?;
    app.add("Rustonomicon", "https://doc.rust-lang.org/nomicon");

    app.type_into(FILTER, "rust");
    assert_eq!(
        app.shown(),
        [
            pair("Rust", "https://www.rust-lang.org"),
            pair("Rustonomicon", "https://doc.rust-lang.org/nomicon"),
        ]
    );

    app.type_into(FILTER, "  CRATES ");
    assert_eq!(app.shown(), [pair("Crates", "https://crates.io")]);

    // Matches inside the name, not only at its start.
    app.type_into(FILTER, "nomi");
    assert_eq!(app.rows(), ["3"]);

    app.type_into(FILTER, "nothing");
    assert!(app.rows().is_empty());

    app.type_into(FILTER, "   ");
    assert_eq!(app.rows(), ["1", "2", "3"]);
    app.type_into(FILTER, "");
    assert_eq!(app.rows(), ["1", "2", "3"]);

    // A bookmark added while filtering shows only if it matches.
    app.type_into(FILTER, "rust");
    app.add("Lib.rs", "https://lib.rs");
    assert_eq!(app.rows(), ["1", "3"]);
    Ok(())
}

/// 6. Status reads "N bookmarks" unfiltered, "N of M bookmarks" filtered.
#[test]
fn status_counts() -> TestResult {
    let mut app = Bookmarks::open()?;
    assert_eq!(app.label(STATUS), "2 bookmarks");

    app.click(&["content", "list", "items", "1", "remove"]);
    assert_eq!(app.label(STATUS), "1 bookmarks");

    app.add("Docs", "https://docs.rs");
    app.add("Rust", "https://www.rust-lang.org");
    assert_eq!(app.label(STATUS), "3 bookmarks");

    app.type_into(FILTER, "RUST");
    assert_eq!(app.label(STATUS), "1 of 3 bookmarks");
    app.type_into(FILTER, "s");
    assert_eq!(app.label(STATUS), "3 of 3 bookmarks");
    app.type_into(FILTER, "zzz");
    assert_eq!(app.label(STATUS), "0 of 3 bookmarks");
    app.type_into(FILTER, "  ");
    assert_eq!(app.label(STATUS), "3 bookmarks");
    Ok(())
}

/// 7. Ctrl+L (Cmd+L on macOS) empties the filter wherever focus is.
#[test]
fn clear_filter_shortcut() -> TestResult {
    let mut app = Bookmarks::open()?;

    // Focus in the filter field itself.
    app.type_into(FILTER, "rust");
    assert_eq!(app.rows(), ["1"]);
    app.chord(Modifiers::PRIMARY, KeyCode::KeyL, 'l');
    assert_eq!(app.field(FILTER), "");
    assert_eq!(app.rows(), ["1", "2"]);
    assert_eq!(app.label(STATUS), "2 bookmarks");

    // Focus in another text field, which keeps its own text.
    app.type_into(FILTER, "crates");
    app.type_into(NAME, "Docs");
    app.chord(Modifiers::PRIMARY, KeyCode::KeyL, 'l');
    assert_eq!(app.field(FILTER), "");
    assert_eq!(app.field(NAME), "Docs");

    // Focus on a button.
    app.type_into(FILTER, "crates");
    let add = app.id(ADD);
    app.ui.focus(&add)?;
    app.chord(Modifiers::PRIMARY, KeyCode::KeyL, 'l');
    assert_eq!(app.field(FILTER), "");

    // Nothing focused.
    app.type_into(FILTER, "crates");
    app.ui.clear_focus();
    assert!(app.ui.focused().is_none());
    app.chord(Modifiers::PRIMARY, KeyCode::KeyL, 'l');
    assert_eq!(app.field(FILTER), "");
    Ok(())
}
