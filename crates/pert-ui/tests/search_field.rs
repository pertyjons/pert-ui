//! `SearchField` contracts: the query is committed text, Escape clears before
//! it reaches the owner, owner keys leave focus and caret alone, the clear
//! button, and the debounce deadline.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{frame, settle};

type Log = Rc<RefCell<Vec<String>>>;

/// An owner that records every key its search field leaves to it.
struct Owner {
    log: Log,
    children: Vec<Element>,
}
impl Component for Owner {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(300.0, 120.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 0.0, 240.0, 32.0)?)?;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Pressed {
            self.log.borrow_mut().push(format!("{:?}", event.code));
        }
        Ok(InputOutcome::Handled)
    }
}

struct Setup {
    ui: Runtime,
    field: WidgetId,
    query: Signal<String>,
    owner_keys: Log,
    reported: Rc<RefCell<Vec<SearchKey>>>,
}

fn setup(text: &str, build: impl FnOnce(SearchField) -> SearchField) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let query = ui.signal(text.to_owned());
    let owner_keys = Log::default();
    let reported = Rc::new(RefCell::new(Vec::new()));
    let root = ui
        .mount_tree(Element::new(
            "owner",
            Owner {
                log: owner_keys.clone(),
                children: vec![Element::new(
                    "search",
                    build(SearchField::new(query.clone())),
                )],
            },
        ))
        .unwrap();
    let field = ui.find_descendant(&root, "search").unwrap().unwrap();
    ui.focus(&field).unwrap();
    frame(&mut ui);
    Setup {
        ui,
        field,
        query,
        owner_keys,
        reported,
    }
}
/// A setup whose field reports its owner keys to `reported`.
fn setup_reporting(text: &str) -> Setup {
    let reported = Rc::new(RefCell::new(Vec::new()));
    let sink = reported.clone();
    let mut setup = setup(text, move |field| {
        field.on_key(move |_, key| {
            sink.borrow_mut().push(key);
            Ok(())
        })
    });
    setup.reported = reported;
    setup
}
fn press(ui: &mut Runtime, code: KeyCode) {
    support::key(ui, code);
    frame(ui);
}
fn native(ui: &mut Runtime, code: KeyCode) -> bool {
    let press = ui
        .command_key_input(CommandKeyInput::new(
            code,
            KeyAction::Press,
            CommandKey::Named(code),
        ))
        .unwrap();
    ui.command_key_input(CommandKeyInput::new(
        code,
        KeyAction::Release,
        CommandKey::Named(code),
    ))
    .unwrap();
    frame(ui);
    press.handled
}
fn property(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "search")
        .unwrap()
        .details
        .properties[key]
        .clone()
}
fn text(ui: &Runtime) -> String {
    match property(ui, "text") {
        InspectValue::Text(text) => text,
        other => panic!("unexpected text {other:?}"),
    }
}
fn click(ui: &mut Runtime, x: f32, y: f32) {
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
    }
    frame(ui);
}

#[test]
fn the_query_is_committed_text_and_never_preedit() {
    let Setup { mut ui, query, .. } = setup("", |field| field);
    ui.text_input("re").unwrap();
    assert_eq!(query.get().unwrap(), "re");
    ui.text_preedit(&TextPreedit::new("verb", Some(4..4)).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(query.get().unwrap(), "re", "composition is not a query");
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(true));
    ui.text_input("verb").unwrap();
    frame(&mut ui);
    assert_eq!(query.get().unwrap(), "reverb");
    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "search")
        .unwrap()
        .details
        .semantics
        .unwrap();
    assert_eq!(semantics.role, SemanticRole::SearchInput);
    assert_eq!(semantics.value, Some(SemanticValue::Text("reverb".into())));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn escape_clears_a_query_and_otherwise_reaches_the_owner() {
    let Setup {
        mut ui,
        field,
        query,
        owner_keys,
        ..
    } = setup("delay", |field| field);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(query.get().unwrap(), "");
    assert_eq!(ui.focused(), Some(field.clone()), "clearing keeps focus");
    assert!(
        owner_keys.borrow().is_empty(),
        "the clearing Escape is handled"
    );
    press(&mut ui, KeyCode::Escape);
    assert_eq!(*owner_keys.borrow(), ["Escape"]);
    assert_eq!(ui.focused(), Some(field));
}

#[test]
fn escape_during_composition_cancels_only_the_composition() {
    let Setup {
        mut ui,
        query,
        owner_keys,
        ..
    } = setup("eq", |field| field);
    ui.text_preedit(&TextPreedit::new("ue", Some(2..2)).unwrap())
        .unwrap();
    frame(&mut ui);
    assert!(native(&mut ui, KeyCode::Escape));
    assert_eq!(property(&ui, "composing"), InspectValue::Bool(false));
    assert_eq!(query.get().unwrap(), "eq");
    assert!(owner_keys.borrow().is_empty());
}

#[test]
fn inside_a_popup_the_first_escape_clears_and_the_second_closes() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let open = ui.signal(true);
    let query = ui.signal("pan".to_owned());
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "popover",
                Popover::new(
                    open.clone(),
                    Element::new("anchor", Label::new("Anchor")),
                    Element::new("search", SearchField::new(query.clone())),
                ),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    let field = ui.find_descendant(&root, "search").unwrap().unwrap();
    ui.focus(&field).unwrap();
    frame(&mut ui);
    assert!(native(&mut ui, KeyCode::Escape));
    assert_eq!(query.get().unwrap(), "");
    assert!(
        open.get().unwrap(),
        "the clearing Escape leaves the popup open"
    );
    assert!(native(&mut ui, KeyCode::Escape));
    assert!(
        !open.get().unwrap(),
        "an empty query leaves Escape to the popup"
    );
}

#[test]
fn escape_closes_the_fields_own_context_menu_before_clearing() {
    let Setup { mut ui, query, .. } = setup("mix", |field| field);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::F10);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    let menu_open = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .any(|c| c.key == "menu" && c.visible)
    };
    assert!(menu_open(&ui));
    assert!(native(&mut ui, KeyCode::Escape));
    assert!(!menu_open(&ui));
    assert_eq!(query.get().unwrap(), "mix");
}

#[test]
fn owner_keys_are_reported_without_moving_focus_or_caret() {
    let mut s = setup_reporting("abc");
    let ui = &mut s.ui;
    // The caret rests at the start after the binding was observed.
    let caret = property(ui, "selection");
    for code in [
        KeyCode::ArrowDown,
        KeyCode::ArrowUp,
        KeyCode::PageDown,
        KeyCode::PageUp,
        KeyCode::Enter,
    ] {
        press(ui, code);
    }
    // A held arrow repeats; a held Enter submits once.
    ui.key_input(KeyCode::ArrowDown, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::ArrowDown, KeyAction::Release)
        .unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    frame(ui);
    assert_eq!(
        *s.reported.borrow(),
        [
            SearchKey::Down,
            SearchKey::Up,
            SearchKey::PageDown,
            SearchKey::PageUp,
            SearchKey::Submit,
            SearchKey::Down,
            SearchKey::Down,
            SearchKey::Submit,
        ]
    );
    assert_eq!(property(&s.ui, "selection"), caret);
    assert_eq!(s.ui.focused(), Some(s.field.clone()));
    assert!(
        s.owner_keys.borrow().is_empty(),
        "reported keys are handled"
    );

    // With Shift the arrows select, as in any editor.
    s.ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)
        .unwrap();
    press(&mut s.ui, KeyCode::ArrowDown);
    s.ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    assert_eq!(s.reported.borrow().len(), 8);
    assert_ne!(property(&s.ui, "selection"), caret);
}

#[test]
fn owner_keys_reach_the_callback_through_the_native_text_phase() {
    let mut s = setup_reporting("abc");
    assert!(native(&mut s.ui, KeyCode::ArrowDown));
    assert!(native(&mut s.ui, KeyCode::Enter));
    assert_eq!(*s.reported.borrow(), [SearchKey::Down, SearchKey::Submit]);
}

#[test]
fn without_a_callback_owner_keys_bubble_to_ancestors() {
    let Setup {
        mut ui, owner_keys, ..
    } = setup("abc", |field| field);
    let caret = property(&ui, "selection");
    for code in [KeyCode::ArrowDown, KeyCode::PageUp, KeyCode::Enter] {
        press(&mut ui, code);
    }
    assert_eq!(*owner_keys.borrow(), ["ArrowDown", "PageUp", "Enter"]);
    assert_eq!(property(&ui, "selection"), caret);
    // Through the native path the text phase passes them on as well.
    assert!(native(&mut ui, KeyCode::ArrowUp));
    assert_eq!(owner_keys.borrow().last().unwrap(), "ArrowUp");
}

#[test]
fn the_clear_button_clears_and_appears_only_with_text() {
    let Setup {
        mut ui,
        field,
        query,
        ..
    } = setup("filter", |field| field);
    let bounds = ui.bounds(&field).unwrap().unwrap();
    let (x, y) = (bounds.width() - 20.0, bounds.height() / 2.0);
    click(&mut ui, x, y);
    assert_eq!(query.get().unwrap(), "");
    assert_eq!(text(&ui), "");
    assert_eq!(ui.focused(), Some(field.clone()));
    // Without text the slot is part of the editing area: a press there edits
    // nothing.
    click(&mut ui, x, y);
    assert_eq!(query.get().unwrap(), "");
    // Clearing is an edit the user can undo.
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::KeyZ);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    assert_eq!(query.get().unwrap(), "filter");
}

#[test]
fn a_read_only_field_neither_clears_nor_handles_escape() {
    let Setup {
        mut ui,
        query,
        owner_keys,
        ..
    } = setup("fixed", SearchField::read_only);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(query.get().unwrap(), "fixed");
    assert_eq!(*owner_keys.borrow(), ["Escape"]);
}

#[test]
fn a_debounced_query_waits_for_its_deadline_and_schedules_nothing_idle() {
    let Setup {
        mut ui,
        query,
        field,
        ..
    } = setup("", |field| field.debounce(Duration::from_millis(200)));
    // Unfocused and idle, the field schedules nothing (focused, its caret
    // blinks).
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(ui.next_deadline(), None);
    ui.focus(&field).unwrap();
    frame(&mut ui);
    ui.text_input("ga").unwrap();
    settle(&mut ui);
    assert_eq!(query.get().unwrap(), "");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(true));
    assert_eq!(text(&ui), "ga");
    ui.flush(Duration::from_millis(150)).unwrap();
    // Each edit restarts the deadline.
    ui.text_input("in").unwrap();
    ui.flush(Duration::from_millis(300)).unwrap();
    assert_eq!(query.get().unwrap(), "", "restarted at 150 ms");
    ui.flush(Duration::from_millis(350)).unwrap();
    assert_eq!(query.get().unwrap(), "gain");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(false));
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(query.get().unwrap(), "gain");
    assert_eq!(ui.next_deadline(), None, "no deadline remains once written");
}

#[test]
fn enter_focus_loss_and_clearing_write_a_pending_query_at_once() {
    let Setup {
        mut ui,
        query,
        field,
        ..
    } = setup("", |field| field.debounce(Duration::from_millis(500)));
    ui.text_input("eq").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(query.get().unwrap(), "eq", "Enter writes before Submit");

    ui.text_input(" low").unwrap();
    settle(&mut ui);
    ui.clear_focus();
    settle(&mut ui);
    assert_eq!(query.get().unwrap(), "eq low", "focus loss writes");

    ui.focus(&field).unwrap();
    frame(&mut ui);
    // Deleting everything leaves the bound query pending; Escape still clears.
    let modifier = KeyCode::primary();
    ui.key_input(modifier, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::KeyA);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    press(&mut ui, KeyCode::Backspace);
    assert_eq!(query.get().unwrap(), "eq low");
    press(&mut ui, KeyCode::Escape);
    assert_eq!(query.get().unwrap(), "");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(false));
}

#[test]
fn the_submit_report_follows_the_written_query() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let query = ui.signal(String::new());
    let submitted = ui.signal(String::new());
    let (read, write) = (query.clone(), submitted.clone());
    let field = ui
        .mount_tree(Element::new(
            "search",
            SearchField::new(query.clone())
                .debounce(Duration::from_millis(500))
                .on_key(move |_, key| {
                    if key == SearchKey::Submit {
                        write.set(read.get()?)?;
                    }
                    Ok(())
                }),
        ))
        .unwrap();
    ui.focus(&field).unwrap();
    frame(&mut ui);
    ui.text_input("comp").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(submitted.get().unwrap(), "comp");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(false));
}

#[test]
fn an_application_write_or_hiding_discards_a_pending_query() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let query = ui.signal(String::new());
    let shown = ui.signal(true);
    let root = ui
        .mount_tree(Element::new(
            "view",
            View::new(Element::new(
                "search",
                SearchField::new(query.clone()).debounce(Duration::from_millis(200)),
            ))
            .visible_signal(shown.read_only()),
        ))
        .unwrap();
    let field = ui.find_descendant(&root, "search").unwrap().unwrap();
    ui.focus(&field).unwrap();
    frame(&mut ui);

    ui.text_input("draft").unwrap();
    settle(&mut ui);
    query.set("external".to_owned()).unwrap();
    frame(&mut ui);
    assert_eq!(text(&ui), "external");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(false));
    ui.flush(Duration::from_secs(1)).unwrap();
    assert_eq!(
        query.get().unwrap(),
        "external",
        "the deadline writes nothing"
    );

    ui.text_input("more ").unwrap();
    settle(&mut ui);
    shown.set(false).unwrap();
    frame(&mut ui);
    shown.set(true).unwrap();
    frame(&mut ui);
    ui.flush(Duration::from_secs(2)).unwrap();
    assert_eq!(query.get().unwrap(), "external");
    assert_eq!(text(&ui), "external");
}

fn search_deadline(ui: &Runtime) -> Option<Duration> {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "search")
        .unwrap()
        .next_deadline
}

#[test]
fn an_application_write_wins_over_a_debounce_due_in_the_same_flush() {
    let Setup { mut ui, query, .. } = setup("", |field| field.debounce(Duration::from_millis(200)));
    let start = ui.now();
    ui.text_input("draft").unwrap();
    settle(&mut ui);
    // The application writes, and the next flush both delivers the write and
    // reaches the deadline; timers run before binding updates.
    query.set("external".to_owned()).unwrap();
    ui.flush(start + Duration::from_millis(300)).unwrap();
    frame(&mut ui);
    assert_eq!(query.get().unwrap(), "external");
    assert_eq!(text(&ui), "external");
    assert_eq!(property(&ui, "pending"), InspectValue::Bool(false));
}

#[test]
fn only_edits_restart_the_debounce_deadline() {
    let Setup { mut ui, query, .. } = setup("", |field| field.debounce(Duration::from_millis(200)));
    let start = ui.now();
    ui.text_input("gain").unwrap();
    settle(&mut ui);
    // Caret movement and pointer motion over the field edit nothing.
    ui.flush(start + Duration::from_millis(100)).unwrap();
    press(&mut ui, KeyCode::ArrowLeft);
    ui.pointer_input(PointerEvent::new(
        Point::new(40.0, 16.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(start + Duration::from_millis(150)).unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    ui.flush(start + Duration::from_millis(250)).unwrap();
    assert_eq!(
        query.get().unwrap(),
        "gain",
        "written 200 ms after the edit"
    );
}

#[test]
fn an_application_write_cancels_the_debounce_deadline() {
    let Setup { mut ui, query, .. } = setup("", |field| field.debounce(Duration::from_millis(200)));
    let start = ui.now();
    ui.text_input("draft").unwrap();
    settle(&mut ui);
    assert_eq!(
        search_deadline(&ui),
        Some(start + Duration::from_millis(200)),
        "the debounce is the earliest wake"
    );
    query.set("external".to_owned()).unwrap();
    settle(&mut ui);
    assert!(
        search_deadline(&ui).is_none_or(|due| due > start + Duration::from_millis(200)),
        "no wake remains for the discarded edit: {:?}",
        search_deadline(&ui)
    );
}

#[test]
fn disabled_redescription_discards_query_before_due_timer_or_focus_loss() {
    for focus_loss in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
            .unwrap();
        let query = ui.signal(String::new());
        let root = ui
            .mount_tree(Element::new("root", Column::default()))
            .unwrap();
        let describe = |disabled| {
            let field = SearchField::new(query.clone()).debounce(Duration::from_millis(200));
            vec![Element::new(
                "search",
                if disabled { field.disabled() } else { field },
            )]
        };
        ui.reconcile_children(&root, describe(false)).unwrap();
        frame(&mut ui);
        let field = ui.children(&root).unwrap()[0].clone();
        ui.focus(&field).unwrap();
        ui.text_input("pending").unwrap();
        ui.reconcile_children(&root, describe(true)).unwrap();
        if focus_loss {
            ui.clear_focus();
        }
        ui.flush(Duration::from_secs(1)).unwrap();
        assert_eq!(query.get().unwrap(), "");
        ui.reconcile_children(&root, describe(false)).unwrap();
        settle(&mut ui);
        assert_eq!(text(&ui), "");
    }
}
