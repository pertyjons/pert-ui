//! Public consumer contracts for local editors and native text extension points.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{find, frame};

fn setup(text: &str, multi: bool) -> (Runtime, WidgetId, Signal<String>) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let element = if multi {
        Element::new("field", TextArea::new(value.clone()))
    } else {
        Element::new("field", TextField::new(value.clone()))
    };
    let id = ui.mount_tree(element).unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    (ui, id, value)
}
fn key(ui: &mut Runtime, key: KeyCode) {
    support::key(ui, key);
    frame(ui);
}
fn command(ui: &mut Runtime, key_code: KeyCode) {
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, key_code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
}
// Painted scrollbar thumbs, resting or emphasized: the only four-pixel-wide
// fills in the thumb roles an editor draws. Field frames stroke the border
// width in Outline, the focus ring roles or Error instead.
fn thumbs(ui: &mut Runtime) -> usize {
    let palette = Palette::dark();
    let roles = [
        palette.color(ColorRole::ScrollbarThumb),
        palette.color(ColorRole::ScrollbarThumbHover),
    ];
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                DrawCommand::FillRect { rect, color }
                    if rect.width() == 4.0 && roles.contains(color)
            )
        })
        .count()
}
fn scroll(ui: &Runtime, axis: &str) -> f64 {
    match property(ui, axis) {
        InspectValue::Number(value) => value,
        other => panic!("unexpected scroll offset {other:?}"),
    }
}
fn property(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .properties[key]
        .clone()
}
fn click(ui: &mut Runtime, x: f32, y: f32, time: u64) {
    ui.flush(Duration::from_millis(time)).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
    }
    frame(ui);
}
#[derive(Clone, Default)]
struct MemoryClipboard(Rc<RefCell<String>>);
impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}

#[test]
fn text_binding_multiline_defaults_enter_and_undo() {
    let (mut ui, _, value) = setup("", true);
    assert_eq!(
        property(&ui, "typography"),
        InspectValue::Text("Monospace".into())
    );
    ui.text_input("Åäö 👩‍💻").unwrap();
    key(&mut ui, KeyCode::Enter);
    ui.text_input("next").unwrap();
    assert_eq!(value.get().unwrap(), "Åäö 👩‍💻\nnext");
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "Åäö 👩‍💻\n");
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    command(&mut ui, KeyCode::KeyZ);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(value.get().unwrap(), "Åäö 👩‍💻\nnext");
    let (mut ui, _, value) = setup("", false);
    assert_eq!(
        property(&ui, "typography"),
        InspectValue::Text("Body".into())
    );
    ui.text_input("first\r\nsecond\tthird").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "first second third");
}

#[test]
fn tall_single_line_field_centers_text_like_a_button() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 120.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new(
                "field-box",
                Container::new(Element::new(
                    "field",
                    TextField::new(ui.signal("Same".to_owned())),
                ))
                .preferred_size(Size::from([240, 64])),
            ),
            Element::new(
                "button-box",
                Container::new(Element::new("button", Button::new("Same", |_| Ok(()))))
                    .preferred_size(Size::from([120, 64])),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let theme = ui.theme();
    let palette = theme.palette(theme.mode().unwrap()).unwrap();
    let foreground = palette.color(ColorRole::OnSurface);
    let button_foreground = palette.color(ColorRole::OnButton);
    let scene = ui.paint().unwrap();
    let text_top = |color| {
        scene.commands().iter().find_map(|command| match command {
            DrawCommand::DrawMask {
                rect, color: tint, ..
            } if *tint == color => Some(rect.y()),
            _ => None,
        })
    };
    let field_y = text_top(foreground).unwrap();
    let button_y = text_top(button_foreground).unwrap();
    assert!((field_y - button_y).abs() < 0.01, "{field_y} != {button_y}");
}
#[test]
fn double_click_selects_word_and_triple_click_selects_all_in_both_fields() {
    for multi in [false, true] {
        let (mut ui, _, value) = setup("hello world", multi);
        // Both bundled fonts place this point within the first word.
        click(&mut ui, 25.0, 15.0, 10);
        click(&mut ui, 25.0, 15.0, 100);
        assert_eq!(
            property(&ui, "selection"),
            InspectValue::Text("0..5".into())
        );
        click(&mut ui, 25.0, 15.0, 200);
        assert_eq!(
            property(&ui, "selection"),
            InspectValue::Text("0..11".into())
        );
        ui.text_input("replacement").unwrap();
        assert_eq!(value.get().unwrap(), "replacement");
    }
}
#[test]
fn triple_click_selects_all_lines_and_click_sequence_expires() {
    let (mut ui, _, value) = setup("hello world\nsecond line", true);
    click(&mut ui, 25.0, 15.0, 10);
    click(&mut ui, 25.0, 15.0, 100);
    click(&mut ui, 25.0, 15.0, 200);
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text(format!("0..{}", value.get().unwrap().len()))
    );
    click(&mut ui, 25.0, 15.0, 800);
    assert_ne!(
        property(&ui, "selection"),
        InspectValue::Text("0..5".into())
    );
}
#[test]
fn a_drifting_series_counts_from_the_previous_click_and_a_fourth_starts_over() {
    for multi in [false, true] {
        let (mut ui, _, value) = setup("hello world", multi);
        // Each click is three pixels from the previous one: still a series.
        click(&mut ui, 22.0, 15.0, 10);
        click(&mut ui, 25.0, 15.0, 100);
        click(&mut ui, 28.0, 15.0, 200);
        assert_eq!(
            property(&ui, "selection"),
            InspectValue::Text(format!("0..{}", value.get().unwrap().len()))
        );
        // A fourth click is a single click again: the selection is what an
        // isolated click at the same place makes.
        click(&mut ui, 28.0, 15.0, 300);
        let fourth = property(&ui, "selection");
        click(&mut ui, 28.0, 15.0, 5000);
        assert_eq!(property(&ui, "selection"), fourth);
        // Five pixels away a second click is no double click.
        click(&mut ui, 25.0, 15.0, 6000);
        click(&mut ui, 30.0, 15.0, 6100);
        assert_ne!(
            property(&ui, "selection"),
            InspectValue::Text("0..5".into())
        );
    }
}
#[test]
fn clipboard_cut_paste_normalization_and_failure_are_atomic() {
    let (mut ui, _, value) = setup("original", false);
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    command(&mut ui, KeyCode::KeyA);
    command(&mut ui, KeyCode::KeyX);
    assert_eq!(&*clipboard.0.borrow(), "original");
    assert_eq!(value.get().unwrap(), "");
    *clipboard.0.borrow_mut() = "first\r\nsecond".into();
    command(&mut ui, KeyCode::KeyV);
    assert_eq!(value.get().unwrap(), "first second");
    let (mut ui, _, value) = setup("preserve", false);
    command(&mut ui, KeyCode::KeyA);
    ui.key_input(KeyCode::primary(), KeyAction::Press).unwrap();
    assert_eq!(
        ui.key_input(KeyCode::KeyX, KeyAction::Press),
        Err(RuntimeError::Clipboard(ClipboardError::Unavailable))
    );
    assert_eq!(value.get().unwrap(), "preserve");
}

#[test]
fn pasting_an_empty_or_non_text_clipboard_is_a_noop() {
    struct Empty;
    impl Clipboard for Empty {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Err(ClipboardError::NoText)
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }
    let (mut ui, _, value) = setup("preserve", false);
    ui.set_clipboard(Empty).unwrap();
    command(&mut ui, KeyCode::KeyA);
    command(&mut ui, KeyCode::KeyV);
    assert_eq!(value.get().unwrap(), "preserve");
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text("0..8".into())
    );
}

#[test]
fn runtime_shutdown_releases_clipboard_even_when_ui_handles_survive() {
    struct OwnedClipboard {
        drops: Rc<std::cell::Cell<usize>>,
        _value: Signal<String>,
    }
    impl Drop for OwnedClipboard {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }
    impl Clipboard for OwnedClipboard {
        fn read_text(&mut self) -> Result<String, ClipboardError> {
            Ok(String::new())
        }
        fn write_text(&mut self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }
    let mut ui = Runtime::new();
    let value = ui.signal(String::new());
    let drops = Rc::new(std::cell::Cell::new(0));
    for _ in 0..2 {
        ui.set_clipboard(OwnedClipboard {
            drops: drops.clone(),
            _value: value.clone(),
        })
        .unwrap();
    }
    assert_eq!(drops.get(), 1);
    drop(ui);
    assert_eq!(drops.get(), 2);
    assert_eq!(value.get(), Err(RuntimeError::Closed));
}
#[test]
fn composition_is_temporary_and_context_loss_preserves_committed_text() {
    let (mut ui, id, value) = setup("original", true);
    command(&mut ui, KeyCode::KeyA);
    ui.text_preedit(&TextPreedit::new("åä", Some(2..4)).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "original");
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(true));
    ui.text_input("ö").unwrap();
    assert_eq!(value.get().unwrap(), "ö");
    ui.text_preedit(&TextPreedit::new("pending", Some(7..7)).unwrap())
        .unwrap();
    ui.set_visible(&id, false).unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "ö");
    assert!(ui.next_deadline().is_none());
    ui.set_visible(&id, true).unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    ui.text_preedit(&TextPreedit::new("pending", None).unwrap())
        .unwrap();
    key(&mut ui, KeyCode::Escape);
    assert_eq!(value.get().unwrap(), "ö");
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
}
#[test]
fn external_write_precedes_the_next_event_and_clears_history() {
    let (mut ui, _, value) = setup("old", false);
    ui.text_input("x").unwrap();
    ui.text_preedit(&TextPreedit::new("pending", None).unwrap())
        .unwrap();
    value.set("external".into()).unwrap();
    ui.text_input("new ").unwrap();
    assert_eq!(value.get().unwrap(), "new external");
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "external");
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "external");
}
#[test]
fn combining_and_emoji_deletion_preserve_utf8() {
    let (mut ui, _, value) = setup("a", true);
    command(&mut ui, KeyCode::KeyA);
    ui.text_input("e\u{301}👩‍💻").unwrap();
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(value.get().unwrap(), "e\u{301}");
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(value.get().unwrap(), "");
}
#[test]
fn invalid_edits_and_preedit_ranges_do_not_corrupt_binding() {
    assert_eq!(
        TextPreedit::new("å", Some(1..2)),
        Err(TextEditError::InvalidComposition)
    );
    assert_eq!(
        TextPreedit::new("ab", Some(std::ops::Range { start: 2, end: 1 })),
        Err(TextEditError::InvalidComposition)
    );
    let (mut ui, _, value) = setup("preserve", false);
    assert_eq!(
        ui.text_input(&"x".repeat(TextEditError::MAX_TEXT_BYTES + 1)),
        Ok(())
    );
    assert_eq!(value.get().unwrap(), "preserve");
    assert_eq!(
        ui.text_input("\0"),
        Err(RuntimeError::TextEdit(TextEditError::InvalidText))
    );
    value.set("bad\nexternal".into()).unwrap();
    assert!(ui.flush(Duration::ZERO).is_ok());
    assert_eq!(value.get().unwrap(), "bad\nexternal");
}
#[test]
fn a_full_field_refuses_more_text_without_failing() {
    for multi in [false, true] {
        // Zero-width spaces fill the byte limit without exceeding the text
        // service's raster limits, so the full field still prepares and paints.
        let full = "\u{200B}".repeat(TextEditError::MAX_TEXT_BYTES / 3)
            + &"x".repeat(TextEditError::MAX_TEXT_BYTES % 3);
        assert_eq!(full.len(), TextEditError::MAX_TEXT_BYTES);
        let (mut ui, id, value) = setup(&full, multi);
        let clipboard = MemoryClipboard::default();
        ui.set_clipboard(clipboard.clone()).unwrap();
        // Typing, an IME commit and a composition that would not fit are
        // refused like an editor at its limit, not reported as a failure.
        assert_eq!(ui.text_input("y"), Ok(()));
        assert_eq!(
            ui.text_preedit(&TextPreedit::new("y", None).unwrap()),
            Ok(())
        );
        assert_eq!(ui.text_input("y"), Ok(()));
        *clipboard.0.borrow_mut() = "pasted".into();
        for command in [
            TextCommand::Paste,
            TextCommand::PasteBefore,
            TextCommand::PasteAfter,
        ] {
            assert_eq!(
                ui.text_command(&id, command),
                Ok(TextCommandOutcome::Unavailable),
                "{command:?}"
            );
        }
        frame(&mut ui);
        assert_eq!(value.get().unwrap(), full);
        // Room made by a deletion accepts text again up to the limit.
        key(&mut ui, KeyCode::End);
        key(&mut ui, KeyCode::Backspace);
        assert_eq!(ui.text_input("y"), Ok(()));
        frame(&mut ui);
        let text = value.get().unwrap();
        assert_eq!(text.len(), TextEditError::MAX_TEXT_BYTES);
        assert!(text.ends_with('y'), "the edit went in at the caret");
    }
}
#[test]
fn blink_stops_on_focus_loss() {
    let (mut ui, id, _) = setup("hello", true);
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(500)));
    ui.flush(Duration::from_millis(500)).unwrap();
    ui.paint().unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(1000)));
    ui.text_input("new ").unwrap();
    ui.flush(Duration::from_millis(500)).unwrap();
    ui.clear_focus();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
    // A field owns a context-menu surface; its teardown finishes inside `unmount`.
    ui.unmount(&id).unwrap();
    assert!(ui.next_deadline().is_none());
}
#[test]
fn readonly_copies_and_disabled_skips_automatic_focus() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("read only".to_owned());
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("disabled", TextField::new(value.clone()).disabled()),
                Element::new("field", TextArea::new(value.clone()).read_only()),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    ui.focus_next(false).unwrap();
    frame(&mut ui);
    assert_eq!(ui.focused(), Some(ui.children(&root).unwrap()[1].clone()));
    command(&mut ui, KeyCode::KeyA);
    command(&mut ui, KeyCode::KeyC);
    ui.text_input("ignored").unwrap();
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(&*clipboard.0.borrow(), "read only");
    assert_eq!(value.get().unwrap(), "read only");
    assert!(ui.next_deadline().is_none());
    assert!(ui.text_input_state().is_none());
}

#[test]
fn resize_before_paint_uses_current_wrapping_and_hit_geometry() {
    for movement in [false, true] {
        let mut selections = Vec::new();
        for paint_first in [false, true] {
            let (mut ui, _, _) = setup("one two three four five six seven eight nine ten", true);
            ui.set_viewport(Viewport::new(120.0, 160.0, 1.25).unwrap())
                .unwrap();
            ui.theme().set_density(Density::Comfortable).unwrap();
            ui.flush(Duration::ZERO).unwrap();
            if paint_first {
                ui.paint().unwrap();
            }
            if movement {
                key(&mut ui, KeyCode::ArrowDown);
            } else {
                click(&mut ui, 35.0, 45.0, 100);
            }
            selections.push(property(&ui, "selection"));
        }
        assert_eq!(selections[0], selections[1]);
    }
}

#[test]
fn pointer_drag_replaces_a_range_and_focus_transfer_cancels_preedit() {
    let (mut ui, _, value) = setup("hello world", false);
    for (x, kind) in [
        (8.0, PointerKind::Press(PointerButton::Primary)),
        (45.0, PointerKind::Move),
        (45.0, PointerKind::Release(PointerButton::Primary)),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, 15.0).unwrap(), kind))
            .unwrap();
    }
    assert_ne!(
        property(&ui, "selection"),
        InspectValue::Text("0..0".into())
    );
    ui.text_input("X").unwrap();
    assert!(value.get().unwrap().starts_with('X'));
    ui.text_preedit(&TextPreedit::new("pending", None).unwrap())
        .unwrap();
    let button = ui
        .mount("button", Button::new("Other", |_| Ok(())))
        .unwrap();
    ui.focus(&button).unwrap();
    frame(&mut ui);
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    assert!(!value.get().unwrap().contains("pending"));
}

#[test]
fn undo_is_bounded_and_edits_do_not_remeasure_fixed_controls() {
    let (mut ui, _, value) = setup("", true);
    ui.set_profiling(true).unwrap();
    ui.clear_profile().unwrap();
    for _ in 0..40 {
        ui.text_input("x").unwrap();
        frame(&mut ui);
    }
    assert!(ui.profile().stats.iter().all(|s| !matches!(
        s.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    for _ in 0..40 {
        command(&mut ui, KeyCode::KeyZ);
    }
    assert_eq!(value.get().unwrap(), "xxxxxxxx");
}

#[test]
fn caret_reveal_scrolls_long_multiline_text_and_wrapped_navigation_remains_valid() {
    let (mut ui, _, value) = setup(&"line with words\n".repeat(40), true);
    command(&mut ui, KeyCode::End);
    let (_, state) = ui.text_input_state().unwrap();
    let [_, y, _, h] = state.caret.values();
    assert!(y >= 0.0 && y + h <= 160.0);
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Home);
    ui.text_input("X").unwrap();
    assert!(value.get().unwrap().ends_with("Xline with words\n"));
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 40.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, -60.0).unwrap()),
    ))
    .unwrap();
    frame(&mut ui);
    click(&mut ui, 25.0, 40.0, 100);
    ui.text_input("Y").unwrap();
    assert!(value.get().unwrap().contains('Y'));
}

struct EscapeParent(Rc<std::cell::Cell<usize>>);
impl Component for EscapeParent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(size))
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Escape && event.transition == KeyTransition::Pressed {
            self.0.set(self.0.get() + 1);
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}
#[test]
fn escape_bubbles_only_after_text_selection_composition_and_drag_are_canceled() {
    for multi in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
            .unwrap();
        let escapes = Rc::new(std::cell::Cell::new(0));
        let value = ui.signal("example text".to_owned());
        let child = if multi {
            Element::new("field", TextArea::new(value.clone()))
        } else {
            Element::new("field", TextField::new(value.clone()))
        };
        let root = ui
            .mount_tree(Element::new("parent", EscapeParent(escapes.clone())).children([child]))
            .unwrap();
        ui.focus(&ui.children(&root).unwrap()[0]).unwrap();
        frame(&mut ui);
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 1);
        command(&mut ui, KeyCode::KeyA);
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 1);
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 2);
        ui.text_preedit(&TextPreedit::new("temporary", Some(0..9)).unwrap())
            .unwrap();
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 2);
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 3);
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 20.0).unwrap(),
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 3);
        ui.pointer_input(PointerEvent::new(
            Point::new(20.0, 20.0).unwrap(),
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
        key(&mut ui, KeyCode::Escape);
        assert_eq!(escapes.get(), 4);
        assert_eq!(value.get().unwrap(), "example text");
    }
}

#[test]
fn numeric_escape_restores_draft_and_remains_consumed() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let escapes = Rc::new(std::cell::Cell::new(0));
    let value = ui.signal(1_i64);
    let root = ui
        .mount_tree(
            Element::new("parent", EscapeParent(escapes.clone()))
                .children([Element::new("field", IntegerInput::new(value.clone()))]),
        )
        .unwrap();
    ui.focus(&ui.children(&root).unwrap()[0]).unwrap();
    ui.text_input("2").unwrap();
    key(&mut ui, KeyCode::Escape);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 1);
    assert_eq!(escapes.get(), 0);
}

#[test]
fn multiline_scrollbar_appears_on_overflow_and_drags_without_moving_the_caret() {
    let (mut ui, id, value) = setup(&"line with words\n".repeat(40), true);
    ui.set_profiling(true).unwrap();
    let [_, _, width, _] = ui.bounds(&id).unwrap().unwrap().values();
    let metrics = ThemeMetrics::default();
    // Inside the eight-pixel track strip along the text viewport's right edge.
    let lane = width - metrics.get(MetricRole::ControlPaddingX) - 2.0;
    let top = metrics.get(MetricRole::ControlPaddingY);
    let send = |ui: &mut Runtime, y: f32, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(lane, y).unwrap(), kind))
            .unwrap();
        frame(ui);
    };
    assert_eq!(thumbs(&mut ui), 1, "overflowing text shows one thumb");
    assert_eq!(scroll(&ui, "scroll_y"), 0.0);
    send(
        &mut ui,
        top + 4.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    ui.clear_profile().unwrap();
    send(&mut ui, top + 4.0, PointerKind::Move);
    assert_eq!(scroll(&ui, "scroll_y"), 0.0, "thumb must not jump on grab");
    send(&mut ui, top + 64.0, PointerKind::Move);
    let middle = scroll(&ui, "scroll_y");
    assert!(middle > 0.0);
    assert!(ui.profile().stats.iter().all(|stat| !matches!(
        stat.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    send(&mut ui, 4_000.0, PointerKind::Move);
    let bottom = scroll(&ui, "scroll_y");
    assert!(bottom > middle);
    send(
        &mut ui,
        4_000.0,
        PointerKind::Release(PointerButton::Primary),
    );
    send(&mut ui, top + 4.0, PointerKind::Move);
    assert_eq!(scroll(&ui, "scroll_y"), bottom, "release ends the drag");
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text("0..0".into()),
        "scrolling must not move the caret"
    );
    // The thumb's travel ends exactly where revealing the last line ends.
    command(&mut ui, KeyCode::End);
    assert_eq!(scroll(&ui, "scroll_y"), bottom);
    // A track click centers the thumb there and starts a drag Escape keeps.
    send(
        &mut ui,
        top + 20.0,
        PointerKind::Press(PointerButton::Primary),
    );
    let clicked = scroll(&ui, "scroll_y");
    assert!(clicked > 0.0 && clicked < bottom);
    key(&mut ui, KeyCode::Escape);
    send(&mut ui, 4_000.0, PointerKind::Move);
    assert_eq!(
        scroll(&ui, "scroll_y"),
        clicked,
        "Escape cancels at the current offset"
    );
    send(
        &mut ui,
        4_000.0,
        PointerKind::Release(PointerButton::Primary),
    );
    // The I-beam returns over the text itself.
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 40.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    frame(&mut ui);
    assert_eq!(ui.cursor_icon(), CursorIcon::Text);
    // Text that fits again removes the thumb and the retained offset.
    value.set("one line".to_owned()).unwrap();
    frame(&mut ui);
    assert_eq!(thumbs(&mut ui), 0);
    assert_eq!(scroll(&ui, "scroll_y"), 0.0);
}

#[test]
fn single_line_fields_scroll_horizontally_without_a_scrollbar() {
    let (mut ui, _, _) = setup(&"long text ".repeat(40), false);
    command(&mut ui, KeyCode::End);
    frame(&mut ui);
    assert!(scroll(&ui, "scroll_x") > 0.0, "the caret stays revealed");
    assert_eq!(
        thumbs(&mut ui),
        0,
        "only multiline editors show a scrollbar"
    );
}

#[test]
fn read_only_multiline_fields_scroll_while_disabled_ones_show_no_scrollbar() {
    for (disabled, expected) in [(false, 1), (true, 0)] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
            .unwrap();
        let value = ui.signal("line with words\n".repeat(40));
        let area = TextArea::new(value);
        let area = if disabled {
            area.disabled()
        } else {
            area.read_only()
        };
        ui.mount_tree(Element::new("field", area)).unwrap();
        frame(&mut ui);
        assert_eq!(thumbs(&mut ui), expected);
    }
}

/// Places the caret without a selection, so spaced pastes see a real word target.
/// A freshly mounted field starts with an empty selection at offset zero.
fn caret(ui: &mut Runtime, at: usize) {
    for _ in 0..at {
        key(ui, KeyCode::ArrowRight);
    }
}

#[test]
fn the_text_command_contract_copies_cuts_and_pastes_like_its_chords() {
    let (mut ui, id, value) = setup("original", false);
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    command(&mut ui, KeyCode::KeyA);
    assert_eq!(
        ui.text_command(&id, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(&*clipboard.0.borrow(), "original");
    assert_eq!(value.get().unwrap(), "original");
    assert_eq!(
        ui.text_command(&id, TextCommand::Cut).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(value.get().unwrap(), "");
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(value.get().unwrap(), "original");
    // One undo step per command, so the menu and the chords share history.
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "");
}

#[test]
fn a_command_that_does_not_apply_is_unavailable_rather_than_an_error() {
    let (mut ui, id, value) = setup("keep", false);
    ui.set_clipboard(MemoryClipboard::default()).unwrap();
    // Nothing selected, and an empty clipboard.
    assert_eq!(
        ui.text_command(&id, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Unavailable
    );
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Unavailable
    );
    assert_eq!(value.get().unwrap(), "keep");

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let read_only = ui.signal(String::from("locked"));
    let id = ui
        .mount_tree(Element::new(
            "field",
            TextField::new(read_only.clone()).read_only(),
        ))
        .unwrap();
    ui.flush(ui.inspect().at).unwrap();
    ui.set_clipboard(MemoryClipboard(Rc::new(RefCell::new("new".into()))))
        .unwrap();
    for command in [
        TextCommand::Paste,
        TextCommand::PasteBefore,
        TextCommand::PasteAfter,
        TextCommand::Cut,
    ] {
        assert_eq!(
            ui.text_command(&id, command).unwrap(),
            TextCommandOutcome::Unavailable,
            "{command:?} must not edit a read-only field"
        );
    }
    assert_eq!(read_only.get().unwrap(), "locked");
}

#[test]
fn spaced_pastes_place_the_clipboard_beside_the_word_at_the_caret() {
    for (after, expected) in [
        (false, "det bruna snabba räven"),
        (true, "det snabba bruna räven"),
    ] {
        let (mut ui, id, value) = setup("det snabba räven", false);
        ui.set_clipboard(MemoryClipboard(Rc::new(RefCell::new("bruna".into()))))
            .unwrap();
        caret(&mut ui, 6); // inside "snabba"
        let command = if after {
            TextCommand::PasteAfter
        } else {
            TextCommand::PasteBefore
        };
        assert_eq!(
            ui.text_command(&id, command).unwrap(),
            TextCommandOutcome::Handled
        );
        frame(&mut ui);
        assert_eq!(value.get().unwrap(), expected);
    }
}

#[test]
fn a_spaced_paste_keeps_its_target_selected_and_undoes_in_one_step() {
    let (mut ui, id, value) = setup("det snabba räven", false);
    ui.set_clipboard(MemoryClipboard(Rc::new(RefCell::new("bruna".into()))))
        .unwrap();
    caret(&mut ui, 4);
    for _ in 0..6 {
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        key(&mut ui, KeyCode::ArrowRight);
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
    }
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text("4..10".into())
    );
    ui.text_command(&id, TextCommand::PasteBefore).unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "det bruna snabba räven");
    // "snabba" moved right by the inserted "bruna ".
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text("10..16".into())
    );
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "det snabba räven");
}

#[test]
fn a_component_can_request_a_command_for_another_component() {
    struct Requester {
        target: Rc<RefCell<Option<WidgetId>>>,
        sent: bool,
    }
    impl Component for Requester {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            let _ = cx;
            Ok(())
        }
        fn keyboard_event(
            &mut self,
            cx: &mut EventContext,
            _: KeyEvent,
            _: &KeyboardState,
        ) -> Result<InputOutcome, RuntimeError> {
            if let Some(target) = self.target.borrow().clone()
                && !self.sent
            {
                self.sent = true;
                cx.request_text_command(&target, TextCommand::PasteAfter)?;
            }
            Ok(InputOutcome::Handled)
        }
        fn focusable(&self) -> bool {
            true
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(String::from("räven"));
    let target = Rc::new(RefCell::new(None));
    let root = ui
        .mount_tree(Element::new(
            "row",
            Column::new([
                Element::new("field", TextField::new(value.clone())),
                Element::new(
                    "requester",
                    Requester {
                        target: target.clone(),
                        sent: false,
                    },
                ),
            ]),
        ))
        .unwrap();
    ui.flush(ui.inspect().at).unwrap();
    let children = ui.children(&root).unwrap();
    let (field, requester) = (children[0].clone(), children[1].clone());
    *target.borrow_mut() = Some(field);
    ui.set_clipboard(MemoryClipboard(Rc::new(RefCell::new("sprang".into()))))
        .unwrap();
    ui.focus(&requester).unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), "räven sprang");
}

fn right_click(ui: &mut Runtime, x: f32, y: f32) {
    for kind in [
        PointerKind::Press(PointerButton::Secondary),
        PointerKind::Release(PointerButton::Secondary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
    }
    frame(ui);
}

#[test]
fn right_clicking_a_field_opens_its_clipboard_menu_and_grays_inapplicable_actions() {
    let (mut ui, id, value) = setup("det snabba räven", false);
    ui.set_clipboard(MemoryClipboard::default()).unwrap();
    // Nothing selected and an empty clipboard: every action is inapplicable, so
    // the list has nothing to focus and Enter cannot edit the text.
    right_click(&mut ui, 30.0, 12.0);
    let items = find(&ui, &id, "items").expect("the menu mounts its action list");
    assert_ne!(ui.focused(), Some(items));
    assert_eq!(value.get().unwrap(), "det snabba räven");
}

#[test]
fn choosing_paste_after_from_the_menu_runs_the_same_command_as_the_contract() {
    let (mut ui, id, value) = setup("det snabba räven", false);
    ui.set_clipboard(MemoryClipboard(Rc::new(RefCell::new("bruna".into()))))
        .unwrap();
    caret(&mut ui, 6); // inside "snabba"
    right_click(&mut ui, 30.0, 12.0);
    let items = find(&ui, &id, "items").expect("the menu mounts its action list");
    assert_eq!(ui.focused(), Some(items));
    // Navigation wraps, so one step back from the first applicable action reaches
    // the last one, Paste After, whichever leading actions happen to be disabled.
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "det snabba bruna räven");
}

#[test]
fn a_read_only_field_can_copy_from_its_menu_but_not_paste() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(String::from("locked text"));
    let id = ui
        .mount_tree(Element::new(
            "field",
            TextField::new(value.clone()).read_only(),
        ))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    let clipboard = MemoryClipboard(Rc::new(RefCell::new("new".into())));
    ui.set_clipboard(clipboard.clone()).unwrap();
    command(&mut ui, KeyCode::KeyA);
    right_click(&mut ui, 30.0, 12.0);
    let items = find(&ui, &id, "items").expect("the menu mounts its action list");
    assert_eq!(ui.focused(), Some(items));
    // Copy is first and enabled; the pastes and Cut are not.
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(&*clipboard.0.borrow(), "locked text");
    assert_eq!(value.get().unwrap(), "locked text");
}

/// The menu stays mounted and hidden, so visibility is what "open" means here.
fn menu_visible(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|c| c.key == "menu" && c.visible)
}

#[test]
fn choosing_an_action_closes_the_menu_and_leaves_the_field_focused() {
    let (mut ui, _, value) = setup("det snabba räven", false);
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    command(&mut ui, KeyCode::KeyA);
    right_click(&mut ui, 30.0, 12.0);
    assert!(menu_visible(&ui), "right-click must show the menu");
    // With a selection and an empty clipboard only Copy and Cut apply, and the
    // list opens with the first applicable action current, so Enter runs Copy.
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(&*clipboard.0.borrow(), "det snabba räven");
    assert!(!menu_visible(&ui), "choosing an action must close the menu");
    assert_eq!(value.get().unwrap(), "det snabba räven");
    // Escape must close it as well, without editing.
    right_click(&mut ui, 30.0, 12.0);
    assert!(menu_visible(&ui));
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert!(!menu_visible(&ui), "Escape must close the menu");
}

#[test]
fn the_menu_key_and_shift_f10_open_the_clipboard_menu_of_both_editors() {
    for multi in [false, true] {
        let (mut ui, _, value) = setup("det snabba räven", multi);
        ui.set_clipboard(MemoryClipboard::default()).unwrap();
        key(&mut ui, KeyCode::ContextMenu);
        assert!(menu_visible(&ui), "the Menu key must show the menu");
        key(&mut ui, KeyCode::Escape);
        assert!(!menu_visible(&ui));
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        key(&mut ui, KeyCode::F10);
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
            .unwrap();
        assert!(menu_visible(&ui), "Shift+F10 must show the menu");
        key(&mut ui, KeyCode::Escape);
        assert_eq!(value.get().unwrap(), "det snabba räven");
    }
}

#[test]
fn the_pointer_over_an_open_menu_is_not_the_editor_caret() {
    let (mut ui, _, _) = setup("det snabba räven", false);
    ui.set_clipboard(MemoryClipboard::default()).unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(30.0, 12.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    frame(&mut ui);
    assert_eq!(ui.cursor_icon(), CursorIcon::Text);
    right_click(&mut ui, 30.0, 12.0);
    // The menu is placed at the press position, so the pointer now rests on it.
    // Without its own answer the resolution would fall back to the field's caret.
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
}

#[test]
fn public_text_limit_counts_bytes_and_rejects_external_values_without_rewriting() {
    let (mut ui, _, value) = setup("preserve", true);
    let oversized = "é".repeat(TextEditError::MAX_TEXT_BYTES / 2 + 1);
    assert!(oversized.chars().count() < TextEditError::MAX_TEXT_BYTES);
    assert_eq!(TextArea::check(&oversized), Err(TextEditError::TooLarge));
    value.set(oversized.clone()).unwrap();
    assert!(ui.flush(Duration::ZERO).is_ok());
    assert_eq!(value.get().unwrap(), oversized);
}

#[test]
fn a_refused_binding_keeps_the_field_and_the_runtime_working() {
    let (mut ui, _, value) = setup("previous", true);
    assert_eq!(TextArea::check("one\ntwo\tthree"), Ok(()));
    assert_eq!(
        TextArea::check("from a file\r\n"),
        Err(TextEditError::InvalidText)
    );
    assert_eq!(
        TextField::check("one\ntwo"),
        Err(TextEditError::InvalidText)
    );
    // CRLF text read from a file, written without checking it first.
    value.set("from a file\r\n".into()).unwrap();
    frame(&mut ui);
    frame(&mut ui);
    assert_eq!(property(&ui, "text"), InspectValue::Text("previous".into()));
    assert!(matches!(
        property(&ui, "binding_error"),
        InspectValue::Text(_)
    ));
    // Editing is refused, so the application's value is never overwritten.
    assert!(ui.text_input_state().is_none());
    ui.text_input("typed").unwrap();
    key(&mut ui, KeyCode::Backspace);
    command(&mut ui, KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), "from a file\r\n");
    // Paint shows the refusal with the error outline.
    let error = Palette::dark().color(ColorRole::Error);
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. }
                    if *color == error
            ))
    );
    // The next acceptable value ends the refusal and editing resumes.
    value.set("from a file\n".into()).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "text"),
        InspectValue::Text("from a file\n".into())
    );
    assert!(
        !ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "field")
            .unwrap()
            .details
            .properties
            .contains_key("binding_error")
    );
    ui.text_input("x").unwrap();
    assert_eq!(value.get().unwrap(), "xfrom a file\n");
}

#[test]
fn pasted_control_characters_become_line_breaks_or_are_dropped() {
    let pasted = "a\u{85}b\u{0B}c\u{0C}d\u{7}e\u{1B}\tf\u{9C}g";
    for (multi, expected) in [(false, "a b c de fg"), (true, "a\nb\nc\nde\tfg")] {
        let (mut ui, id, value) = setup("", multi);
        let clipboard = MemoryClipboard::default();
        ui.set_clipboard(clipboard.clone()).unwrap();
        *clipboard.0.borrow_mut() = pasted.into();
        command(&mut ui, KeyCode::KeyV);
        assert_eq!(value.get().unwrap(), expected);
        // Only control characters to paste: nothing to do.
        *clipboard.0.borrow_mut() = "\u{7}\u{1B}".into();
        assert_eq!(
            ui.text_command(&id, TextCommand::Paste).unwrap(),
            TextCommandOutcome::Unavailable
        );
        assert_eq!(value.get().unwrap(), expected);
    }
}

#[test]
fn a_command_during_composition_replaces_it_and_starts_a_new_ime_session() {
    let (mut ui, id, value) = setup("abc", false);
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    *clipboard.0.borrow_mut() = "PASTE".into();
    ui.text_preedit(&TextPreedit::new("ka", None).unwrap())
        .unwrap();
    frame(&mut ui);
    let session = ui.text_input_state().unwrap().1.session;
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "PASTEabc");
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    // A new session tells the host to reset native composition, so the IME's
    // late commit of the old session cannot land after the pasted text.
    assert_ne!(ui.text_input_state().unwrap().1.session, session);
}

#[test]
fn number_inputs_and_inline_text_answer_text_commands_like_their_chords() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    let count = ui.signal(7_i64);
    let id = ui
        .mount_tree(Element::new("field", IntegerInput::new(count.clone())))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    command(&mut ui, KeyCode::KeyA);
    assert_eq!(
        ui.text_command(&id, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(&*clipboard.0.borrow(), "7");
    *clipboard.0.borrow_mut() = "42".into();
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(count.get().unwrap(), 7, "a pasted number is a draft");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(count.get().unwrap(), 42);

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    ui.set_clipboard(clipboard.clone()).unwrap();
    *clipboard.0.borrow_mut() = "9".into();
    let amount = ui.signal(1_i64);
    let id = ui
        .mount_tree(Element::new(
            "field",
            InlineInteger::new(amount.clone()).range(0, 100).unwrap(),
        ))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    // A resting row holds no draft to act on.
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::NotAnEditor
    );
    key(&mut ui, KeyCode::F2);
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    key(&mut ui, KeyCode::Enter);
    assert_eq!(amount.get().unwrap(), 9);

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    ui.set_clipboard(clipboard.clone()).unwrap();
    *clipboard.0.borrow_mut() = "Bass".into();
    let name = ui.signal(String::from("Lead"));
    let id = ui
        .mount_tree(Element::new("field", InlineText::new(name.clone())))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::NotAnEditor
    );
    key(&mut ui, KeyCode::F2);
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(name.get().unwrap(), "Lead", "still a draft");
    key(&mut ui, KeyCode::Enter);
    assert_eq!(name.get().unwrap(), "Bass");
}

#[test]
fn a_saved_inline_text_session_leaves_no_undo_history_behind() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let name = ui.signal(String::from("Lead"));
    let id = ui
        .mount_tree(Element::new("field", InlineText::new(name.clone())))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::F2);
    ui.text_input("Bass").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert_eq!(name.get().unwrap(), "Bass");
    // The next session starts with an empty history: undo cannot bring back
    // the previous session's draft states.
    key(&mut ui, KeyCode::F2);
    command(&mut ui, KeyCode::KeyZ);
    command(&mut ui, KeyCode::KeyZ);
    key(&mut ui, KeyCode::Enter);
    assert_eq!(name.get().unwrap(), "Bass");
}

#[test]
fn selected_text_follows_an_on_surface_override() {
    let (mut ui, _, _) = setup("hello", false);
    command(&mut ui, KeyCode::KeyA);
    assert_eq!(
        property(&ui, "selection"),
        InspectValue::Text("0..5".into())
    );
    let text = Color::rgba(0.9, 0.6, 0.2, 1.0).unwrap();
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::OnSurface, text)
        .unwrap();
    frame(&mut ui);
    let scene = ui.paint().unwrap();
    let masks = |color: Color| {
        scene.commands().iter().any(|command| {
            matches!(command, DrawCommand::DrawMask { color: drawn, .. } if *drawn == color)
        })
    };
    assert!(masks(text), "selected text takes the overridden text color");
    assert!(
        !masks(Palette::dark().color(ColorRole::OnSurface)),
        "no glyph keeps the default text color"
    );
}

#[test]
fn select_all_is_a_text_command_that_an_empty_editor_cannot_answer() {
    let (mut ui, id, value) = setup("det snabba räven", false);
    assert_eq!(
        ui.text_command(&id, TextCommand::SelectAll).unwrap(),
        TextCommandOutcome::Handled
    );
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    ui.text_command(&id, TextCommand::Copy).unwrap();
    assert_eq!(*clipboard.0.borrow(), "det snabba räven");
    value.set(String::new()).unwrap();
    frame(&mut ui);
    assert_eq!(
        ui.text_command(&id, TextCommand::SelectAll).unwrap(),
        TextCommandOutcome::Unavailable
    );
}
