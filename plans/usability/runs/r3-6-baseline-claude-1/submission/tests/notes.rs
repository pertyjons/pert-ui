//! Headless acceptance checks: each test starts the application with
//! `App::headless` on a folder of its own and drives it with the library's
//! `Driver`, reading what it shows through inspection.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{Detail, InspectionQuery, KeyChord, Selector, SemanticRole, Semantics};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

type Res<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Harness {
    app: HeadlessApp,
    root: WidgetId,
    dir: PathBuf,
}

/// Creates a fresh notes folder holding `files` and starts the application.
fn start(test: &str, files: &[(&str, &str)]) -> Res<Harness> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(test);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    for (name, text) in files {
        std::fs::write(dir.join(name), text)?;
    }
    let slot = Rc::new(RefCell::new(None));
    let app = notes::build_app_reporting(dir.clone(), slot.clone())?.headless()?;
    let root = slot.borrow().clone().ok_or("the application mounted nothing")?;
    let mut harness = Harness { app, root, dir };
    harness.driver().settle()?;
    Ok(harness)
}

impl Harness {
    fn driver(&mut self) -> Driver<'_> {
        Driver::new(self.app.runtime_mut())
    }

    fn find(&self, key: &str) -> Res<WidgetId> {
        Ok(self.app.runtime().find_descendant(&self.root, key)?.ok_or(format!("no {key}"))?)
    }

    fn semantics(&self, id: &WidgetId) -> Res<Semantics> {
        let rt = self.app.runtime();
        let id = rt.inspection_id(id)?;
        let result = rt.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics));
        let component = result.components.into_iter().next().ok_or("not inspected")?;
        Ok(component.details.semantics.ok_or("no semantics")?)
    }

    /// The names the sidebar shows, in display order.
    fn listed(&self) -> Res<Vec<String>> {
        let rt = self.app.runtime();
        let sidebar = self.find("sidebar")?;
        let mut rows = Vec::new();
        for row in rt.children(&sidebar)? {
            let top = rt.bounds(&row)?.ok_or("row not laid out")?.y();
            let name = self.semantics(&row)?.name.ok_or("row without a name")?;
            rows.push((top, name));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|(_, name)| name).collect())
    }

    /// The tab titles in strip order, with the selected one's index.
    fn tabs(&self) -> Res<(Vec<String>, Option<usize>)> {
        let strip = self.semantics(&self.find("tabs")?)?;
        let titles = strip.items.iter().map(|item| item.name.clone().unwrap_or_default()).collect();
        let selected = strip.items.iter().position(|item| item.selected);
        Ok((titles, selected))
    }

    fn selected_title(&self) -> Res<Option<String>> {
        let (titles, selected) = self.tabs()?;
        Ok(selected.map(|index| titles[index].clone()))
    }

    /// The text of the status line.
    fn status(&self) -> Res<String> {
        let rt = self.app.runtime();
        let status = self.find("status")?;
        let line = rt.children(&status)?.into_iter().next().ok_or("no status label")?;
        Ok(self.semantics(&line)?.name.unwrap_or_default())
    }

    /// Every name, description and text value visible in the window.
    fn visible_text(&self) -> Vec<String> {
        let snapshot = self.app.runtime().inspect();
        let mut texts = Vec::new();
        for component in snapshot.components.iter().filter(|c| c.visible) {
            if let Some(semantics) = &component.details.semantics {
                texts.extend(semantics.name.clone());
                texts.extend(semantics.description.clone());
                if let Some(SemanticValue::Text(text)) = &semantics.value {
                    texts.push(text.clone());
                }
            }
        }
        texts
    }

    fn shows(&self, needle: &str) -> bool {
        self.visible_text().iter().any(|text| text.contains(needle))
    }

    fn is_visible(&self, key: &str) -> Res<bool> {
        let rt = self.app.runtime();
        let id = rt.inspection_id(&self.find(key)?)?;
        let result = rt.query(&InspectionQuery::components(Selector::id(id)));
        Ok(result.components.first().is_some_and(|c| c.visible))
    }

    /// The editor of the open note `name`.
    fn editor(&self, name: &str) -> Res<WidgetId> {
        let strip = self.find("tabs")?;
        Ok(self.app.runtime().find(&strip, &[name, "editor"])?.ok_or(format!("no editor for {name}"))?)
    }

    /// Opens `name` with a click on its sidebar row.
    fn click_note(&mut self, name: &str) -> Res {
        let sidebar = self.find("sidebar")?;
        let row = self.app.runtime().find(&sidebar, &[name])?.ok_or(format!("no row {name}"))?;
        let mut driver = self.driver();
        driver.click(&row)?;
        // Separate single clicks, not a double click.
        driver.advance(ClickCounter::DEFAULT_INTERVAL + std::time::Duration::from_millis(1))?;
        driver.settle()?;
        Ok(())
    }

    /// The highlighted row of the open File menu, if one is open.
    fn menu_highlight(&self) -> Option<String> {
        let snapshot = self.app.runtime().inspect();
        snapshot.components.iter().filter(|c| c.visible).find_map(|c| {
            let semantics = c.details.semantics.as_ref()?;
            if semantics.role != SemanticRole::Menu || semantics.items.is_empty() {
                return None;
            }
            semantics.items.iter().find(|item| item.selected).and_then(|item| item.name.clone())
        })
    }

    /// Chooses the File menu's row named `row` with F10, the arrow keys and
    /// Enter, as a keyboard user does.
    fn file_menu(&mut self, row: &str) -> Res {
        self.driver().key(KeyCode::F10)?;
        self.driver().key(KeyCode::ArrowDown)?;
        for _ in 0..4 {
            if self.menu_highlight().as_deref() == Some(row) {
                break;
            }
            self.driver().key(KeyCode::ArrowDown)?;
        }
        assert_eq!(self.menu_highlight().as_deref(), Some(row));
        self.driver().key(KeyCode::Enter)?;
        self.driver().settle()?;
        Ok(())
    }

    /// A button of the close confirmation dialog.
    fn close_action(&self, key: &str) -> Res<WidgetId> {
        let form = self.find("close-form")?;
        Ok(self.app.runtime().find(&form, &["actions", key])?.ok_or(format!("no {key}"))?)
    }

    fn chord(&mut self, letter: char, code: KeyCode) -> Res {
        let mut driver = self.driver();
        driver.chord(KeyChord::primary(letter), code)?;
        driver.settle()?;
        Ok(())
    }

    fn file(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(name)).ok()
    }
}

#[test]
fn lists_notes_in_order_and_reopening_switches_tab() -> Res {
    let mut h = start(
        "listing",
        &[("cherry.txt", "c"), ("apple.txt", "a"), ("banana.txt", "b"), ("readme.md", "x"), ("data.json", "{}")],
    )?;
    std::fs::create_dir_all(h.dir.join("folder.txt"))?;
    assert_eq!(h.listed()?, ["apple.txt", "banana.txt", "cherry.txt"]);
    assert_eq!(h.tabs()?.0, Vec::<String>::new());
    assert_eq!(h.status()?, "");

    // Pointer: open two notes.
    h.click_note("banana.txt")?;
    assert_eq!(h.tabs()?, (vec!["banana.txt".to_owned()], Some(0)));
    h.click_note("cherry.txt")?;
    assert_eq!(h.tabs()?, (vec!["banana.txt".to_owned(), "cherry.txt".to_owned()], Some(1)));

    // Reopening switches to the existing tab instead of opening another.
    h.click_note("banana.txt")?;
    assert_eq!(h.tabs()?, (vec!["banana.txt".to_owned(), "cherry.txt".to_owned()], Some(0)));
    assert_eq!(h.status()?, "banana.txt: 1 lines, 1 words");

    // Keyboard: arrows and Enter in the sidebar.
    let sidebar = h.find("sidebar")?;
    let mut driver = h.driver();
    driver.runtime_mut().focus(&sidebar)?;
    driver.key(KeyCode::Home)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(h.selected_title()?.as_deref(), Some("apple.txt"));
    let mut driver = h.driver();
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::ArrowDown)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(h.tabs()?.0, ["banana.txt", "cherry.txt", "apple.txt"]);
    assert_eq!(h.selected_title()?.as_deref(), Some("cherry.txt"));

    // The divider between the sidebar and the tabs resizes them.
    let width = |h: &Harness| -> Res<f32> {
        Ok(h.app.runtime().bounds(&h.find("sidebar")?)?.ok_or("no bounds")?.width())
    };
    let before = width(&h)?;
    let split = h.find("split")?;
    let mut driver = h.driver();
    driver.runtime_mut().focus(&split)?;
    driver.key(KeyCode::ArrowRight)?;
    driver.key(KeyCode::ArrowRight)?;
    assert_eq!(width(&h)?, before + 16.0);
    h.app.shutdown()?;
    Ok(())
}

#[test]
fn typing_marks_modified_and_saving_clears_it() -> Res {
    let mut h = start("saving", &[("one.txt", "first line"), ("two.txt", "")])?;
    h.click_note("one.txt")?;
    assert_eq!(h.status()?, "one.txt: 1 lines, 2 words");

    let editor = h.editor("one.txt")?;
    let mut driver = h.driver();
    driver.runtime_mut().focus(&editor)?;
    driver.type_text("Hello brave world\nnew ")?;
    driver.settle()?;
    assert_eq!(h.tabs()?.0, ["one.txt *"]);
    assert_eq!(h.status()?, "one.txt: 2 lines, 6 words");

    // Ctrl+S saves.
    h.chord('s', KeyCode::KeyS)?;
    assert_eq!(h.file("one.txt").as_deref(), Some("Hello brave world\nnew first line"));
    assert_eq!(h.tabs()?.0, ["one.txt"]);

    // File > Save saves too.
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("More ")?;
    assert_eq!(h.tabs()?.0, ["one.txt *"]);
    assert_eq!(h.status()?, "one.txt: 2 lines, 7 words");
    h.file_menu("Save")?;
    assert_eq!(h.file("one.txt").as_deref(), Some("Hello brave world\nnew More first line"));
    assert_eq!(h.tabs()?.0, ["one.txt"]);

    // A failed save is reported and keeps the marker.
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("Lost ")?;
    std::fs::remove_dir_all(&h.dir)?;
    h.chord('s', KeyCode::KeyS)?;
    assert!(h.shows("Could not save one.txt"), "visible: {:?}", h.visible_text());
    assert_eq!(h.tabs()?.0, ["one.txt *"]);
    h.app.shutdown()?;
    Ok(())
}

#[test]
fn new_note_dialog_refuses_invalid_names() -> Res {
    let mut h = start("new-note", &[("existing.txt", "here")])?;
    assert!(!h.is_visible("new-form")?);
    h.chord('n', KeyCode::KeyN)?;
    assert!(h.is_visible("new-form")?);
    let field = h.find("name")?;
    let create = h.find("create")?;
    let before = std::fs::read_dir(&h.dir)?.count();

    // Empty name.
    h.driver().click(&create)?;
    assert!(h.is_visible("new-form")?);
    assert!(h.shows("Enter a name for the note."), "visible: {:?}", h.visible_text());

    // A name with a slash.
    h.driver().replace_text(&field, "a/b")?;
    h.driver().click(&create)?;
    assert!(h.is_visible("new-form")?);
    assert!(h.shows("must not contain /"), "visible: {:?}", h.visible_text());

    // An existing name, with and without the extension; Enter submits.
    h.driver().replace_text(&field, "existing")?;
    h.driver().key(KeyCode::Enter)?;
    assert!(h.is_visible("new-form")?);
    assert!(h.shows("A note named existing.txt already exists."), "visible: {:?}", h.visible_text());
    assert_eq!(std::fs::read_dir(&h.dir)?.count(), before);
    assert_eq!(h.file("existing.txt").as_deref(), Some("here"));

    // A valid name creates, lists and opens the note.
    h.driver().replace_text(&field, "groceries")?;
    h.driver().click(&create)?;
    h.driver().settle()?;
    assert!(!h.is_visible("new-form")?);
    assert_eq!(h.file("groceries.txt").as_deref(), Some(""));
    assert_eq!(h.listed()?, ["existing.txt", "groceries.txt"]);
    assert_eq!(h.tabs()?, (vec!["groceries.txt".to_owned()], Some(0)));
    assert_eq!(h.status()?, "groceries.txt: 1 lines, 0 words");

    // The menu row opens the same dialog, empty again.
    h.file_menu("New note")?;
    assert!(h.is_visible("new-form")?);
    assert_eq!(h.semantics(&field)?.value, Some(SemanticValue::Text(String::new())));
    assert!(!h.shows("already exists"));
    h.app.shutdown()?;
    Ok(())
}

#[test]
fn closing_tabs() -> Res {
    let mut h = start("closing", &[("a.txt", "alpha"), ("b.txt", "beta")])?;
    h.click_note("a.txt")?;
    h.click_note("b.txt")?;
    let editor = h.editor("b.txt")?;
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("new ")?;
    assert_eq!(h.tabs()?.0, ["a.txt", "b.txt *"]);

    // Cancel keeps the tab and its text.
    h.chord('w', KeyCode::KeyW)?;
    assert!(h.is_visible("close-form")?);
    assert!(h.shows("b.txt has unsaved changes."));
    let cancel = h.close_action("cancel")?;
    h.driver().click(&cancel)?;
    assert!(!h.is_visible("close-form")?);
    assert_eq!(h.tabs()?.0, ["a.txt", "b.txt *"]);
    assert_eq!(h.semantics(&editor)?.value, Some(SemanticValue::Text("new beta".into())));
    assert_eq!(h.file("b.txt").as_deref(), Some("beta"));

    // Escape cancels as well.
    h.file_menu("Close tab")?;
    assert!(h.is_visible("close-form")?);
    h.driver().key(KeyCode::Escape)?;
    assert!(!h.is_visible("close-form")?);
    assert_eq!(h.tabs()?.0, ["a.txt", "b.txt *"]);

    // Discard closes without writing.
    h.chord('w', KeyCode::KeyW)?;
    let discard = h.close_action("discard")?;
    h.driver().click(&discard)?;
    h.driver().settle()?;
    assert!(!h.is_visible("close-form")?);
    assert_eq!(h.tabs()?.0, ["a.txt"]);
    assert_eq!(h.file("b.txt").as_deref(), Some("beta"));
    assert_eq!(h.status()?, "a.txt: 1 lines, 1 words");

    // Save writes, then closes.
    let editor = h.editor("a.txt")?;
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("saved ")?;
    h.chord('w', KeyCode::KeyW)?;
    assert!(h.is_visible("close-form")?, "visible: {:?}", h.visible_text());
    let save = h.close_action("save")?;
    h.driver().click(&save)?;
    h.driver().settle()?;
    assert_eq!(h.tabs()?.0, Vec::<String>::new());
    assert_eq!(h.file("a.txt").as_deref(), Some("saved alpha"));
    assert_eq!(h.status()?, "");

    // An unmodified tab closes at once, with no dialog.
    h.click_note("b.txt")?;
    assert_eq!(h.tabs()?.0, ["b.txt"]);
    h.chord('w', KeyCode::KeyW)?;
    assert!(!h.is_visible("close-form")?);
    assert_eq!(h.tabs()?.0, Vec::<String>::new());

    // A failed save in the dialog keeps the tab open.
    h.click_note("b.txt")?;
    let editor = h.editor("b.txt")?;
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("x")?;
    std::fs::remove_dir_all(&h.dir)?;
    h.file_menu("Close tab")?;
    let save = h.close_action("save")?;
    h.driver().click(&save)?;
    h.driver().settle()?;
    assert_eq!(h.tabs()?.0, ["b.txt *"]);
    assert!(h.shows("Could not save b.txt"), "visible: {:?}", h.visible_text());
    h.app.shutdown()?;
    Ok(())
}

#[test]
fn capture_two_notes_one_modified() -> Res {
    let mut h = start(
        "capture",
        &[
            ("groceries.txt", "Milk\nBread\nCoffee beans\n"),
            ("ideas.txt", "A notes editor in pert-ui.\nTabs, a sidebar and a status line."),
            ("todo.txt", "Write the friction log"),
        ],
    )?;
    h.click_note("groceries.txt")?;
    h.click_note("ideas.txt")?;
    let editor = h.editor("ideas.txt")?;
    h.driver().runtime_mut().focus(&editor)?;
    h.driver().type_text("Edited: ")?;
    assert_eq!(h.tabs()?.0, ["groceries.txt", "ideas.txt *"]);

    let mut driver = h.driver();
    driver.runtime().set_capture_support(true);
    let window = driver.runtime().primary_window();
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(&window)));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (900, 600));
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capture.png");
    std::fs::write(&out, capture.image.encode_png())?;
    h.app.shutdown()?;
    Ok(())
}
