//! Adverse-path input ownership and modal admission through the public facade.
use pert_ui::*;
mod support;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[test]
fn modal_blocked_capture_bindings_yield_to_dialog_keys() {
    for code in [KeyCode::Escape, KeyCode::Enter, KeyCode::Space] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let dialog = DialogController::new(&ui);
        let invoked = Rc::new(Cell::new(0));
        let hit = invoked.clone();
        ui.mount_tree(Element::new(
            "dialog",
            Dialog::new(
                dialog.clone(),
                Element::new("anchor", Label::new("Owner")),
                Element::new(
                    "button",
                    Button::new("Action", move |_| {
                        hit.set(hit.get() + 1);
                        Ok(())
                    }),
                ),
            )
            .application_modal(),
        ))
        .unwrap();
        let blocked = Rc::new(Cell::new(0));
        let hit = blocked.clone();
        let _command = ui
            .register_command(
                "Captured",
                CommandScope::Capture,
                vec![KeyChord::new(CommandKey::Named(code), Modifiers::default())],
                move |_, _| {
                    hit.set(hit.get() + 1);
                    Ok(())
                },
            )
            .unwrap();
        dialog.open().unwrap();
        ui.flush(Duration::ZERO).unwrap();
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                .unwrap();
        }
        assert_eq!(blocked.get(), 0);
        if code == KeyCode::Escape {
            assert_eq!(
                dialog.status().get().unwrap(),
                DialogStatus::Finished(DialogOutcome::Cancelled)
            );
        } else {
            assert_eq!(invoked.get(), 1);
        }
    }
}

#[test]
fn direct_and_queued_text_commands_refuse_an_editor_behind_a_modal() {
    struct CountClipboard(Rc<Cell<usize>>);
    impl Clipboard for CountClipboard {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            self.0.set(self.0.get() + 1);
            Ok("paste".into())
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
    }
    struct Requester(WidgetId, Rc<RefCell<Option<WidgetId>>>);
    impl Component for Requester {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn focusable(&self) -> bool {
            true
        }
        fn keyboard_event(
            &mut self,
            cx: &mut EventContext,
            _: KeyEvent,
            _: &KeyboardState,
        ) -> Result<InputOutcome, RuntimeError> {
            cx.request_text_command(&self.0, TextCommand::PasteAfter)?;
            if let Some(target) = self.1.borrow().as_ref() {
                cx.request_text_command(target, TextCommand::PasteAfter)?;
            }
            Ok(InputOutcome::Handled)
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("Unchanged".to_owned());
    let clipboard_calls = Rc::new(Cell::new(0));
    ui.set_clipboard(CountClipboard(clipboard_calls.clone()))
        .unwrap();
    let dialog = DialogController::new(&ui);
    let owner = ui
        .mount_tree(Element::new(
            "owner",
            Dialog::new(
                dialog.clone(),
                Element::new("editor", TextField::new(value.clone())),
                Element::new(
                    "initial",
                    Column::new([Element::new("label", Label::new("Modal"))]),
                ),
            )
            .application_modal(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let editor = support::find(&ui, &owner, "editor").unwrap();
    dialog.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        ui.text_command(&editor, TextCommand::PasteAfter),
        Err(RuntimeError::ModalBlocked)
    );
    assert_eq!(value.get().unwrap(), "Unchanged");
    // Replace the modal's content with a requester targeting its blocked anchor.
    let modal = support::find(&ui, &owner, "initial").unwrap();
    let permitted = ui.signal("Allowed".to_owned());
    let target = Rc::new(RefCell::new(None));
    ui.reconcile_children(
        &modal,
        vec![
            Element::new("request", Requester(editor, target.clone())),
            Element::new("permitted", TextField::new(permitted.clone())),
        ],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    *target.borrow_mut() = Some(support::find(&ui, &modal, "permitted").unwrap());
    let request = ui.children(&modal).unwrap()[0].clone();
    ui.focus(&request).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ModalBlocked));
    assert_eq!(value.get().unwrap(), "Unchanged");
    assert_eq!(permitted.get().unwrap(), "Allowed");
    assert_eq!(clipboard_calls.get(), 0);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(value.get().unwrap(), "Unchanged");
    assert!(permitted.get().unwrap().contains("paste"));
    assert_eq!(clipboard_calls.get(), 1);
}
