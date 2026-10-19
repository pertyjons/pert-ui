//! Clipboard text from the runtime: commands and application code copy and
//! paste without a component and without moving focus.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);
impl Clipboard for Memory {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}

#[test]
fn a_command_copies_without_moving_focus() {
    let mut ui = Runtime::new();
    let memory = Memory::default();
    ui.set_clipboard(memory.clone()).unwrap();
    let name = ui.signal("draft".to_owned());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([
                Element::new("field", TextField::new(&name)),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    ui.focus(&field).unwrap();
    let copy = ui
        .register_command("Copy", CommandScope::Application, vec![], |ui, _| {
            ui.set_clipboard_text("copied")
        })
        .unwrap();
    let context = ui.command_context(CommandSource::Programmatic);
    assert_eq!(
        ui.invoke_command(&copy, context).unwrap(),
        CommandOutcome::Invoked
    );
    assert_eq!(*memory.0.borrow(), "copied");
    assert_eq!(ui.focused(), Some(field), "focus stays where it was");
    assert_eq!(ui.clipboard_text().unwrap(), "copied");
}

#[test]
fn without_a_service_or_after_shutdown_the_clipboard_is_refused() {
    let mut ui = Runtime::new();
    assert!(matches!(
        ui.set_clipboard_text("x"),
        Err(RuntimeError::Clipboard(ClipboardError::Unavailable))
    ));
    assert!(matches!(
        ui.clipboard_text(),
        Err(RuntimeError::Clipboard(ClipboardError::Unavailable))
    ));
    ui.set_clipboard(Memory::default()).unwrap();
    ui.shutdown();
    assert_eq!(ui.set_clipboard_text("x"), Err(RuntimeError::Closed));
}

/// A service that fails both ways.
struct Busy;
impl Clipboard for Busy {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Err(ClipboardError::Busy)
    }
    fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
        Err(ClipboardError::Busy)
    }
}

#[test]
fn a_failing_service_reports_its_own_error() {
    let mut ui = Runtime::new();
    ui.set_clipboard(Busy).unwrap();
    assert!(matches!(
        ui.clipboard_text(),
        Err(RuntimeError::Clipboard(ClipboardError::Busy))
    ));
    assert!(matches!(
        ui.set_clipboard_text("x"),
        Err(RuntimeError::Clipboard(ClipboardError::Busy))
    ));
}
