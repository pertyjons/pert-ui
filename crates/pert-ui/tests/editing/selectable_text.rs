//! Selectable display text: pointer and keyboard selection, copying, Escape,
//! the clipboard menu, bound text, refusal, focus and semantics.
use crate::support::{self, frame, widget};
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

const TEXT: &str = "the quick brown fox";

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

struct Setup {
    ui: Runtime,
    root: WidgetId,
    text: WidgetId,
    value: Signal<String>,
    copied: Memory,
}

fn setup(initial: &str) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let copied = Memory::default();
    ui.set_clipboard(copied.clone()).unwrap();
    let value = ui.signal(initial.to_owned());
    let root = ui
        .mount_tree(
            Column::new([
                SelectableText::new(value.read_only()).key("text"),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ])
            .key("page"),
        )
        .unwrap();
    frame(&mut ui);
    let text = widget(&ui, &root, "text");
    Setup {
        ui,
        root,
        text,
        value,
        copied,
    }
}

/// The inspection property `name` of the component `id`, whose key is unique.
fn property(ui: &Runtime, id: &WidgetId, name: &str) -> Option<InspectValue> {
    let key = ui.key(id).unwrap();
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.properties.get(name).cloned())
}
fn selection(ui: &Runtime, id: &WidgetId) -> String {
    match property(ui, id, "selection") {
        Some(InspectValue::Text(text)) => text,
        other => panic!("selection {other:?}"),
    }
}
fn shown(ui: &Runtime, id: &WidgetId) -> String {
    match property(ui, id, "text") {
        Some(InspectValue::Text(text)) => text,
        other => panic!("text {other:?}"),
    }
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
}
fn click(ui: &mut Runtime, x: f32, y: f32, time: u64) {
    ui.flush(Duration::from_millis(time)).unwrap();
    pointer(ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(ui, x, y, PointerKind::Release(PointerButton::Primary));
    frame(ui);
}
fn key(ui: &mut Runtime, code: KeyCode) {
    support::key(ui, code);
    frame(ui);
}
fn held(ui: &mut Runtime, modifier: KeyCode, code: KeyCode) {
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
}
fn shifted(ui: &mut Runtime, code: KeyCode) {
    held(ui, KeyCode::ShiftLeft, code);
}
fn command(ui: &mut Runtime, code: KeyCode) {
    held(ui, KeyCode::primary(), code);
}
/// The platform's word-movement modifier: Option on macOS, Control elsewhere.
fn word() -> KeyCode {
    if cfg!(target_os = "macos") {
        KeyCode::AltLeft
    } else {
        KeyCode::ControlLeft
    }
}
fn native_escape(ui: &mut Runtime) {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::Escape,
            action,
            CommandKey::Named(KeyCode::Escape),
        ))
        .unwrap();
    }
    frame(ui);
}

#[test]
fn a_drag_selects_and_the_copy_chord_copies_exactly_that() {
    let Setup {
        mut ui,
        text,
        copied,
        ..
    } = setup(TEXT);
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(ui.focused(), Some(text.clone()), "the press focused it");
    pointer(&mut ui, 60.0, 8.0, PointerKind::Move);
    pointer(
        &mut ui,
        60.0,
        8.0,
        PointerKind::Release(PointerButton::Primary),
    );
    frame(&mut ui);
    let range = selection(&ui, &text);
    let (anchor, focus) = range.split_once("..").unwrap();
    let (anchor, focus): (usize, usize) = (anchor.parse().unwrap(), focus.parse().unwrap());
    assert_eq!(anchor, 0);
    assert!(focus > 0 && focus < TEXT.len(), "{range}");
    command(&mut ui, KeyCode::KeyC);
    assert_eq!(*copied.0.borrow(), TEXT[..focus]);
    // Shift+press extends from the anchor.
    key(&mut ui, KeyCode::End);
    ui.flush(Duration::from_millis(1000)).unwrap();
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    pointer(
        &mut ui,
        60.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut ui,
        60.0,
        8.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), format!("{}..{focus}", TEXT.len()));
}

#[test]
fn double_and_triple_clicks_select_a_word_and_everything_and_a_fourth_starts_over() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    click(&mut ui, 2.0, 8.0, 10);
    assert_eq!(selection(&ui, &text), "0..0");
    click(&mut ui, 2.0, 8.0, 100);
    assert_eq!(selection(&ui, &text), "0..3");
    click(&mut ui, 2.0, 8.0, 200);
    assert_eq!(selection(&ui, &text), format!("0..{}", TEXT.len()));
    click(&mut ui, 2.0, 8.0, 300);
    assert_eq!(selection(&ui, &text), "0..0");
}

#[test]
fn the_editors_movement_keys_move_and_extend_the_selection() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup(TEXT);
    ui.focus(&text).unwrap();
    frame(&mut ui);
    for _ in 0..3 {
        shifted(&mut ui, KeyCode::ArrowRight);
    }
    assert_eq!(selection(&ui, &text), "0..3");
    // Without Shift, a movement collapses the selection to its end.
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(selection(&ui, &text), "3..3");
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(selection(&ui, &text), "4..4");
    // The word modifier moves by word; with Shift it extends.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    held(&mut ui, word(), KeyCode::ArrowRight);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(selection(&ui, &text), "4..9");
    shifted(&mut ui, KeyCode::End);
    assert_eq!(selection(&ui, &text), format!("4..{}", TEXT.len()));
    key(&mut ui, KeyCode::Home);
    assert_eq!(selection(&ui, &text), "0..0");
    // A repeat moves like a press.
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Repeat)
        .unwrap();
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)
        .unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "2..2");
    command(&mut ui, KeyCode::KeyA);
    assert_eq!(selection(&ui, &text), format!("0..{}", TEXT.len()));
    command(&mut ui, KeyCode::End);
    assert_eq!(selection(&ui, &text), format!("{0}..{0}", TEXT.len()));
    // Typing and composition change nothing.
    ui.text_input("x").unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), TEXT);
    assert_eq!(shown(&ui, &text), TEXT);
    assert!(ui.text_input_state().is_none());
}

#[test]
fn escape_ends_a_drag_then_collapses_then_bubbles() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 60.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    let dragged = selection(&ui, &text);
    key(&mut ui, KeyCode::Escape);
    // The drag ended and kept its selection: moving no longer extends it.
    pointer(&mut ui, 200.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), dragged);
    key(&mut ui, KeyCode::Escape);
    let focus = dragged.split_once("..").unwrap().1;
    assert_eq!(selection(&ui, &text), format!("{focus}..{focus}"));
}

fn popover(native: bool) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let open = ui.signal(false);
    let root = ui
        .mount_tree(
            // A column keeps the anchor at its natural height, leaving room
            // below it for the popup.
            Column::new([Element::new(
                "popover",
                Popover::new(
                    open.clone(),
                    Element::new("anchor", Label::new("Anchor")),
                    SelectableText::new(TEXT).key("text"),
                ),
            )])
            .key("page"),
        )
        .unwrap();
    frame(&mut ui);
    open.set(true).unwrap();
    frame(&mut ui);
    let text = widget(&ui, &root, "text");
    ui.focus(&text).unwrap();
    command(&mut ui, KeyCode::KeyA);
    let escape = |ui: &mut Runtime| {
        if native {
            native_escape(ui);
        } else {
            key(ui, KeyCode::Escape);
        }
    };
    escape(&mut ui);
    assert_eq!(
        selection(&ui, &text),
        format!("{0}..{0}", TEXT.len()),
        "the first Escape collapses"
    );
    assert!(open.get().unwrap(), "and the popover stays");
    escape(&mut ui);
    assert!(!open.get().unwrap(), "the second closes it");
}

#[test]
fn inside_a_popover_the_first_escape_collapses_and_the_second_closes() {
    popover(false);
    popover(true);
}

#[test]
fn a_capture_command_on_escape_waits_while_the_selection_collapses() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    let ran = Rc::new(std::cell::Cell::new(0));
    let hit = ran.clone();
    let _command = ui
        .register_command(
            "Escape",
            CommandScope::Capture,
            vec![KeyChord::named(KeyCode::Escape)],
            move |_, _| {
                hit.set(hit.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    ui.focus(&text).unwrap();
    command(&mut ui, KeyCode::KeyA);
    native_escape(&mut ui);
    assert_eq!(ran.get(), 0);
    assert_eq!(selection(&ui, &text), format!("{0}..{0}", TEXT.len()));
    native_escape(&mut ui);
    assert_eq!(ran.get(), 1, "with nothing to collapse, Escape reaches it");
}

#[test]
fn the_context_menu_copies_the_selection_it_keeps() {
    let Setup {
        mut ui,
        root,
        text,
        copied,
        ..
    } = setup(TEXT);
    ui.focus(&text).unwrap();
    for _ in 0..3 {
        shifted(&mut ui, KeyCode::ArrowRight);
    }
    pointer(
        &mut ui,
        80.0,
        8.0,
        PointerKind::Press(PointerButton::Secondary),
    );
    pointer(
        &mut ui,
        80.0,
        8.0,
        PointerKind::Release(PointerButton::Secondary),
    );
    frame(&mut ui);
    let items = widget(&ui, &root, "items");
    assert_eq!(ui.focused(), Some(items), "Copy is available");
    assert_eq!(
        selection(&ui, &text),
        "0..3",
        "the menu keeps the selection"
    );
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(*copied.0.borrow(), "the");
    assert_eq!(selection(&ui, &text), "0..3");
    assert_eq!(
        ui.focused(),
        Some(text),
        "the menu closed and focus returned"
    );
}

#[test]
fn without_a_selection_the_menu_offers_select_all_and_escape_returns_focus() {
    let Setup {
        mut ui, root, text, ..
    } = setup(TEXT);
    ui.focus(&text).unwrap();
    frame(&mut ui);
    // Shift+F10 opens it from the keyboard; Copy is unavailable, so the
    // first available action is Select All.
    shifted(&mut ui, KeyCode::F10);
    let items = widget(&ui, &root, "items");
    assert_eq!(ui.focused(), Some(items));
    key(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(text.clone()), "focus returns");
    shifted(&mut ui, KeyCode::F10);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), format!("0..{}", TEXT.len()));
}

/// A clipboard service that is busy.
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
fn text_commands_copy_select_and_never_change_the_text() {
    let Setup {
        mut ui,
        text,
        value,
        copied,
        ..
    } = setup(TEXT);
    assert_eq!(
        ui.text_command(&text, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Unavailable,
        "nothing selected"
    );
    assert_eq!(
        ui.text_command(&text, TextCommand::SelectAll).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(
        ui.text_command(&text, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(*copied.0.borrow(), TEXT);
    for command in [
        TextCommand::Cut,
        TextCommand::Paste,
        TextCommand::PasteBefore,
        TextCommand::PasteAfter,
    ] {
        assert_eq!(
            ui.text_command(&text, command).unwrap(),
            TextCommandOutcome::Unavailable,
            "{command:?}"
        );
    }
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), TEXT);
    assert_eq!(shown(&ui, &text), TEXT);
    ui.set_clipboard(Busy).unwrap();
    assert_eq!(
        ui.text_command(&text, TextCommand::Copy),
        Err(RuntimeError::Clipboard(ClipboardError::Busy))
    );
}

/// Selects `anchor..focus` by keyboard from the start.
fn select(ui: &mut Runtime, text: &WidgetId, anchor: usize, focus: usize) {
    ui.focus(text).unwrap();
    command(ui, KeyCode::Home);
    for _ in 0..anchor {
        key(ui, KeyCode::ArrowRight);
    }
    let code = if focus >= anchor {
        KeyCode::ArrowRight
    } else {
        KeyCode::ArrowLeft
    };
    for _ in 0..anchor.abs_diff(focus) {
        shifted(ui, code);
    }
}

#[test]
fn a_replaced_text_keeps_a_selection_inside_it_snapped_and_directed() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup(TEXT);
    select(&mut ui, &text, 4, 9);
    value.set("the quick red fox".into()).unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "4..9");
    select(&mut ui, &text, 9, 4);
    value.set("the quick fox".into()).unwrap();
    frame(&mut ui);
    assert_eq!(
        selection(&ui, &text),
        "9..4",
        "a backward selection stays backward"
    );
    // An end outside the new text collapses it to the start.
    value.set("the".into()).unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "0..0");
    // An end inside a grapheme snaps to its start: a combining mark...
    value.set("abc".into()).unwrap();
    frame(&mut ui);
    select(&mut ui, &text, 1, 3);
    value.set("a\u{301}bc".into()).unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "0..3");
    // ...and an emoji joined by a zero-width joiner.
    value.set("abcdefghijk".into()).unwrap();
    frame(&mut ui);
    select(&mut ui, &text, 0, 4);
    value.set("👩\u{200d}💻 dev".into()).unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "0..0");
}

#[test]
fn a_replacement_ends_a_drag_and_empty_text_gives_up_focus() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup(TEXT);
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    frame(&mut ui);
    value.set("the quick brown fox jumps".into()).unwrap();
    frame(&mut ui);
    pointer(&mut ui, 200.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "0..0", "the drag ended");
    assert_eq!(ui.focused(), Some(text.clone()));
    value.set(String::new()).unwrap();
    frame(&mut ui);
    assert_ne!(ui.focused(), Some(text));
}

#[test]
fn role_and_rich_sources_show_and_copy_their_plain_text() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let copied = Memory::default();
    ui.set_clipboard(copied.clone()).unwrap();
    let rich = RichText::from("bold")
        .span(TextSpan::new(" and plain"))
        .trailing_icon(Icon::Check);
    let root = ui
        .mount_tree(
            Column::new([
                SelectableText::new(StringRole::TextCopy).key("role"),
                SelectableText::new(rich).key("rich"),
                SelectableText::new(RichText::from("").icon(Icon::Check)).key("icon"),
            ])
            .key("page"),
        )
        .unwrap();
    frame(&mut ui);
    let role = widget(&ui, &root, "role");
    assert_eq!(shown(&ui, &role), "Copy");
    ui.ui_settings()
        .set_strings(
            UiStrings::english()
                .with_text(StringRole::TextCopy, "Kopiera")
                .unwrap(),
        )
        .unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui, &role), "Kopiera", "it follows a replaced table");
    let rich = widget(&ui, &root, "rich");
    assert_eq!(shown(&ui, &rich), "bold and plain");
    ui.text_command(&rich, TextCommand::SelectAll).unwrap();
    ui.text_command(&rich, TextCommand::Copy).unwrap();
    assert_eq!(*copied.0.borrow(), "bold and plain");
    let icon = widget(&ui, &root, "icon");
    assert_eq!(shown(&ui, &icon), "");
}

fn selection_fills(ui: &mut Runtime) -> Vec<[f32; 4]> {
    support::settle(ui);
    let color = Palette::dark().color(ColorRole::TextSelection);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color: c } if *c == color => Some(rect.values()),
            _ => None,
        })
        .collect()
}

#[test]
fn wrapped_and_mixed_direction_selections_paint_every_region() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(60.0, 400.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("one two three four five six seven".to_owned());
    let root = ui
        .mount_tree(Column::new([SelectableText::new(value.read_only()).key("text")]).key("page"))
        .unwrap();
    frame(&mut ui);
    let text = widget(&ui, &root, "text");
    let bounds = ui.bounds(&text).unwrap().unwrap();
    let line = ui
        .event(&text, |cx| cx.text("one", None, TextStyle::Body))
        .unwrap()
        .unwrap()
        .size()
        .height();
    assert!(
        bounds.height() > 2.0 * line,
        "it wraps and grows: {bounds:?}"
    );
    assert!(bounds.width() <= 60.0);
    ui.text_command(&text, TextCommand::SelectAll).unwrap();
    let fills = selection_fills(&mut ui);
    assert!(fills.len() >= 3, "one region per line: {fills:?}");
    // Mixed direction: a selection from inside "abc" into the right-to-left
    // word covers separate visual regions; every rectangle the selection has
    // is painted, clipped to the component, and nothing between them.
    let mixed = "abc אבג def";
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    value.set(mixed.into()).unwrap();
    frame(&mut ui);
    ui.focus(&text).unwrap();
    key(&mut ui, KeyCode::Home);
    key(&mut ui, KeyCode::ArrowRight);
    for _ in 0..4 {
        shifted(&mut ui, KeyCode::ArrowRight);
    }
    let range = selection(&ui, &text);
    let (anchor, focus) = range.split_once("..").unwrap();
    let (anchor, focus): (usize, usize) = (anchor.parse().unwrap(), focus.parse().unwrap());
    let bounds = ui.bounds(&text).unwrap().unwrap();
    let expected: Vec<[f32; 4]> = ui
        .event(&text, |cx| -> Result<Vec<[f32; 4]>, RuntimeError> {
            let layout = cx.text(mixed, Some(bounds.width()), TextStyle::Body)?;
            Ok(TextSelection::new(&layout, anchor, focus)
                .rectangles(&layout)?
                .into_iter()
                .map(|r| {
                    let [x, y, w, h] = r.values();
                    let right = (x + w).min(bounds.width());
                    [x + bounds.x(), y + bounds.y(), right - x, h]
                })
                .collect())
        })
        .unwrap()
        .unwrap();
    assert!(
        expected.len() >= 2,
        "separate regions for {range}: {expected:?}"
    );
    let painted = selection_fills(&mut ui);
    assert_eq!(painted.len(), expected.len(), "{painted:?} {expected:?}");
    for (painted, expected) in painted.iter().zip(&expected) {
        assert!(
            painted
                .iter()
                .zip(expected)
                .all(|(a, b)| (a - b).abs() < 1e-3),
            "{painted:?} {expected:?}"
        );
    }
}

#[test]
fn text_that_cannot_be_prepared_is_refused_and_recovers() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup(TEXT);
    // Over the size limit.
    value
        .set("a".repeat(TextEditError::MAX_TEXT_BYTES + 1))
        .unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui, &text), TEXT, "the last prepared text stays");
    assert!(property(&ui, &text, "text_error").is_some());
    let invalid = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "text")
            .and_then(|c| c.details.semantics)
            .is_some_and(|s| s.invalid)
    };
    assert!(invalid(&ui));
    value.set("short".into()).unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui, &text), "short");
    assert!(property(&ui, &text, "text_error").is_none());
    assert!(!invalid(&ui));
    // Beyond the raster limit at a narrow width: too many wrapped lines.
    let long = "ab ".repeat(1500);
    ui.set_viewport(Viewport::new(1000.0, 200.0, 1.0).unwrap())
        .unwrap();
    value.set(long.clone()).unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui, &text), long);
    ui.set_viewport(Viewport::new(30.0, 200.0, 1.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert!(property(&ui, &text, "text_error").is_some());
    assert_eq!(shown(&ui, &text), long, "the wider layout stays");
    ui.set_viewport(Viewport::new(1000.0, 200.0, 1.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert!(property(&ui, &text, "text_error").is_none());
}

#[test]
fn a_binding_from_another_runtime_fails_the_update() {
    let mut ui = Runtime::new();
    let other = Runtime::new();
    let foreign = other.signal("elsewhere".to_owned());
    ui.mount_tree(SelectableText::new(foreign.read_only()).key("text"))
        .unwrap();
    assert_eq!(
        ui.flush(Duration::ZERO).unwrap_err(),
        RuntimeError::ForeignRuntime
    );
}

#[test]
fn it_is_one_tab_stop_and_empty_text_is_none() {
    let Setup {
        mut ui,
        root,
        text,
        value,
        ..
    } = setup(TEXT);
    let other = widget(&ui, &root, "other");
    ui.focus(&other).unwrap();
    shifted(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(text.clone()));
    key(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(other.clone()));
    value.set(String::new()).unwrap();
    frame(&mut ui);
    shifted(&mut ui, KeyCode::Tab);
    assert_ne!(ui.focused(), Some(text));
}

#[test]
fn semantics_name_it_by_its_text_and_content_search_finds_it() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    ui.text_command(&text, TextCommand::SelectAll).unwrap();
    frame(&mut ui);
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "text")
        .and_then(|c| c.details.semantics)
        .unwrap();
    assert_eq!(semantics.role, SemanticRole::Label);
    assert_eq!(semantics.name.as_deref(), Some(TEXT));
    assert!(semantics.read_only);
    assert_eq!(selection(&ui, &text), format!("0..{}", TEXT.len()));
    let mut search = ui.content_search(ContentQuery::new("brown").unwrap());
    let mut hits = Vec::new();
    loop {
        let step = search.step(&ui).unwrap();
        hits.extend(step.hits);
        if step.done {
            break;
        }
    }
    assert!(
        hits.iter()
            .any(|hit| matches!(&hit.target, ContentTarget::Widget(id) if *id == text)),
        "{hits:?}"
    );
}

#[test]
fn cancelled_input_ends_a_drag_and_keeps_the_selection() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 60.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    let dragged = selection(&ui, &text);
    ui.cancel_input().unwrap();
    frame(&mut ui);
    assert_eq!(ui.focused(), Some(text.clone()), "focus stays");
    // A move over the text itself, which it receives without capture.
    assert!(ui.bounds(&text).unwrap().unwrap().width() > 100.0);
    pointer(&mut ui, 100.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), dragged);
}

#[test]
fn native_keys_are_read_by_their_layout_identity() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    ui.focus(&text).unwrap();
    frame(&mut ui);
    let chord = |ui: &mut Runtime, physical: KeyCode, base: char| {
        let modifier = KeyCode::primary();
        let named = |action| CommandKeyInput::new(modifier, action, CommandKey::Named(modifier));
        ui.command_key_input(named(KeyAction::Press)).unwrap();
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                physical,
                action,
                CommandKey::Character(base),
            ))
            .unwrap();
        }
        ui.command_key_input(named(KeyAction::Release)).unwrap();
        frame(ui);
    };
    // The physical A key types Q in this layout: nothing is selected.
    chord(&mut ui, KeyCode::KeyA, 'q');
    assert_eq!(selection(&ui, &text), "0..0");
    // The physical Q key types A: everything is.
    chord(&mut ui, KeyCode::KeyQ, 'a');
    assert_eq!(selection(&ui, &text), format!("0..{}", TEXT.len()));
}

#[test]
fn native_escape_ends_a_drag_before_collapsing() {
    let Setup { mut ui, text, .. } = setup(TEXT);
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 60.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    let dragged = selection(&ui, &text);
    native_escape(&mut ui);
    pointer(&mut ui, 200.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    assert_eq!(
        selection(&ui, &text),
        dragged,
        "the drag ended, the selection stayed"
    );
    native_escape(&mut ui);
    let focus = dragged.split_once("..").unwrap().1;
    assert_eq!(selection(&ui, &text), format!("{focus}..{focus}"));
}

#[test]
fn up_down_word_and_document_movement_in_multiline_text() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup("first line\nsecond line");
    ui.focus(&text).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::ArrowDown);
    let below = selection(&ui, &text);
    let offset: usize = below.split_once("..").unwrap().0.parse().unwrap();
    assert!(offset >= "first line\n".len(), "{below}");
    key(&mut ui, KeyCode::ArrowUp);
    assert_eq!(selection(&ui, &text), "0..0");
    command(&mut ui, KeyCode::End);
    let end = "first line\nsecond line".len();
    assert_eq!(selection(&ui, &text), format!("{end}..{end}"));
    held(&mut ui, word(), KeyCode::ArrowLeft);
    assert_eq!(selection(&ui, &text), "18..18", "to the start of \"line\"");
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    command(&mut ui, KeyCode::Home);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(selection(&ui, &text), "18..0");
    // Composition changes nothing.
    ui.text_preedit(&TextPreedit::new("ä", Some(2..2)).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(value.get().unwrap(), "first line\nsecond line");
    assert_eq!(selection(&ui, &text), "18..0");
}

#[test]
fn another_source_ends_a_drag_and_the_same_source_keeps_it() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (first, second) = (ui.signal(TEXT.to_owned()), ui.signal(TEXT.to_owned()));
    let which = ui.signal(0u8);
    let (a, b) = (first.clone(), second.clone());
    let root = ui
        .mount_tree(Element::new(
            "dynamic",
            Dynamic::new(which.read_only(), move |_, which| {
                // 0 and 1 describe the first source again, 2 the second.
                let source = if *which == 2 { &b } else { &a };
                SelectableText::new(source.read_only()).key("text")
            }),
        ))
        .unwrap();
    frame(&mut ui);
    let text = widget(&ui, &root, "text");
    pointer(
        &mut ui,
        1.0,
        8.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 40.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    which.set(1).unwrap();
    frame(&mut ui);
    pointer(&mut ui, 60.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    let kept = selection(&ui, &text);
    assert_ne!(kept, "0..0", "the same source keeps the drag going");
    which.set(2).unwrap();
    frame(&mut ui);
    pointer(&mut ui, 200.0, 8.0, PointerKind::Move);
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), kept, "another source ended it");
}

#[test]
fn a_width_change_keeps_the_selection_and_selected_glyphs_take_their_color() {
    let Setup {
        mut ui,
        text,
        value,
        ..
    } = setup(TEXT);
    select(&mut ui, &text, 4, 9);
    ui.set_viewport(Viewport::new(60.0, 200.0, 1.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "4..9");
    let selected = Palette::dark().color(ColorRole::OnTextSelection);
    let tinted = ui
        .paint()
        .unwrap()
        .commands()
        .iter()
        .filter(
            |command| matches!(command, DrawCommand::DrawMask { color, .. } if *color == selected),
        )
        .count();
    assert!(tinted > 0, "selected glyphs are painted in OnTextSelection");
    // A refused text keeps the shown text with its selection, and copies it.
    value
        .set("a".repeat(TextEditError::MAX_TEXT_BYTES + 1))
        .unwrap();
    frame(&mut ui);
    assert_eq!(selection(&ui, &text), "4..9");
    let copied = Memory::default();
    ui.set_clipboard(copied.clone()).unwrap();
    ui.text_command(&text, TextCommand::Copy).unwrap();
    assert_eq!(*copied.0.borrow(), "quick");
}
