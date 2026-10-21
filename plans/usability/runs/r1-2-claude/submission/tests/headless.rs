//! Drives the to-do list headlessly through the runtime's own input entry
//! points: native key routing (`command_key_input`), committed text and
//! pointer events. The model is only read, never called.

use std::time::Duration;

use pert_ui::prelude::*;
use pert_ui::{CommandKey, CommandKeyInput, Modifiers, Point, PointerEvent, PointerKind};
use todo::{Task, TodoApp, task_key};

type Res = Result<(), Box<dyn std::error::Error>>;

struct Harness {
    ui: Runtime,
    app: TodoApp,
}

impl Harness {
    fn new(initial: Vec<Task>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 600.0, 1.0)?)?;
        let app = todo::build(&mut ui, initial)?;
        ui.flush(Duration::ZERO)?;
        Ok(Self { ui, app })
    }

    fn flush(&mut self) -> Result<(), RuntimeError> {
        self.ui.flush(Duration::ZERO).map(|_| ())
    }

    /// A key press and release as the native host delivers it, with
    /// `modifiers` held.
    fn key_with(&mut self, code: KeyCode, base: CommandKey, modifiers: Modifiers) -> Res {
        self.ui.native_modifiers(modifiers)?;
        for action in [KeyAction::Press, KeyAction::Release] {
            self.ui
                .command_key_input(CommandKeyInput::new(code, action, base))?;
        }
        self.ui.native_modifiers(Modifiers::default())?;
        self.flush()?;
        Ok(())
    }

    fn key(&mut self, code: KeyCode) -> Res {
        self.key_with(code, CommandKey::Named(code), Modifiers::default())
    }

    fn alt(&mut self, code: KeyCode) -> Res {
        let alt = Modifiers {
            alt: true,
            ..Modifiers::default()
        };
        self.key_with(code, CommandKey::Named(code), alt)
    }

    fn ctrl_n(&mut self) -> Res {
        self.key_with(KeyCode::KeyN, CommandKey::Character('n'), Modifiers::PRIMARY)
    }

    fn type_text(&mut self, text: &str) -> Res {
        self.ui.text_input(text)?;
        self.flush()?;
        Ok(())
    }

    fn center(&self, id: &WidgetId) -> Result<Point, Box<dyn std::error::Error>> {
        let rect = self.ui.bounds(id)?.ok_or("component has no bounds")?;
        Ok(Point::new(
            rect.x() + rect.width() / 2.0,
            rect.y() + rect.height() / 2.0,
        )?)
    }

    fn click_at(&mut self, at: Point) -> Res {
        self.ui.pointer_input(PointerEvent::new(at, PointerKind::Move))?;
        self.ui
            .pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
        self.ui
            .pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
        self.flush()?;
        Ok(())
    }

    fn click(&mut self, id: &WidgetId) -> Res {
        let at = self.center(id)?;
        self.click_at(at)
    }

    fn double_click(&mut self, id: &WidgetId) -> Res {
        let at = self.center(id)?;
        self.click_at(at)?;
        self.click_at(at)
    }

    /// A component of the task row with `id`, by its key in the row.
    fn row_part(&self, id: u64, part: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        let row = self
            .ui
            .find_descendant(&self.app.list, &task_key(id))?
            .ok_or_else(|| format!("row {id} is not mounted"))?;
        Ok(self
            .ui
            .find_descendant(&row, part)?
            .ok_or_else(|| format!("row {id} has no {part}"))?)
    }

    fn button(&self, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        Ok(self
            .ui
            .find(&self.app.root, &["actions", key])?
            .ok_or_else(|| format!("no button {key}"))?)
    }

    fn titles(&self) -> Vec<String> {
        self.app.tasks.get().unwrap().iter().map(|t| t.title.clone()).collect()
    }

    fn status(&self) -> String {
        self.app.status.get().unwrap()
    }

    fn active(&self) -> Option<String> {
        self.app.selection.active().get().unwrap()
    }

    fn mounted_rows(&self) -> Vec<String> {
        self.ui
            .children(&self.app.list)
            .unwrap()
            .iter()
            .map(|c| self.ui.key(c).unwrap().to_owned())
            .collect()
    }
}

fn task(id: u64, title: &str) -> Task {
    Task {
        id,
        title: title.to_owned(),
        done: false,
    }
}

fn three() -> Vec<Task> {
    vec![task(1, "Buy milk"), task(2, "Walk the dog"), task(3, "Write report")]
}

#[test]
fn add_with_enter_and_button_refusing_empty_titles() -> Res {
    let mut h = Harness::new(Vec::new())?;
    assert_eq!(h.status(), "0 of 0 done");

    // Ctrl+N focuses the new-task field.
    h.ctrl_n()?;
    assert_eq!(h.ui.focused(), Some(h.app.new_title_field.clone()));

    h.type_text("Buy milk")?;
    h.key(KeyCode::Enter)?;
    assert_eq!(h.titles(), ["Buy milk"]);
    assert_eq!(h.app.new_title.get()?, "");
    assert_eq!(h.status(), "0 of 1 done");

    // Empty and blank titles are refused.
    h.key(KeyCode::Enter)?;
    h.type_text("   ")?;
    h.key(KeyCode::Enter)?;
    assert_eq!(h.titles(), ["Buy milk"]);

    // The Add button.
    h.ctrl_n()?;
    h.type_text("Walk the dog")?;
    let add = h.ui.find(&h.app.root, &["entry", "add"])?.ok_or("add")?;
    h.click(&add)?;
    assert_eq!(h.titles(), ["Buy milk", "Walk the dog"]);
    assert_eq!(h.status(), "0 of 2 done");
    assert_eq!(h.mounted_rows(), [task_key(1), task_key(2)]);
    Ok(())
}

#[test]
fn check_select_reorder_remove() -> Res {
    let mut h = Harness::new(three())?;
    assert_eq!(h.status(), "0 of 3 done");

    // Checking a task's box.
    let done = h.row_part(2, "done")?;
    h.click(&done)?;
    assert!(h.app.tasks.get()?[1].done);
    assert_eq!(h.status(), "1 of 3 done");
    h.click(&h.row_part(2, "done")?)?;
    assert_eq!(h.status(), "0 of 3 done");
    h.click(&h.row_part(2, "done")?)?;

    // Selecting by pointer: a press on the row outside its controls.
    let row = h.ui.find_descendant(&h.app.list, &task_key(1))?.ok_or("row")?;
    let rect = h.ui.bounds(&row)?.ok_or("bounds")?;
    let at = Point::new(rect.x() + rect.width() - 4.0, rect.y() + rect.height() / 2.0)?;
    h.click_at(at)?;
    assert_eq!(h.active(), Some(task_key(1)));

    // Arrow keys move the selection.
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::ArrowDown)?;
    assert_eq!(h.active(), Some(task_key(2)));
    h.key(KeyCode::ArrowDown)?;
    h.key(KeyCode::ArrowUp)?;
    assert_eq!(h.active(), Some(task_key(2)));

    // Alt+Up / Alt+Down move it, with the list focused.
    h.alt(KeyCode::ArrowDown)?;
    assert_eq!(h.titles(), ["Buy milk", "Write report", "Walk the dog"]);
    h.alt(KeyCode::ArrowUp)?;
    h.alt(KeyCode::ArrowUp)?;
    assert_eq!(h.titles(), ["Walk the dog", "Buy milk", "Write report"]);
    assert_eq!(h.active(), Some(task_key(2)));

    // ... and through the application shortcuts when the list is not focused
    // (a text field keeps Alt+arrows for itself, so focus a button).
    h.ui.focus(&h.button("remove")?)?;
    h.alt(KeyCode::ArrowDown)?;
    assert_eq!(h.titles(), ["Buy milk", "Walk the dog", "Write report"]);

    // The buttons.
    h.click(&h.button("down")?)?;
    assert_eq!(h.titles(), ["Buy milk", "Write report", "Walk the dog"]);
    h.click(&h.button("up")?)?;
    assert_eq!(h.titles(), ["Buy milk", "Walk the dog", "Write report"]);
    assert_eq!(h.status(), "1 of 3 done");

    // Delete removes the selected task, the next one becomes selected.
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::Delete)?;
    assert_eq!(h.titles(), ["Buy milk", "Write report"]);
    assert_eq!(h.status(), "0 of 2 done");
    assert_eq!(h.active(), Some(task_key(3)));
    assert_eq!(h.mounted_rows(), [task_key(1), task_key(3)]);

    // The Remove button.
    h.click(&h.button("remove")?)?;
    assert_eq!(h.titles(), ["Buy milk"]);
    assert_eq!(h.status(), "0 of 1 done");

    // Delete inside the text field edits the text, it removes no task.
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::ArrowDown)?;
    assert_eq!(h.active(), Some(task_key(1)));
    h.ctrl_n()?;
    h.type_text("abc")?;
    h.key(KeyCode::Home)?;
    h.key(KeyCode::Delete)?;
    assert_eq!(h.app.new_title.get()?, "bc");
    assert_eq!(h.titles(), ["Buy milk"]);
    Ok(())
}

#[test]
fn rename_in_place_commits_on_enter_and_cancels_on_escape() -> Res {
    let mut h = Harness::new(three())?;

    // Double-click the title, type, Enter.
    h.double_click(&h.row_part(2, "title")?)?;
    h.type_text("Walk the cat")?;
    assert_eq!(h.titles()[1], "Walk the dog", "a draft until committed");
    h.key(KeyCode::Enter)?;
    assert_eq!(h.titles(), ["Buy milk", "Walk the cat", "Write report"]);

    // Escape discards the draft.
    h.double_click(&h.row_part(3, "title")?)?;
    h.type_text("Something else")?;
    h.key(KeyCode::Escape)?;
    assert_eq!(h.titles(), ["Buy milk", "Walk the cat", "Write report"]);

    // An empty title is refused; Escape then restores the old one.
    h.double_click(&h.row_part(1, "title")?)?;
    h.type_text("  ")?;
    h.key(KeyCode::Enter)?;
    assert_eq!(h.titles()[0], "Buy milk");
    h.key(KeyCode::Escape)?;
    assert_eq!(h.titles()[0], "Buy milk");
    Ok(())
}

#[test]
fn filter_hides_rows_and_keeps_the_vector() -> Res {
    let mut h = Harness::new(three())?;
    let before = h.app.tasks.get()?;

    // Type into the search field, case-insensitively.
    let tasks = h.ui.find(&h.app.root, &["tasks"])?.ok_or("tasks")?;
    let field = h.ui.find_descendant(&tasks, "filter")?.ok_or("filter field")?;
    h.click(&field)?;
    h.type_text("WALK")?;
    assert_eq!(h.app.query.get()?, "WALK");
    assert_eq!(h.mounted_rows(), [task_key(2)]);
    assert_eq!(h.app.tasks.get()?, before, "filtering leaves the vector alone");

    // Down moves into the list; the active item is a shown one.
    h.key(KeyCode::ArrowDown)?;
    h.key(KeyCode::ArrowDown)?;
    assert_eq!(h.active(), Some(task_key(2)));

    // Clearing the filter shows everything again, the selection stays.
    h.click(&field)?;
    h.key(KeyCode::Escape)?;
    assert_eq!(h.app.query.get()?, "");
    assert_eq!(h.mounted_rows(), [task_key(1), task_key(2), task_key(3)]);
    assert_eq!(h.active(), Some(task_key(2)));

    // A selected task hidden by the filter is not touched by Delete or moves.
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::ArrowDown)?;
    assert_eq!(h.active(), Some(task_key(3)));
    h.click(&field)?;
    h.type_text("milk")?;
    assert_eq!(h.mounted_rows(), [task_key(1)]);
    h.key(KeyCode::Tab)?; // leave the field
    h.key(KeyCode::Delete)?;
    h.alt(KeyCode::ArrowUp)?;
    assert_eq!(h.app.tasks.get()?, before);

    // Reordering under a filter moves among the shown tasks.
    h.click(&field)?;
    h.key(KeyCode::Escape)?;
    h.type_text("w")?; // "Walk the dog", "Write report"
    assert_eq!(h.mounted_rows(), [task_key(2), task_key(3)]);
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::Home)?;
    assert_eq!(h.active(), Some(task_key(2)));
    h.alt(KeyCode::ArrowDown)?;
    assert_eq!(h.titles(), ["Buy milk", "Write report", "Walk the dog"]);
    Ok(())
}

#[test]
fn enter_on_the_selected_task_starts_a_rename() -> Res {
    let mut h = Harness::new(three())?;
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::ArrowDown)?;
    h.key(KeyCode::ArrowDown)?;
    assert_eq!(h.active(), Some(task_key(2)));
    h.key(KeyCode::Enter)?;
    h.type_text("Feed the dog")?;
    h.key(KeyCode::Enter)?;
    assert_eq!(h.titles(), ["Buy milk", "Feed the dog", "Write report"]);

    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::Enter)?;
    h.type_text("Nope")?;
    h.key(KeyCode::Escape)?;
    assert_eq!(h.titles(), ["Buy milk", "Feed the dog", "Write report"]);
    Ok(())
}

#[test]
fn a_selection_the_filter_hides_disables_remove_and_move() -> Res {
    let mut h = Harness::new(three())?;
    let before = h.app.tasks.get()?;
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::End)?;
    assert_eq!(h.active(), Some(task_key(3)));

    // Filter from the application side while the list is not focused.
    h.ui.clear_focus();
    h.app.query.set("milk".to_owned())?;
    h.flush()?;
    assert_eq!(h.mounted_rows(), [task_key(1)]);
    assert_eq!(h.active(), Some(task_key(3)), "hidden, still selected");

    // Clicking the disabled Remove button does nothing, nor do the shortcuts.
    h.click(&h.button("remove")?)?;
    h.key(KeyCode::Delete)?;
    h.alt(KeyCode::ArrowUp)?;
    assert_eq!(h.app.tasks.get()?, before);

    // Selecting a shown task enables them again.
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::Home)?;
    assert_eq!(h.active(), Some(task_key(1)));
    h.click(&h.button("remove")?)?;
    assert_eq!(h.titles(), ["Walk the dog", "Write report"]);
    assert_eq!(h.status(), "0 of 2 done");
    Ok(())
}

#[test]
fn capture_and_validation() -> Res {
    use pert_ui::cpu::CpuRenderer;
    use pert_ui::{CaptureRequest, CaptureTarget};
    let mut h = Harness::new(three())?;
    h.click(&h.row_part(2, "done")?)?;
    h.ui.focus(&h.app.list.clone())?;
    h.key(KeyCode::ArrowDown)?;
    let report = h.ui.validation_report();
    println!("{report:?}");
    h.ui.set_capture_support(true);
    let window = CaptureTarget::window(&h.ui.primary_window());
    let pending = h.ui.request_capture(CaptureRequest::new(window));
    h.flush()?;
    CpuRenderer::new().serve_captures(&mut h.ui);
    let capture = pending.wait()?.value?;
    std::fs::write(
        concat!(env!("CARGO_MANIFEST_DIR"), "/target/tasks.png"),
        capture.image.encode_png(),
    )?;
    Ok(())
}
