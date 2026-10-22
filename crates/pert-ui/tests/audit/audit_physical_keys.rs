//! Distinct native physical ownership and press-time control meanings.
use pert_ui::*;
use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

const NUMPAD: KeyCode = KeyCode::Native(200);

fn input(physical: KeyCode, action: KeyAction, logical: CommandKey) -> CommandKeyInput {
    CommandKeyInput {
        physical,
        action,
        base: Some(logical),
        logical: Some(logical),
        has_text: matches!(logical, CommandKey::Character(_)),
        composing: false,
    }
}

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}

#[test]
fn main_and_numpad_enter_have_distinct_matching_release_ownership() {
    for first in [KeyCode::Enter, NUMPAD] {
        let mut ui = runtime();
        let hits = Rc::new(Cell::new(0));
        let seen = hits.clone();
        let button = ui
            .mount_tree(Element::new(
                "button",
                Button::new("Act", move |_| {
                    seen.set(seen.get() + 1);
                    Ok(())
                }),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.focus(&button).unwrap();
        let second = if first == NUMPAD {
            KeyCode::Enter
        } else {
            NUMPAD
        };
        for code in [first, second] {
            ui.command_key_input(input(
                code,
                KeyAction::Press,
                CommandKey::Named(KeyCode::Enter),
            ))
            .unwrap();
        }
        assert!(ui.keyboard().is_pressed(first));
        assert!(ui.keyboard().is_pressed(second));
        ui.command_key_input(input(
            second,
            KeyAction::Release,
            CommandKey::Named(KeyCode::Enter),
        ))
        .unwrap();
        assert_eq!(hits.get(), 0);
        assert!(ui.keyboard().is_pressed(first));
        ui.command_key_input(input(
            first,
            KeyAction::Release,
            CommandKey::Named(KeyCode::Enter),
        ))
        .unwrap();
        assert_eq!(hits.get(), 1);
        assert!(!ui.keyboard().is_pressed(first));
        assert!(!ui.keyboard().is_pressed(second));
    }
}

type SeenKeys = Rc<RefCell<Vec<(KeyEvent, KeyCode)>>>;

struct Observe(SeenKeys);
impl Component for Observe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 30.0)?)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.borrow_mut().push((event, keys.control_key(event)));
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn numlock_changes_do_not_retarget_a_held_native_key() {
    for first in [
        CommandKey::Named(KeyCode::ArrowLeft),
        CommandKey::Character('4'),
    ] {
        let mut ui = runtime();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let id = ui
            .mount_tree(Element::new("observer", Observe(seen.clone())))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.focus(&id).unwrap();
        let next = if matches!(first, CommandKey::Named(_)) {
            CommandKey::Character('4')
        } else {
            CommandKey::Named(KeyCode::ArrowLeft)
        };
        for (action, logical) in [
            (KeyAction::Press, first),
            (KeyAction::Repeat, next),
            (KeyAction::Release, next),
        ] {
            ui.command_key_input(input(NUMPAD, action, logical))
                .unwrap();
        }
        let expected = match first {
            CommandKey::Named(code) => code,
            _ => NUMPAD,
        };
        assert_eq!(
            *seen.borrow(),
            [
                (
                    KeyEvent {
                        code: NUMPAD,
                        transition: KeyTransition::Pressed
                    },
                    expected
                ),
                (
                    KeyEvent {
                        code: NUMPAD,
                        transition: KeyTransition::Repeated
                    },
                    expected
                ),
                (
                    KeyEvent {
                        code: NUMPAD,
                        transition: KeyTransition::Released
                    },
                    expected
                ),
            ]
        );
        assert!(!ui.keyboard().is_pressed(NUMPAD));
        ui.command_key_input(input(NUMPAD, KeyAction::Press, next))
            .unwrap();
        let expected = match next {
            CommandKey::Named(code) => code,
            _ => NUMPAD,
        };
        assert_eq!(seen.borrow().last().unwrap().1, expected);
        ui.clear_focus();
        assert!(!ui.keyboard().is_pressed(NUMPAD));
    }
}

#[test]
fn text_editors_keep_the_press_time_named_navigation_under_numlock_changes() {
    for starts_named in [false, true] {
        let mut ui = runtime();
        let text = ui.signal("ab".to_owned());
        let field = ui
            .mount_tree(Element::new("field", TextField::new(text.clone())))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.focus(&field).unwrap();
        ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
        let named = CommandKey::Named(KeyCode::ArrowLeft);
        let digit = CommandKey::Character('4');
        let first = if starts_named { named } else { digit };
        let later = if starts_named { digit } else { named };
        ui.command_key_input(input(NUMPAD, KeyAction::Press, first))
            .unwrap();
        let repeated = ui
            .command_key_input(input(NUMPAD, KeyAction::Repeat, later))
            .unwrap();
        assert!(!repeated.deliver_text);
        ui.command_key_input(input(NUMPAD, KeyAction::Release, later))
            .unwrap();
        let properties = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "field")
            .unwrap()
            .details
            .properties;
        let expected = if starts_named { "0..0" } else { "2..2" };
        assert_eq!(
            properties.get("selection"),
            Some(&InspectValue::Text(expected.into()))
        );
        assert_eq!(text.get().unwrap(), "ab");
    }
}

#[test]
fn inline_parameter_steps_commit_only_the_physical_owner_after_numlock_changes() {
    let mut ui = runtime();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "count").unwrap(),
            ParameterSpec::stepped("Count", 0, 10, 0).unwrap(),
        )
        .unwrap();
    let row = ui
        .mount_tree(Element::new(
            "number",
            InlineInteger::for_parameter(parameter.clone()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&row).unwrap();
    let up = CommandKey::Named(KeyCode::ArrowUp);
    ui.command_key_input(input(NUMPAD, KeyAction::Press, up))
        .unwrap();
    ui.command_key_input(input(KeyCode::ArrowUp, KeyAction::Press, up))
        .unwrap();
    ui.command_key_input(input(KeyCode::ArrowUp, KeyAction::Release, up))
        .unwrap();
    assert_eq!(parameter.snapshot().get().unwrap().base, 0);
    assert_eq!(parameter.snapshot().get().unwrap().preview, Some(1));
    ui.command_key_input(input(NUMPAD, KeyAction::Repeat, CommandKey::Character('8')))
        .unwrap();
    assert_eq!(parameter.snapshot().get().unwrap().preview, Some(2));
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Release,
        CommandKey::Character('8'),
    ))
    .unwrap();
    assert_eq!(parameter.snapshot().get().unwrap().base, 2);
    assert_eq!(parameter.snapshot().get().unwrap().preview, None);
    ui.command_key_input(input(NUMPAD, KeyAction::Press, up))
        .unwrap();
    assert_eq!(parameter.snapshot().get().unwrap().preview, Some(3));
    ui.clear_focus();
    assert_eq!(parameter.snapshot().get().unwrap().base, 2);
    assert_eq!(parameter.snapshot().get().unwrap().preview, None);
}

#[test]
fn overlapping_native_resize_keys_keep_the_original_until_every_physical_release() {
    let mut ui = runtime();
    let width = ui.signal(Length::new(100.0).unwrap());
    let grid = ui
        .mount_tree(Element::new(
            "grid",
            PropertyGrid::new([Property::new(
                "value",
                "Value",
                Element::new("editor", Label::new("Content")),
            )])
            .bind_label_width(width.clone()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&grid).unwrap();
    let right = CommandKey::Named(KeyCode::ArrowRight);
    for code in [KeyCode::ArrowRight, NUMPAD] {
        ui.command_key_input(input(code, KeyAction::Press, right))
            .unwrap();
    }
    ui.command_key_input(input(NUMPAD, KeyAction::Release, right))
        .unwrap();
    assert_eq!(width.get().unwrap(), Length::new(100.0).unwrap());
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(width.get().unwrap(), Length::new(100.0).unwrap());
}

#[test]
fn an_inline_numeric_draft_accepts_native_numpad_enter() {
    let mut ui = runtime();
    let value = ui.signal(1_i64);
    let field = ui
        .mount_tree(Element::new("field", InlineInteger::new(value.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&field).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::End, KeyAction::Release).unwrap();
    ui.key_input(KeyCode::Backspace, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Backspace, KeyAction::Release)
        .unwrap();
    ui.text_input("7").unwrap();
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Press,
        CommandKey::Named(KeyCode::Enter),
    ))
    .unwrap();
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Release,
        CommandKey::Named(KeyCode::Enter),
    ))
    .unwrap();
    assert_eq!(value.get().unwrap(), 7);
}

#[test]
fn captured_named_navigation_suppresses_changed_printable_text() {
    let mut ui = runtime();
    let text = ui.signal("ab".to_owned());
    let field = ui
        .mount_tree(Element::new("field", TextField::new(text.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&field).unwrap();
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Press,
        CommandKey::Named(KeyCode::ArrowUp),
    ))
    .unwrap();
    let dispatch = ui
        .command_key_input(input(NUMPAD, KeyAction::Repeat, CommandKey::Character('8')))
        .unwrap();
    assert!(!dispatch.deliver_text);
    assert_eq!(text.get().unwrap(), "ab");
}

#[test]
fn unreported_search_navigation_cannot_leak_late_printable_key_text() {
    let mut ui = runtime();
    let query = ui.signal(String::new());
    let field = ui
        .mount_tree(Element::new("search", SearchField::new(query.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&field).unwrap();
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Press,
        CommandKey::Named(KeyCode::ArrowUp),
    ))
    .unwrap();
    let dispatch = ui
        .command_key_input(input(NUMPAD, KeyAction::Repeat, CommandKey::Character('8')))
        .unwrap();
    if dispatch.deliver_text {
        ui.text_input("8").unwrap();
    }
    assert!(!dispatch.deliver_text);
    assert_eq!(query.get().unwrap(), "");
}

struct DraftOwner {
    draft: DraftEditorController<()>,
    editor: Option<WidgetId>,
    children: Vec<Element>,
}

impl Component for DraftOwner {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.editor = cx.children().first().map(|child| child.id().clone());
        Ok(())
    }
    fn focusable(&self) -> bool {
        !self.draft.is_active()
    }
    fn child_visible(&self, _: &str) -> bool {
        self.draft.is_active()
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(240.0, 30.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 0.0, size.width(), size.height())?)?;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code != KeyCode::Enter || event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        self.draft.begin(cx, "draft", None, DraftSelection::End)?;
        cx.request_focus_for(self.editor.as_ref().unwrap())?;
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn unreported_draft_navigation_cannot_leak_late_printable_key_text() {
    let mut ui = runtime();
    let draft = DraftEditorController::new();
    let owner = ui
        .mount_tree(Element::new(
            "owner",
            DraftOwner {
                draft: draft.clone(),
                editor: None,
                children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
            },
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&owner).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(draft.is_active());
    ui.command_key_input(input(
        NUMPAD,
        KeyAction::Press,
        CommandKey::Named(KeyCode::ArrowUp),
    ))
    .unwrap();
    let dispatch = ui
        .command_key_input(input(NUMPAD, KeyAction::Repeat, CommandKey::Character('8')))
        .unwrap();
    if dispatch.deliver_text {
        ui.text_input("8").unwrap();
    }
    assert!(!dispatch.deliver_text);
    assert_eq!(draft.text(), "draft");
}

#[test]
fn unchanged_native_named_space_keeps_its_printable_text() {
    let mut ui = runtime();
    let query = ui.signal(String::new());
    let field = ui
        .mount_tree(Element::new("search", SearchField::new(query.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&field).unwrap();
    for action in [KeyAction::Press, KeyAction::Repeat] {
        let mut key = input(NUMPAD, action, CommandKey::Named(KeyCode::Space));
        key.has_text = true;
        let dispatch = ui.command_key_input(key).unwrap();
        assert!(dispatch.deliver_text);
        ui.text_input(" ").unwrap();
    }
    assert_eq!(query.get().unwrap(), "  ");
}
