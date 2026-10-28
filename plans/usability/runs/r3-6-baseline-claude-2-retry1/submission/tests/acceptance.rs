//! Headless acceptance checks: each drives the editor through `Driver`
//! (clicks, keys, chords, typing) and reads it back through inspection.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::fs;
use std::path::PathBuf;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A notes folder of the test's own: three notes and one other file.
fn folder(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("notes-{test}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("beta.txt"), "second note\nwith two lines").unwrap();
    fs::write(dir.join("alpha.txt"), "hello world").unwrap();
    fs::write(dir.join("gamma.txt"), "").unwrap();
    fs::write(dir.join("readme.md"), "not a note").unwrap();
    fs::create_dir_all(dir.join("folder.txt")).unwrap();
    dir
}

struct Editor {
    ui: Runtime,
    root: WidgetId,
    dir: PathBuf,
}

impl Editor {
    fn start(test: &str) -> Result<Editor> {
        let dir = folder(test);
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 600.0, 1.0)?)?;
        let root = notes::build(&mut ui, dir.clone())?;
        Driver::new(&mut ui).settle()?;
        Ok(Editor { ui, root, dir })
    }

    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }

    fn widget(&self, key: &str) -> Result<WidgetId> {
        Ok(self.ui.find_descendant(&self.root, key)?.ok_or(format!("no {key}"))?)
    }

    fn widget_in(&self, parent: &str, key: &str) -> Result<WidgetId> {
        let parent = self.widget(parent)?;
        Ok(self.ui.find_descendant(&parent, key)?.ok_or(format!("no {key} in {parent:?}"))?)
    }

    /// The note names the sidebar lists, in display order.
    fn listed(&self) -> Result<Vec<String>> {
        let list = self.widget("notes")?;
        Ok(self.ui.children(&list)?.iter()
            .map(|row| self.ui.key(row).map(str::to_owned))
            .collect::<std::result::Result<_, _>>()?)
    }

    /// The tab titles and the selected title.
    fn tabs(&mut self) -> Result<(Vec<String>, Option<String>)> {
        let tabs = self.widget("tabs")?;
        let semantics = self.driver().semantics(&tabs)?;
        let titles = semantics.items.iter().filter_map(|item| item.name.clone()).collect();
        let selected = semantics.items.iter().find(|item| item.selected).and_then(|item| item.name.clone());
        Ok((titles, selected))
    }

    fn status(&mut self) -> Result<String> {
        let status = self.widget("status")?;
        let line = self.ui.children(&status)?.into_iter().next().ok_or("no status label")?;
        Ok(self.driver().semantics(&line)?.name.unwrap_or_default())
    }

    /// The names of every visible component, such as a notification's text.
    fn visible_text(&self) -> Vec<String> {
        self.ui.inspect().components.into_iter()
            .filter(|c| c.visible)
            .filter_map(|c| c.details.semantics.and_then(|s| s.name))
            .collect()
    }

    /// How many notifications the toast host shows. Its messages are not
    /// exposed to inspection or content search, only their count.
    fn notifications(&mut self) -> Result<f64> {
        let host = self.widget("toasts")?;
        match self.driver().semantics(&host)?.value {
            Some(SemanticValue::Number { value, .. }) => Ok(value),
            other => Err(format!("toast host value {other:?}").into()),
        }
    }

    fn is_visible(&self, key: &str) -> bool {
        self.ui.inspect().components.iter().any(|c| c.key == key && c.visible)
    }

    fn editor(&self, note: &str) -> Result<WidgetId> {
        let tabs = self.widget("tabs")?;
        Ok(self.ui.find(&tabs, &[note, "editor"])?.ok_or(format!("no editor for {note}"))?)
    }

    fn editor_text(&mut self, note: &str) -> Result<String> {
        let editor = self.editor(note)?;
        match self.driver().semantics(&editor)?.value {
            Some(SemanticValue::Text(text)) => Ok(text),
            other => Err(format!("editor value {other:?}").into()),
        }
    }

    fn file(&self, note: &str) -> String {
        fs::read_to_string(self.dir.join(note)).unwrap_or_else(|e| format!("<{e}>"))
    }

    fn click_row(&mut self, note: &str) -> Result {
        let row = self.widget_in("notes", note)?;
        self.driver().click(&row)?;
        Ok(())
    }

    /// Chooses File > `item` from the menu bar: F10 focuses the File title,
    /// Down opens its menu on the first row, Down moves, Enter chooses.
    fn menu(&mut self, item: &str) -> Result {
        let index = ["new", "save", "close"].iter().position(|i| *i == item).ok_or("no such row")?;
        self.driver().key(KeyCode::F10)?;
        self.driver().key(KeyCode::ArrowDown)?;
        let bar = self.widget("bar")?;
        let file = self.ui.find_descendant(&bar, "title/file")?.ok_or("no File title")?;
        let items = self.ui.find_descendant(&file, "items")?.ok_or("menu not open")?;
        let rows = self.driver().semantics(&items)?.items;
        assert_eq!(rows.iter().map(|r| r.name.clone().unwrap_or_default()).collect::<Vec<_>>(),
            ["New note", "Save", "Close tab"]);
        for _ in 0..index {
            self.driver().key(KeyCode::ArrowDown)?;
        }
        self.driver().key(KeyCode::Enter)?;
        Ok(())
    }

    fn chord(&mut self, c: char, key: KeyCode) -> Result {
        self.driver().chord(KeyChord::primary(c), key)?;
        Ok(())
    }

    fn click(&mut self, parent: &str, key: &str) -> Result {
        let target = self.widget_in(parent, key)?;
        self.driver().click(&target)?;
        Ok(())
    }
}

#[test]
fn lists_notes_and_opens_tabs() -> Result {
    let mut app = Editor::start("open")?;
    assert_eq!(app.listed()?, ["alpha.txt", "beta.txt", "gamma.txt"]);
    assert_eq!(app.status()?, "");
    assert_eq!(app.tabs()?.0, Vec::<String>::new());

    // Pointer: a click on a row opens it.
    app.click_row("alpha.txt")?;
    assert_eq!(app.tabs()?, (vec!["alpha.txt".into()], Some("alpha.txt".into())));
    assert_eq!(app.editor_text("alpha.txt")?, "hello world");

    // Keyboard: focus the list, Down to the next row, Enter opens it.
    let list = app.widget("notes")?;
    app.driver().runtime_mut().focus(&list)?;
    app.driver().key(KeyCode::ArrowDown)?;
    app.driver().key(KeyCode::Enter)?;
    assert_eq!(app.tabs()?, (vec!["alpha.txt".into(), "beta.txt".into()], Some("beta.txt".into())));
    assert_eq!(app.status()?, "beta.txt: 2 lines, 5 words");

    // Reopening switches to the open tab instead of adding one.
    app.click_row("alpha.txt")?;
    assert_eq!(app.tabs()?, (vec!["alpha.txt".into(), "beta.txt".into()], Some("alpha.txt".into())));
    assert_eq!(app.status()?, "alpha.txt: 1 lines, 2 words");
    app.driver().runtime_mut().focus(&list)?;
    app.driver().key(KeyCode::ArrowDown)?;
    app.driver().key(KeyCode::Enter)?;
    assert_eq!(app.tabs()?, (vec!["alpha.txt".into(), "beta.txt".into()], Some("beta.txt".into())));
    Ok(())
}

#[test]
fn typing_marks_and_saving_clears() -> Result {
    let mut app = Editor::start("save")?;
    app.click_row("alpha.txt")?;
    // The editor has focus and its caret at the start.
    app.driver().type_text("Dear ")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt *"));
    assert_eq!(app.status()?, "alpha.txt: 1 lines, 3 words");
    app.driver().key(KeyCode::Enter)?;
    assert_eq!(app.status()?, "alpha.txt: 2 lines, 3 words");
    assert_eq!(app.file("alpha.txt"), "hello world");

    // Ctrl+S saves.
    app.chord('s', KeyCode::KeyS)?;
    assert_eq!(app.file("alpha.txt"), "Dear \nhello world");
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt"));

    // File > Save saves too.
    let editor = app.editor("alpha.txt")?;
    app.driver().replace_text(&editor, "replaced text")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt *"));
    assert_eq!(app.status()?, "alpha.txt: 1 lines, 2 words");
    app.menu("save")?;
    assert_eq!(app.file("alpha.txt"), "replaced text");
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt"));

    // A failed save is reported and leaves the tab modified.
    let editor = app.editor("alpha.txt")?;
    app.driver().replace_text(&editor, "lost")?;
    assert_eq!(app.notifications()?, 0.0);
    fs::remove_dir_all(&app.dir)?;
    app.chord('s', KeyCode::KeyS)?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt *"));
    assert_eq!(app.notifications()?, 1.0);
    // The same through the menu.
    app.menu("save")?;
    assert_eq!(app.tabs()?.1.as_deref(), Some("alpha.txt *"));
    assert_eq!(app.notifications()?, 2.0);
    Ok(())
}

#[test]
fn new_note_dialog_refuses_and_creates() -> Result {
    let mut app = Editor::start("new")?;
    let before = fs::read_dir(&app.dir)?.count();
    app.chord('n', KeyCode::KeyN)?;
    assert!(app.is_visible("new-form"));
    let field = app.widget_in("new-form", "name")?;

    for (name, reason) in [
        ("", "A name is required."),
        ("a/b", "A name must not contain / or \\."),
        ("alpha", "A note named alpha.txt already exists."),
        ("alpha.txt", "A note named alpha.txt already exists."),
        (".hidden", "A name must not start with a dot."),
        ("back\\slash", "A name must not contain / or \\."),
        (&"x".repeat(65), "A name has at most 64 characters."),
    ] {
        app.driver().replace_text(&field, name)?;
        app.click("new-form", "create")?;
        assert!(app.is_visible("new-form"), "dialog closed for {name:?}");
        assert!(app.visible_text().iter().any(|t| t == reason), "{name:?}: {:?}", app.visible_text());
        assert_eq!(fs::read_dir(&app.dir)?.count(), before, "a file was created for {name:?}");
    }
    assert_eq!(app.listed()?, ["alpha.txt", "beta.txt", "gamma.txt"]);

    // A valid name, submitted with Enter, creates, lists and opens the note.
    app.driver().replace_text(&field, "delta")?;
    app.driver().key(KeyCode::Enter)?;
    assert!(!app.is_visible("new-form"));
    assert_eq!(app.file("delta.txt"), "");
    assert_eq!(app.listed()?, ["alpha.txt", "beta.txt", "delta.txt", "gamma.txt"]);
    assert_eq!(app.tabs()?, (vec!["delta.txt".into()], Some("delta.txt".into())));
    assert_eq!(app.status()?, "delta.txt: 1 lines, 0 words");

    // File > New note opens the same dialog, empty, and Create works by pointer.
    app.menu("new")?;
    assert!(app.is_visible("new-form"));
    assert!(!app.visible_text().iter().any(|t| t.starts_with("A note named")));
    let field = app.widget_in("new-form", "name")?;
    app.driver().replace_text(&field, "epsilon.txt")?;
    app.click("new-form", "create")?;
    assert!(!app.is_visible("new-form"));
    assert_eq!(app.file("epsilon.txt"), "");
    assert_eq!(app.tabs()?, (vec!["delta.txt".into(), "epsilon.txt".into()], Some("epsilon.txt".into())));
    Ok(())
}

#[test]
fn closing_tabs() -> Result {
    let mut app = Editor::start("close")?;

    // An unmodified tab closes at once.
    app.click_row("gamma.txt")?;
    app.chord('w', KeyCode::KeyW)?;
    assert!(!app.is_visible("close-form"));
    assert_eq!(app.tabs()?.0, Vec::<String>::new());
    assert_eq!(app.status()?, "");

    // Cancel keeps the tab and its text.
    app.click_row("alpha.txt")?;
    app.driver().type_text("x")?;
    app.chord('w', KeyCode::KeyW)?;
    assert!(app.is_visible("close-form"));
    app.click("close-form", "cancel")?;
    assert!(!app.is_visible("close-form"));
    assert_eq!(app.tabs()?, (vec!["alpha.txt *".into()], Some("alpha.txt *".into())));
    assert_eq!(app.editor_text("alpha.txt")?, "xhello world");
    assert_eq!(app.file("alpha.txt"), "hello world");

    // Discard closes without writing.
    app.chord('w', KeyCode::KeyW)?;
    assert!(app.is_visible("close-form"));
    app.click("close-form", "discard")?;
    assert!(!app.is_visible("close-form"));
    assert_eq!(app.tabs()?.0, Vec::<String>::new());
    assert_eq!(app.file("alpha.txt"), "hello world");

    // Reopened, the note shows the file's text again; Save writes, then closes.
    app.click_row("alpha.txt")?;
    assert_eq!(app.editor_text("alpha.txt")?, "hello world");
    app.click_row("beta.txt")?;
    app.driver().type_text("y")?;
    app.chord('w', KeyCode::KeyW)?;
    app.click("close-form", "save")?;
    assert!(!app.is_visible("close-form"));
    assert_eq!(app.tabs()?, (vec!["alpha.txt".into()], Some("alpha.txt".into())));
    assert_eq!(app.file("beta.txt"), "ysecond note\nwith two lines");

    // Closing moved focus to the remaining tab's editor.
    assert_eq!(app.ui.focused(), Some(app.editor("alpha.txt")?));

    // A failed save in the dialog keeps the tab open and modified.
    app.driver().type_text("z")?;
    fs::remove_dir_all(&app.dir)?;
    app.chord('w', KeyCode::KeyW)?;
    app.click("close-form", "save")?;
    assert_eq!(app.tabs()?, (vec!["alpha.txt *".into()], Some("alpha.txt *".into())));
    assert_eq!(app.notifications()?, 1.0);
    Ok(())
}

