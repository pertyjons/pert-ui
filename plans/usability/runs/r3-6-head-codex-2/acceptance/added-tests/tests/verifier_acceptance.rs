//! Independent acceptance checks for task 6 (notes editor), added by the
//! acceptance verifier in the acceptance copy only. They start the
//! submission's real application through `App::headless`, drive it with the
//! library's `Driver` and `Runtime` input methods (pointer, typed text, named
//! keys, chords) and read results from the library's inspection snapshot,
//! semantics and the files on disk. No application function other than the
//! public `app` constructor is called.
use notes_editor::app;
use pert_ui::{prelude::*, testing::*};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const FILES: &[&str] = &["close-dialog", "bar", "body", "split", "files"];
const TABS: &[&str] = &["close-dialog", "bar", "body", "split", "tabs"];
const NOTIFICATION: &[&str] = &["close-dialog", "bar", "body", "notification"];
const NEW_FORM: &[&str] = &["dialog", "new-form"];
const CLOSE_FORM: &[&str] = &["close-dialog", "dialog", "close-form"];

/// A fresh notes folder of the verifier's own, below the copy's target.
fn folder(name: &str) -> Result<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/verifier-notes")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn dir_names(dir: &Path) -> Result<BTreeSet<String>> {
    Ok(fs::read_dir(dir)?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<_>>()?)
}

fn ctrl(c: char) -> KeyChord {
    KeyChord::new(
        CommandKey::Character(c),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    )
}

fn press_ctrl(d: &mut Driver<'_>, c: char) -> Result {
    let code = match c {
        'n' => KeyCode::KeyN,
        's' => KeyCode::KeyS,
        'w' => KeyCode::KeyW,
        'a' => KeyCode::KeyA,
        _ => unreachable!(),
    };
    d.chord(ctrl(c), code)?;
    d.settle()?;
    Ok(())
}

/// The inspection record of `id` in a fresh snapshot.
fn record(d: &Driver<'_>, id: &WidgetId) -> Result<ComponentInspection> {
    let iid = d.runtime().inspection_id(id)?;
    Ok(d
        .runtime()
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == iid)
        .ok_or("no record")?)
}

fn prop(c: &ComponentInspection, name: &str) -> Option<InspectValue> {
    c.details.properties.get(name).cloned()
}

/// Whether the library Dialog at `id` reports itself open in the snapshot.
fn dialog_open(d: &Driver<'_>, id: &WidgetId) -> Result<bool> {
    let rec = record(d, id)?;
    let sem = rec.details.semantics.as_ref().ok_or("no semantics")?;
    assert!(matches!(sem.role, SemanticRole::Dialog));
    Ok(prop(&rec, "open") == Some(InspectValue::Bool(true)))
}

/// Every visible component in the snapshot whose address lies below `prefix`.
fn visible_below(d: &Driver<'_>, prefix: &[&str]) -> Vec<ComponentInspection> {
    d.runtime()
        .inspect()
        .components
        .into_iter()
        .filter(|c| {
            c.visible
                && c.path.as_ref().is_some_and(|p| {
                    p.len() > prefix.len() + 1
                        && p[1..=prefix.len()].iter().zip(prefix).all(|(a, b)| a == b)
                })
        })
        .collect()
}

struct Ui {
    root: WidgetId,
}

impl Ui {
    fn at(&self, d: &Driver<'_>, path: &[&str]) -> Result<WidgetId> {
        Ok(d.find(&self.root, path)?)
    }
    /// The listed note names, read from the list's rows and their buttons.
    fn listed(&self, d: &Driver<'_>) -> Result<Vec<String>> {
        let list = self.at(d, FILES)?;
        let mut names = Vec::new();
        for row in d.address_children(&list)? {
            let button = d.find(&row.id, &["open"])?;
            let name = d.semantics(&button)?.name.unwrap_or_default();
            assert_eq!(name, row.segment, "row key and shown name agree");
            names.push(name);
        }
        Ok(names)
    }
    fn titles(&self, d: &Driver<'_>) -> Result<Vec<String>> {
        let tabs = self.at(d, TABS)?;
        Ok(d.semantics(&tabs)?
            .items
            .into_iter()
            .map(|i| i.name.unwrap_or_default())
            .collect())
    }
    /// The active tab's title, from the tab list's semantic value.
    fn active(&self, d: &Driver<'_>) -> Result<Option<String>> {
        let tabs = self.at(d, TABS)?;
        match d.semantics(&tabs)?.value {
            Some(SemanticValue::Choice { label, .. }) => Ok(label),
            other => Err(format!("tab list value {other:?}").into()),
        }
    }
    fn tab_count(&self, d: &Driver<'_>) -> Result<usize> {
        let tabs = self.at(d, TABS)?;
        match prop(&record(d, &tabs)?, "tab_count") {
            Some(InspectValue::Number(n)) => Ok(n as usize),
            other => Err(format!("tab_count {other:?}").into()),
        }
    }
    fn editor(&self, d: &Driver<'_>, note: &str) -> Result<WidgetId> {
        let tabs = self.at(d, TABS)?;
        Ok(d.find(&tabs, &[note, "page", "editor"])?)
    }
    fn editor_text(&self, d: &Driver<'_>, note: &str) -> Result<String> {
        match d.semantics(&self.editor(d, note)?)?.value {
            Some(SemanticValue::Text(t)) => Ok(t),
            other => Err(format!("editor value {other:?}").into()),
        }
    }
    /// The texts of the visible status labels in the snapshot.
    fn visible_status(&self, d: &Driver<'_>) -> Vec<String> {
        visible_below(d, TABS)
            .into_iter()
            .filter(|c| c.key == "status")
            .filter_map(|c| c.details.semantics.and_then(|s| s.name))
            .collect()
    }
    fn notification(&self, d: &Driver<'_>) -> Result<(bool, String)> {
        let id = self.at(d, NOTIFICATION)?;
        let rec = record(d, &id)?;
        let text = rec
            .details
            .semantics
            .and_then(|s| s.name)
            .unwrap_or_default();
        Ok((rec.visible, text))
    }
    fn new_dialog_open(&self, d: &Driver<'_>) -> Result<bool> {
        dialog_open(d, &self.root)
    }
    fn close_dialog_open(&self, d: &Driver<'_>) -> Result<bool> {
        dialog_open(d, &self.at(d, &["close-dialog"])?)
    }
    /// No dialog open and no dialog form content visible.
    fn no_dialog(&self, d: &Driver<'_>) -> Result<bool> {
        Ok(!self.new_dialog_open(d)?
            && !self.close_dialog_open(d)?
            && visible_below(d, &["dialog"]).is_empty()
            && visible_below(d, &["close-dialog", "dialog"]).is_empty())
    }
    fn error_text(&self, d: &Driver<'_>) -> Result<(bool, String)> {
        let mut path = NEW_FORM.to_vec();
        path.push("error");
        let rec = record(d, &self.at(d, &path)?)?;
        Ok((
            rec.visible,
            rec.details
                .semantics
                .and_then(|s| s.name)
                .unwrap_or_default(),
        ))
    }
    fn click_path(&self, d: &mut Driver<'_>, path: &[&str]) -> Result {
        let id = self.at(d, path)?;
        d.click(&id)?;
        d.settle()?;
        Ok(())
    }
    fn click_close_choice(&self, d: &mut Driver<'_>, choice: &str) -> Result {
        let mut path = CLOSE_FORM.to_vec();
        path.extend(["actions", choice]);
        self.click_path(d, &path)
    }
    fn click_note(&self, d: &mut Driver<'_>, note: &str) -> Result {
        let mut path = FILES.to_vec();
        path.extend([note, "open"]);
        self.click_path(d, &path)
    }
    /// Clicks the editor of `note` (focusing it and placing the caret), moves
    /// the caret to the end with Ctrl+End and types `lines`, pressing Enter
    /// between them.
    fn type_lines(&self, d: &mut Driver<'_>, note: &str, lines: &[&str]) -> Result {
        let editor = self.editor(d, note)?;
        d.click(&editor)?;
        assert_eq!(d.runtime().focused(), Some(editor.clone()), "editor focused");
        d.chord(
            KeyChord::new(
                CommandKey::Named(KeyCode::End),
                Modifiers {
                    control: true,
                    ..Modifiers::default()
                },
            ),
            KeyCode::End,
        )?;
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                d.key(KeyCode::Enter)?;
            }
            d.type_text(line)?;
        }
        Ok(())
    }
    /// Opens File in the menu bar with a pointer click on its title, moves the
    /// pointer over the menu until the row named `row` is highlighted, and
    /// presses and releases the primary button there.
    fn menu(&self, d: &mut Driver<'_>, row: &str) -> Result {
        let bar = self.at(d, &["close-dialog", "bar"])?;
        let title = d
            .runtime()
            .children(&bar)?
            .into_iter()
            .find(|c| d.runtime().key(c).is_ok_and(|k| k == "title/file"))
            .ok_or("no File title")?;
        d.click(&title)?;
        let mut stack = vec![title.clone()];
        let mut items = None;
        while let Some(at) = stack.pop() {
            for c in d.runtime().children(&at)? {
                if d.runtime().key(&c)? == "items" {
                    items = Some(c.clone());
                }
                stack.push(c);
            }
        }
        let items = items.ok_or("no menu rows")?;
        assert!(record(d, &items)?.visible, "File menu opened by the click");
        let [x, y, w, h] = d.runtime().bounds(&items)?.ok_or("menu not laid out")?.values();
        let mut at_y = y + 1.0;
        while at_y < y + h {
            let at = Point::new(x + w / 2.0, at_y)?;
            d.runtime_mut()
                .pointer_input(PointerEvent::new(at, PointerKind::Move))?;
            d.settle()?;
            if let Some(SemanticValue::Choice {
                label: Some(label), ..
            }) = d.semantics(&items)?.value
                && label == row
            {
                d.runtime_mut().pointer_input(PointerEvent::new(
                    at,
                    PointerKind::Press(PointerButton::Primary),
                ))?;
                d.runtime_mut().pointer_input(PointerEvent::new(
                    at,
                    PointerKind::Release(PointerButton::Primary),
                ))?;
                d.settle()?;
                d.settle()?;
                assert!(!record(d, &items)?.visible, "menu closed after choosing");
                return Ok(());
            }
            at_y += 2.0;
        }
        Err(format!("menu row {row} never highlighted").into())
    }
}

fn start(dir: &Path) -> Result<HeadlessApp<notes_editor::Window>> {
    Ok(app(dir.to_path_buf()).headless()?)
}

// ---------------------------------------------------------------- item 1

#[test]
fn item1_listing_open_and_reopen() -> Result {
    let dir = folder("item1")?;
    // Created deliberately out of alphabetical order, with non-notes.
    for (name, text) in [
        ("zeta.txt", "last"),
        ("mango.txt", "middle one"),
        ("readme.md", "not a note"),
        ("apple.txt", "first\nnote"),
        ("data.txt.bak", "backup"),
        ("beta.txt", "second"),
    ] {
        fs::write(dir.join(name), text)?;
    }
    fs::create_dir(dir.join("folder.txt"))?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    assert_eq!(
        ui.listed(&d)?,
        ["apple.txt", "beta.txt", "mango.txt", "zeta.txt"]
    );
    let list = ui.at(&d, FILES)?;
    assert_eq!(
        prop(&record(&d, &list)?, "rows"),
        Some(InspectValue::Number(4.0))
    );
    assert_eq!(ui.tab_count(&d)?, 0);
    assert!(ui.visible_status(&d).is_empty(), "no status without a tab");

    // Pointer open.
    ui.click_note(&mut d, "mango.txt")?;
    assert_eq!(ui.titles(&d)?, ["mango.txt"]);
    assert_eq!(ui.active(&d)?.as_deref(), Some("mango.txt"));
    assert_eq!(ui.editor_text(&d, "mango.txt")?, "middle one");

    // Keyboard open: focus the list, Home, Down x3, Enter -> zeta.txt.
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::Home)?;
    for _ in 0..3 {
        d.key(KeyCode::ArrowDown)?;
    }
    d.key(KeyCode::Enter)?;
    d.settle()?;
    assert_eq!(ui.titles(&d)?, ["mango.txt", "zeta.txt"]);
    assert_eq!(ui.tab_count(&d)?, 2);
    assert_eq!(ui.active(&d)?.as_deref(), Some("zeta.txt"));
    assert_eq!(ui.editor_text(&d, "zeta.txt")?, "last");
    assert_eq!(ui.visible_status(&d), ["zeta.txt: 1 lines, 1 words"]);

    // Reopen mango by pointer: no new tab, its tab becomes active.
    ui.click_note(&mut d, "mango.txt")?;
    assert_eq!(ui.tab_count(&d)?, 2);
    assert_eq!(ui.titles(&d)?, ["mango.txt", "zeta.txt"]);
    assert_eq!(ui.active(&d)?.as_deref(), Some("mango.txt"));
    assert_eq!(ui.visible_status(&d), ["mango.txt: 1 lines, 2 words"]);

    // Reopen zeta from the keyboard: still two tabs, zeta active.
    d.runtime_mut().focus(&list)?;
    d.key(KeyCode::End)?;
    d.key(KeyCode::Enter)?;
    d.settle()?;
    assert_eq!(ui.tab_count(&d)?, 2);
    assert_eq!(ui.active(&d)?.as_deref(), Some("zeta.txt"));

    // A third note, apple, opens a third tab with its two-line text.
    ui.click_note(&mut d, "apple.txt")?;
    assert_eq!(ui.titles(&d)?, ["mango.txt", "zeta.txt", "apple.txt"]);
    assert_eq!(ui.visible_status(&d), ["apple.txt: 2 lines, 2 words"]);
    Ok(())
}

// ---------------------------------------------------------------- item 2

#[test]
fn item2_typing_marker_status_saves_and_failed_save() -> Result {
    let dir = folder("item2")?;
    fs::write(dir.join("journal.txt"), "hello world")?;
    fs::write(dir.join("other.txt"), "x")?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    ui.click_note(&mut d, "other.txt")?;
    ui.click_note(&mut d, "journal.txt")?;
    assert_eq!(ui.titles(&d)?, ["other.txt", "journal.txt"]);
    assert_eq!(ui.visible_status(&d), ["journal.txt: 1 lines, 2 words"]);

    // Typing with a line break, through the focused TextArea.
    ui.type_lines(&mut d, "journal.txt", &[" again", "second  line   here"])?;
    let typed = "hello world again\nsecond  line   here";
    assert_eq!(ui.editor_text(&d, "journal.txt")?, typed);
    let marked = ui.titles(&d)?;
    assert_eq!(marked[0], "other.txt", "untouched tab unmarked");
    assert_ne!(marked[1], "journal.txt", "modified tab title carries a marker");
    assert!(marked[1].starts_with("journal.txt"));
    let marked_title = marked[1].clone();
    assert_eq!(ui.visible_status(&d), ["journal.txt: 2 lines, 6 words"]);
    d.key(KeyCode::Enter)?;
    d.key(KeyCode::Enter)?;
    d.type_text("end")?;
    assert_eq!(ui.visible_status(&d), ["journal.txt: 4 lines, 7 words"]);
    let typed = format!("{typed}\n\nend");
    // Unsaved: the file is untouched.
    assert_eq!(fs::read_to_string(dir.join("journal.txt"))?, "hello world");

    // Ctrl+S with the editor focused.
    press_ctrl(&mut d, 's')?;
    assert_eq!(fs::read_to_string(dir.join("journal.txt"))?, typed);
    assert_eq!(ui.titles(&d)?, ["other.txt", "journal.txt"], "marker gone");
    assert_eq!(ui.editor_text(&d, "journal.txt")?, typed);

    // Typing back to the saved text clears the marker as well (text equals file).
    d.type_text("!")?;
    assert_eq!(ui.titles(&d)?[1], marked_title);
    d.key(KeyCode::Backspace)?;
    assert_eq!(ui.titles(&d)?[1], "journal.txt");

    // File -> Save with a pointer click in the menu bar.
    d.type_text(" via menu")?;
    assert_eq!(ui.titles(&d)?[1], marked_title);
    ui.menu(&mut d, "Save")?;
    let typed = format!("{typed} via menu");
    assert_eq!(fs::read_to_string(dir.join("journal.txt"))?, typed);
    assert_eq!(ui.titles(&d)?, ["other.txt", "journal.txt"]);
    // Save only wrote the active tab.
    assert_eq!(fs::read_to_string(dir.join("other.txt"))?, "x");
    let (_, before) = ui.notification(&d)?;
    assert!(before.is_empty(), "no notification before a failure");

    // Failure: remove the notes folder, then Ctrl+S.
    ui.type_lines(&mut d, "journal.txt", &["", "lost?"])?;
    fs::remove_dir_all(&dir)?;
    press_ctrl(&mut d, 's')?;
    let (visible, text) = ui.notification(&d)?;
    assert!(visible, "notification visible");
    assert!(
        text.contains("journal.txt") && text.to_lowercase().contains("fail"),
        "notification names the failure: {text}"
    );
    assert_eq!(ui.titles(&d)?[1], marked_title, "marker kept after failure");
    assert_eq!(ui.editor_text(&d, "journal.txt")?, format!("{typed}\nlost?"));
    assert!(!dir.exists());

    // The menu Save fails the same way.
    ui.menu(&mut d, "Save")?;
    assert_eq!(ui.titles(&d)?[1], marked_title);
    assert!(ui.notification(&d)?.1.contains("journal.txt"));
    println!("failure notification: {text}");
    Ok(())
}

// ---------------------------------------------------------------- item 3

#[test]
fn item3_new_note_dialog_refusals_and_creation() -> Result {
    let dir = folder("item3")?;
    fs::write(dir.join("existing.txt"), "keep")?;
    fs::write(dir.join("plain"), "no extension")?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    let before = dir_names(&dir)?;
    assert_eq!(ui.listed(&d)?, ["existing.txt"]);

    // Ctrl+N with the focus in an open note's editor.
    ui.click_note(&mut d, "existing.txt")?;
    let editor = ui.editor(&d, "existing.txt")?;
    d.click(&editor)?;
    assert!(ui.no_dialog(&d)?);
    press_ctrl(&mut d, 'n')?;
    assert!(ui.new_dialog_open(&d)?, "Ctrl+N opens the library dialog");
    let mut field_path = NEW_FORM.to_vec();
    field_path.push("name");
    let mut create_path = NEW_FORM.to_vec();
    create_path.extend(["actions", "create"]);
    let field = ui.at(&d, &field_path)?;
    assert!(record(&d, &field)?.visible, "name field shown in the dialog");

    // Empty name.
    ui.click_path(&mut d, &create_path)?;
    let (visible, text) = ui.error_text(&d)?;
    assert!(visible && !text.is_empty(), "empty name explained: {text:?}");
    assert!(ui.new_dialog_open(&d)?);
    assert_eq!(dir_names(&dir)?, before);
    println!("empty: {text}");

    let refused = [
        "a/b",
        "a\\b",
        ".hidden",
        "existing",
        "existing.txt",
        &"n".repeat(65),
    ];
    for name in refused {
        d.replace_text(&field, name)?;
        ui.click_path(&mut d, &create_path)?;
        let (visible, text) = ui.error_text(&d)?;
        assert!(visible && !text.is_empty(), "{name:?} explained");
        assert!(ui.new_dialog_open(&d)?, "{name:?}: dialog stays open");
        assert_eq!(dir_names(&dir)?, before, "{name:?}: no file created");
        assert_eq!(ui.listed(&d)?, ["existing.txt"]);
        println!("{name:?}: {text}");
    }
    assert_eq!(fs::read_to_string(dir.join("existing.txt"))?, "keep");
    let (_, slash) = {
        d.replace_text(&field, "x/y")?;
        ui.click_path(&mut d, &create_path)?;
        ui.error_text(&d)?
    };
    assert!(slash.contains('/'), "slash refusal explains: {slash}");
    let (_, dup) = {
        d.replace_text(&field, "existing")?;
        ui.click_path(&mut d, &create_path)?;
        ui.error_text(&d)?
    };
    assert!(dup.to_lowercase().contains("exist"), "duplicate explains: {dup}");

    // Valid name typed into the field: created, listed, opened and active.
    d.replace_text(&field, "")?;
    d.click(&field)?;
    d.type_text("Groceries")?;
    ui.click_path(&mut d, &create_path)?;
    assert!(ui.no_dialog(&d)?, "dialog closed after creation");
    assert_eq!(fs::read_to_string(dir.join("Groceries.txt"))?, "");
    assert_eq!(ui.listed(&d)?, ["Groceries.txt", "existing.txt"]);
    assert_eq!(ui.titles(&d)?, ["existing.txt", "Groceries.txt"]);
    assert_eq!(ui.active(&d)?.as_deref(), Some("Groceries.txt"));
    assert_eq!(ui.editor_text(&d, "Groceries.txt")?, "");
    assert_eq!(ui.visible_status(&d), ["Groceries.txt: 1 lines, 0 words"]);

    // File -> New note by pointer: a 64-character name that already ends
    // with .txt is accepted as is.
    ui.menu(&mut d, "New note")?;
    assert!(ui.new_dialog_open(&d)?, "menu opens the same dialog");
    let field = ui.at(&d, &field_path)?;
    assert_eq!(
        d.semantics(&field)?.value,
        Some(SemanticValue::Text(String::new())),
        "fresh draft"
    );
    assert!(ui.error_text(&d)?.1.is_empty(), "no stale explanation");
    let long = format!("{}.txt", "m".repeat(60));
    assert_eq!(long.chars().count(), 64);
    d.replace_text(&field, &long)?;
    ui.click_path(&mut d, &create_path)?;
    assert!(ui.no_dialog(&d)?);
    assert!(dir.join(&long).is_file());
    assert_eq!(ui.listed(&d)?, ["Groceries.txt", "existing.txt", long.as_str()]);
    assert_eq!(ui.active(&d)?.as_deref(), Some(long.as_str()));

    // Cancel in the dialog creates nothing.
    let before = dir_names(&dir)?;
    press_ctrl(&mut d, 'n')?;
    let field = ui.at(&d, &field_path)?;
    d.replace_text(&field, "never")?;
    let mut cancel = NEW_FORM.to_vec();
    cancel.extend(["actions", "cancel"]);
    ui.click_path(&mut d, &cancel)?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(dir_names(&dir)?, before);
    Ok(())
}

// ---------------------------------------------------------------- item 4

#[test]
fn item4_close_choices_and_unmodified_close() -> Result {
    let dir = folder("item4")?;
    for (name, text) in [
        ("a.txt", "alpha"),
        ("b.txt", "beta"),
        ("c.txt", "gamma"),
        ("d.txt", "delta"),
    ] {
        fs::write(dir.join(name), text)?;
    }
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    for n in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        ui.click_note(&mut d, n)?;
    }
    assert_eq!(ui.titles(&d)?, ["a.txt", "b.txt", "c.txt", "d.txt"]);
    assert_eq!(ui.active(&d)?.as_deref(), Some("d.txt"));

    // Unmodified, Ctrl+W with the editor focused: closes, no dialog.
    let ed = ui.editor(&d, "d.txt")?;
    d.click(&ed)?;
    press_ctrl(&mut d, 'w')?;
    assert!(ui.no_dialog(&d)?, "no dialog for an unmodified tab");
    assert_eq!(ui.titles(&d)?, ["a.txt", "b.txt", "c.txt"]);
    assert_eq!(fs::read_to_string(dir.join("d.txt"))?, "delta");

    // Unmodified, File -> Close tab by pointer: also closes at once.
    ui.click_note(&mut d, "c.txt")?;
    assert_eq!(ui.active(&d)?.as_deref(), Some("c.txt"));
    ui.menu(&mut d, "Close tab")?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(ui.titles(&d)?, ["a.txt", "b.txt"]);

    // Modified b: Ctrl+W asks; Cancel keeps tab and text.
    ui.click_note(&mut d, "b.txt")?;
    ui.type_lines(&mut d, "b.txt", &[" edited", "more"])?;
    let b_text = "beta edited\nmore";
    assert_eq!(ui.editor_text(&d, "b.txt")?, b_text);
    let b_marked = ui.titles(&d)?[1].clone();
    assert_ne!(b_marked, "b.txt");
    press_ctrl(&mut d, 'w')?;
    assert!(ui.close_dialog_open(&d)?, "close dialog shown in the snapshot");
    let shown: Vec<_> = visible_below(&d, &["close-dialog", "dialog"])
        .into_iter()
        .filter_map(|c| c.details.semantics.and_then(|s| s.name))
        .collect();
    for choice in ["Save", "Discard", "Cancel"] {
        assert!(shown.iter().any(|s| s == choice), "{choice} shown: {shown:?}");
    }
    ui.click_close_choice(&mut d, "cancel")?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(ui.titles(&d)?, ["a.txt", b_marked.as_str()]);
    assert_eq!(ui.editor_text(&d, "b.txt")?, b_text);
    assert_eq!(fs::read_to_string(dir.join("b.txt"))?, "beta");
    // The tab is still editable after Cancel.
    d.type_text("!")?;
    let b_text = format!("{b_text}!");
    assert_eq!(ui.editor_text(&d, "b.txt")?, b_text);

    // Discard through the menu's Close tab: tab gone, file unchanged.
    ui.menu(&mut d, "Close tab")?;
    assert!(ui.close_dialog_open(&d)?);
    ui.click_close_choice(&mut d, "discard")?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(ui.titles(&d)?, ["a.txt"]);
    assert_eq!(fs::read_to_string(dir.join("b.txt"))?, "beta");

    // Save: written, then closed.
    assert_eq!(ui.active(&d)?.as_deref(), Some("a.txt"));
    ui.type_lines(&mut d, "a.txt", &["", "saved on close"])?;
    press_ctrl(&mut d, 'w')?;
    assert!(ui.close_dialog_open(&d)?);
    assert_eq!(fs::read_to_string(dir.join("a.txt"))?, "alpha");
    ui.click_close_choice(&mut d, "save")?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(fs::read_to_string(dir.join("a.txt"))?, "alpha\nsaved on close");
    assert_eq!(ui.tab_count(&d)?, 0);
    assert!(ui.visible_status(&d).is_empty(), "status empty with no tab");

    // Discarded b reopens with the file's text.
    ui.click_note(&mut d, "b.txt")?;
    assert_eq!(ui.editor_text(&d, "b.txt")?, "beta");
    assert_eq!(ui.titles(&d)?, ["b.txt"]);

    // Save that fails keeps the tab open and modified.
    ui.type_lines(&mut d, "b.txt", &[" unsaved"])?;
    fs::remove_dir_all(&dir)?;
    press_ctrl(&mut d, 'w')?;
    assert!(ui.close_dialog_open(&d)?);
    ui.click_close_choice(&mut d, "save")?;
    assert_eq!(ui.tab_count(&d)?, 1, "failed save keeps the tab");
    assert_ne!(ui.titles(&d)?[0], "b.txt", "still marked");
    assert_eq!(ui.editor_text(&d, "b.txt")?, "beta unsaved");
    let (visible, text) = ui.notification(&d)?;
    assert!(visible && text.contains("b.txt"), "failure notified: {text}");
    // The dialog stays up after the failure; Cancel returns to the tab.
    assert!(ui.close_dialog_open(&d)?);
    ui.click_close_choice(&mut d, "cancel")?;
    assert!(ui.no_dialog(&d)?);
    assert_eq!(ui.tab_count(&d)?, 1);
    Ok(())
}

// ---------------------------------------------------------------- item 5

#[test]
fn item5_cpu_capture_two_tabs_one_modified() -> Result {
    let dir = folder("item5")?;
    fs::write(dir.join("Shopping.txt"), "milk\neggs\nbread")?;
    fs::write(dir.join("Ideas.txt"), "a notes editor")?;
    fs::write(dir.join("ignored.log"), "not listed")?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    ui.click_note(&mut d, "Ideas.txt")?;
    ui.click_note(&mut d, "Shopping.txt")?;
    ui.type_lines(&mut d, "Shopping.txt", &["", "apples"])?;
    let titles = ui.titles(&d)?;
    assert_eq!(titles.len(), 2);
    assert_eq!(titles[0], "Ideas.txt");
    assert_ne!(titles[1], "Shopping.txt");
    println!("titles at capture: {titles:?}");
    d.runtime().set_capture_support(true);
    let pending = d.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &d.runtime().primary_window(),
    )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("verifier-captures");
    fs::create_dir_all(&out)?;
    let png = capture.image.encode_png();
    assert!(png.starts_with(b"\x89PNG"));
    fs::write(out.join("two-tabs-one-modified.png"), png)?;
    Ok(())
}

// ------------------------------------------------- feature (not an item)

/// The sidebar divider is draggable: a pointer drag on the splitter's
/// divider changes the first pane's extent reported by inspection.
#[test]
fn feature_divider_drag() -> Result {
    let dir = folder("divider")?;
    fs::write(dir.join("a.txt"), "a")?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    let split = ui.at(&d, &["close-dialog", "bar", "body", "split"])?;
    let extent = |d: &Driver<'_>| prop(&record(d, &split).unwrap(), "extent");
    let start_extent = extent(&d);
    assert_eq!(start_extent, Some(InspectValue::Number(220.0)));
    let [sx, sy, _, sh] = d.runtime().bounds(&split)?.ok_or("split not laid out")?.values();
    let mid = sy + sh / 2.0;
    let mut moved = false;
    for dx in [220.0f32, 221.0, 222.0, 223.0, 219.0, 224.0, 225.0, 226.0, 218.0] {
        let p = Point::new(sx + dx, mid)?;
        let rt = d.runtime_mut();
        rt.pointer_input(PointerEvent::new(p, PointerKind::Move))?;
        rt.pointer_input(PointerEvent::new(p, PointerKind::Press(PointerButton::Primary)))?;
        let q = Point::new(sx + dx + 80.0, mid)?;
        rt.pointer_input(PointerEvent::new(q, PointerKind::Move))?;
        rt.pointer_input(PointerEvent::new(q, PointerKind::Release(PointerButton::Primary)))?;
        d.settle()?;
        if extent(&d) != start_extent {
            println!("divider dragged from x={dx}: extent {:?}", extent(&d));
            moved = true;
            break;
        }
    }
    assert!(moved, "divider drag changed the extent");
    Ok(())
}

// ------------------------------------------- supporting captures (items 2, 3)

fn capture(d: &mut Driver<'_>, name: &str) -> Result {
    d.runtime().set_capture_support(true);
    let pending = d.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &d.runtime().primary_window(),
    )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.try_reply()?.value?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("verifier-captures");
    fs::create_dir_all(&out)?;
    fs::write(out.join(name), capture.image.encode_png())?;
    Ok(())
}

/// Captures the failed-save notification and a New note refusal, so the
/// displayed failure text is seen as well as read from inspection.
#[test]
fn supporting_captures_failure_and_refusal() -> Result {
    let dir = folder("captures")?;
    fs::write(dir.join("todo.txt"), "one")?;
    let mut app = start(&dir)?;
    let ui = Ui {
        root: app.state().root.clone(),
    };
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    ui.click_note(&mut d, "todo.txt")?;
    ui.type_lines(&mut d, "todo.txt", &[" two"])?;
    fs::remove_dir_all(&dir)?;
    press_ctrl(&mut d, 's')?;
    assert!(ui.notification(&d)?.1.contains("Save failed"));
    capture(&mut d, "failed-save-notification.png")?;
    fs::create_dir_all(&dir)?;
    press_ctrl(&mut d, 'n')?;
    let mut create = NEW_FORM.to_vec();
    create.extend(["actions", "create"]);
    let mut field = NEW_FORM.to_vec();
    field.push("name");
    let field = ui.at(&d, &field)?;
    d.replace_text(&field, "bad/name")?;
    ui.click_path(&mut d, &create)?;
    assert!(ui.error_text(&d)?.1.contains('/'));
    capture(&mut d, "new-note-refusal.png")?;
    Ok(())
}
