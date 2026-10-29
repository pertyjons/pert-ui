//! The layout guide's recipe for focus after closing the selected tab.
//!
//! Removing the selected tab selects its neighbour with origin
//! `Redescription`; focus that was in the removed editor falls back to the
//! strip. The recipe's `on_select` queues a refocus only while the strip
//! itself has focus. The queued work flushes first, so input handled since
//! has reported its selection change, then rechecks that the strip still has
//! focus and the queued key is still selected, and focuses that tab's editor,
//! found by an application-unique key, when it has one.
use pert_ui::testing::Driver;
use pert_ui::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<String>>>;

#[derive(Clone)]
struct Note {
    key: String,
    /// `None` for a tab without an editor, such as a read-only preview.
    text: Option<Signal<String>>,
}
impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && match (&self.text, &other.text) {
                (Some(text), Some(other)) => text.read_only().same_source(&other.read_only()),
                (None, None) => true,
                _ => false,
            }
    }
}

/// Unique in the application, so the lookup below the strip finds one editor.
fn editor_key(note: &str) -> String {
    format!("note-editor-{note}")
}

/// The recipe's `on_select`, with each decision of the queued work logged.
fn refocus_after_close(
    selected: Signal<Option<String>>,
    log: Log,
) -> impl FnMut(&mut EventContext, &SelectionChange) -> Result<(), RuntimeError> + 'static {
    move |cx, change| {
        selected.set(change.selected.clone())?;
        if change.origin == ChangeOrigin::Redescription
            && cx.focused()
            && let Some(key) = change.selected.clone()
        {
            log.borrow_mut().push(format!("queued:{key}"));
            let (tabs, selected, log) = (cx.id(), selected.clone(), log.clone());
            cx.defer(move |ui| {
                ui.flush(ui.now())?;
                if ui.focused().as_ref() != Some(&tabs) {
                    log.borrow_mut().push(format!("focus-elsewhere:{key}"));
                    return Ok(());
                }
                if selected.get()?.as_deref() != Some(key.as_str()) {
                    log.borrow_mut().push(format!("stale:{key}"));
                    return Ok(());
                }
                let Some(editor) = ui.find_descendant(&tabs, &editor_key(&key))? else {
                    log.borrow_mut().push(format!("no-editor:{key}"));
                    return Ok(());
                };
                log.borrow_mut().push(format!("focus:{key}"));
                ui.focus(&editor)
            })?;
        }
        Ok(())
    }
}

struct Fixture {
    notes: Signal<Vec<Note>>,
    selected: Signal<Option<String>>,
    log: Log,
    tabs: WidgetId,
    sidebar: WidgetId,
    sidebar_text: Signal<String>,
}

impl Fixture {
    /// One root: a sidebar field, the other region, above the notes' tabs.
    fn mount(ui: &mut Runtime, keys: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        ui.set_viewport(Viewport::new(640.0, 360.0, 1.0)?)?;
        let notes = ui.signal(keys.iter().map(|key| note(ui, key)).collect::<Vec<_>>());
        let selected = ui.signal(keys.first().map(|key| (*key).to_owned()));
        let log: Log = Rc::default();
        let sidebar_text = ui.signal(String::new());
        let (observed, decisions) = (selected.clone(), log.clone());
        let root = ui.mount_tree(
            Column::new([
                TextField::new(&sidebar_text).key("sidebar"),
                Dynamic::new(&notes, move |_, notes: &Vec<Note>| {
                    Tabs::new(notes.iter().map(|note| {
                        let content = match &note.text {
                            Some(text) => TextArea::new(text).key(editor_key(&note.key)),
                            None => Label::new(format!("Preview of {}", note.key)).key("preview"),
                        };
                        Tab::new(note.key.clone(), note.key.to_uppercase(), content)
                    }))
                    .on_select(refocus_after_close(observed.clone(), decisions.clone()))
                    .key("tabs")
                })
                .key("notes")
                .with_grow(1),
            ])
            .key("app"),
        )?;
        ui.flush(Duration::ZERO)?;
        let issues = ui.validation_report().issues;
        assert!(issues.is_empty(), "{issues:?}");
        Ok(Self {
            notes,
            selected,
            log,
            tabs: ui.find(&root, &["notes", "tabs"])?.ok_or("the strip")?,
            sidebar: ui.find(&root, &["sidebar"])?.ok_or("the sidebar")?,
            sidebar_text,
        })
    }

    fn editor(&self, ui: &Runtime, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        Ok(ui
            .find_descendant(&self.tabs, &editor_key(key))?
            .ok_or("an editor")?)
    }

    fn text(&self, key: &str) -> Result<String, Box<dyn std::error::Error>> {
        let notes = self.notes.get()?;
        let note = notes.iter().find(|note| note.key == key).ok_or("a note")?;
        Ok(note.text.as_ref().ok_or("an editable note")?.get()?)
    }

    fn close(&self, key: &str) -> Result<(), RuntimeError> {
        self.notes
            .update(|notes| notes.retain(|note| note.key != key))
    }

    fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}

fn note(ui: &Runtime, key: &str) -> Note {
    Note {
        key: key.to_owned(),
        text: (key != "preview").then(|| ui.signal(format!("text {key}"))),
    }
}

fn some(key: &str) -> Option<String> {
    Some(key.to_owned())
}

/// A primary click at the middle of `[x, y, width, height]` straight through
/// [`Runtime::pointer_input`], as a host delivers it: unlike `Driver::click`,
/// nothing is revealed or flushed before the press.
fn host_click(ui: &mut Runtime, [x, y, width, height]: [f32; 4]) -> TestResult {
    let at = Point::new(x + width / 2.0, y + height / 2.0)?;
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(at, kind))?;
    }
    Ok(())
}

#[test]
fn closing_the_focused_selected_editor_moves_typing_to_the_next_editor() -> TestResult {
    let mut ui = Runtime::new();
    let fixture = Fixture::mount(&mut ui, &["a", "b", "c"])?;
    let (editor_b, editor_c) = (fixture.editor(&ui, "b")?, fixture.editor(&ui, "c")?);
    ui.focus(&fixture.editor(&ui, "a")?)?;
    let mut driver = Driver::new(&mut ui);
    driver.type_text("1")?;
    assert_eq!(fixture.text("a")?, "1text a");

    fixture.close("a")?;
    driver.settle()?;
    // The removal flush reported the change; focus fell back to the strip.
    assert_eq!(fixture.selected.get()?, some("b"));
    assert_eq!(driver.runtime().focused(), Some(fixture.tabs.clone()));
    assert_eq!(fixture.log(), ["queued:b"]);
    driver.settle()?;
    assert_eq!(driver.runtime().focused(), Some(editor_b.clone()));
    assert_eq!(fixture.log(), ["queued:b", "focus:b"]);
    driver.type_text("Z")?;
    assert_eq!(fixture.text("b")?, "Ztext b");

    // Adding a note keeps the selection, the focused editor and its text.
    let d = note(driver.runtime(), "d");
    fixture.notes.update(|notes| notes.push(d))?;
    driver.settle()?;
    driver.settle()?;
    assert_eq!(fixture.selected.get()?, some("b"));
    assert_eq!(driver.runtime().focused(), Some(editor_b.clone()));
    assert_eq!(fixture.editor(driver.runtime(), "b")?, editor_b);
    assert_eq!(fixture.editor(driver.runtime(), "c")?, editor_c);
    assert_eq!(fixture.text("b")?, "Ztext b");
    assert_eq!(fixture.text("c")?, "text c");
    assert_eq!(fixture.log(), ["queued:b", "focus:b"]);
    Ok(())
}

#[test]
fn an_already_focused_strip_also_moves_to_the_next_editor() -> TestResult {
    let mut ui = Runtime::new();
    let fixture = Fixture::mount(&mut ui, &["a", "b", "c"])?;
    ui.focus(&fixture.tabs)?;
    fixture.close("a")?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.settle()?;
    let editor_b = fixture.editor(driver.runtime(), "b")?;
    assert_eq!(driver.runtime().focused(), Some(editor_b));
    assert_eq!(fixture.log(), ["queued:b", "focus:b"]);
    Ok(())
}

#[test]
fn focus_in_another_region_is_never_taken() -> TestResult {
    let mut ui = Runtime::new();
    let fixture = Fixture::mount(&mut ui, &["a", "b", "c"])?;
    ui.focus(&fixture.sidebar)?;
    fixture.close("a")?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.settle()?;
    assert_eq!(fixture.selected.get()?, some("b"));
    assert_eq!(driver.runtime().focused(), Some(fixture.sidebar.clone()));
    assert_eq!(fixture.log(), Vec::<String>::new());
    driver.type_text("query")?;
    assert_eq!(fixture.sidebar_text.get()?, "query");
    assert_eq!(fixture.text("b")?, "text b");
    Ok(())
}

#[test]
fn focus_moved_elsewhere_before_the_queued_work_is_kept() -> TestResult {
    for click in [false, true] {
        let mut ui = Runtime::new();
        let fixture = Fixture::mount(&mut ui, &["a", "b", "c"])?;
        ui.focus(&fixture.editor(&ui, "a")?)?;
        fixture.close("a")?;
        let mut driver = Driver::new(&mut ui);
        driver.settle()?;
        assert_eq!(driver.runtime().focused(), Some(fixture.tabs.clone()));
        assert_eq!(fixture.log(), ["queued:b"]);
        // Focus leaves the strip before the next flush runs the queued work.
        if click {
            let bounds = driver
                .runtime()
                .bounds(&fixture.sidebar)?
                .ok_or("laid out")?;
            host_click(driver.runtime_mut(), bounds.values())?;
        } else {
            driver.runtime_mut().focus(&fixture.sidebar)?;
        }
        driver.settle()?;
        assert_eq!(
            driver.runtime().focused(),
            Some(fixture.sidebar.clone()),
            "click {click}"
        );
        assert_eq!(
            fixture.log(),
            ["queued:b", "focus-elsewhere:b"],
            "click {click}"
        );
    }
    Ok(())
}

#[test]
fn a_selection_changed_before_the_queued_work_is_kept() -> TestResult {
    let mut ui = Runtime::new();
    let fixture = Fixture::mount(&mut ui, &["a", "b", "c"])?;
    ui.focus(&fixture.editor(&ui, "a")?)?;
    fixture.close("a")?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    assert_eq!(fixture.log(), ["queued:b"]);
    // `Runtime::key_input` delivers the key at once, so the strip selects C
    // before the flush that runs the queued work, and that flush reports the
    // change only after the work has started. (`command_key_input`, and with
    // it `Driver::key`, flushes first: the work would run before the key.)
    let ui = driver.runtime_mut();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    driver.settle()?;
    assert_eq!(fixture.selected.get()?, some("c"));
    assert_eq!(fixture.log(), ["queued:b", "stale:b"]);
    // A user's selection queues nothing: focus stays on the strip.
    driver.settle()?;
    assert_eq!(driver.runtime().focused(), Some(fixture.tabs.clone()));
    assert_eq!(fixture.log(), ["queued:b", "stale:b"]);
    Ok(())
}

#[test]
fn a_selected_tab_without_the_editor_leaves_focus_on_the_strip() -> TestResult {
    let mut ui = Runtime::new();
    let fixture = Fixture::mount(&mut ui, &["a", "preview", "c"])?;
    ui.focus(&fixture.editor(&ui, "a")?)?;
    fixture.close("a")?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.settle()?;
    assert_eq!(fixture.selected.get()?, some("preview"));
    assert_eq!(driver.runtime().focused(), Some(fixture.tabs.clone()));
    assert_eq!(fixture.log(), ["queued:preview", "no-editor:preview"]);
    Ok(())
}
