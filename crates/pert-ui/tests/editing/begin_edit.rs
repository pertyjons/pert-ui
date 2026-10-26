//! Beginning an edit from code with `TextCommand::BeginEdit`: the resting
//! editors that answer it, when its focus request settles, routing and
//! admission, and the editors that have nothing to begin.
use crate::support;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    time::Duration,
};
use support::{settle, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn property(ui: &Runtime, key: &str, name: &str) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.properties.get(name).cloned())
}

fn editing(ui: &Runtime, key: &str) -> bool {
    property(ui, key, "editing") == Some(InspectValue::Bool(true))
}

/// A modal dialog closed over its anchor: a button and a row holding the
/// label `name`, which the tests begin to edit.
struct Fixture {
    ui: Runtime,
    name: Signal<String>,
    dialog: DialogController,
    other: WidgetId,
    row: WidgetId,
    label: WidgetId,
    inside: WidgetId,
}

fn fixture(
    label: InlineText,
    name: Signal<String>,
    ui: Runtime,
) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = ui;
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let dialog = DialogController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "owner",
        Dialog::new(
            dialog.clone(),
            Element::new(
                "anchor",
                Column::new([
                    Element::new("other", Button::new("Other", |_| Ok(()))),
                    Element::new("row", Row::new([Element::new("name", label)])),
                ]),
            ),
            Element::new("inside", Button::new("Inside", |_| Ok(()))),
        )
        .application_modal(),
    ))?;
    settle(&mut ui);
    let other = widget(&ui, &root, "other");
    let row = widget(&ui, &root, "row");
    let label = widget(&ui, &root, "name");
    let inside = widget(&ui, &root, "inside");
    Ok(Fixture {
        ui,
        name,
        dialog,
        other,
        row,
        label,
        inside,
    })
}

fn named(text: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    let name = ui.signal(text.to_owned());
    fixture(InlineText::new(name.clone()), name, ui)
}

/// The label's editor child, which holds focus while it edits.
fn editor(ui: &Runtime, label: &WidgetId) -> WidgetId {
    widget(ui, label, "editor")
}

#[test]
fn a_held_f2_is_a_repeat_and_begin_edit_starts_the_rename() -> TestResult {
    let mut f = named("Draft")?;
    // F2 is physically held while focus is elsewhere, as a tester's command
    // shortcut leaves it, so a synthesized press is a repeat.
    f.ui.focus(&f.other)?;
    f.ui.key_input(KeyCode::F2, KeyAction::Press)?;
    f.ui.focus(&f.label)?;
    f.ui.key_input(KeyCode::F2, KeyAction::Press)?;
    settle(&mut f.ui);
    assert!(
        !editing(&f.ui, "name"),
        "a repeated F2 leaves the label resting"
    );

    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    settle(&mut f.ui);
    assert_eq!(f.ui.focused(), Some(editor(&f.ui, &f.label)));
    f.ui.text_input("Final")?; // replaces the selected text
    f.ui.key_input(KeyCode::F2, KeyAction::Release)?;
    f.ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(f.name.get()?, "Final");
    Ok(())
}

#[test]
fn a_direct_begin_edit_edits_at_once_and_moves_focus_at_the_next_flush() -> TestResult {
    let mut f = named("Draft")?;
    f.ui.focus(&f.other)?;
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(editing(&f.ui, "name"));
    assert_eq!(
        property(&f.ui, "name", "draft"),
        Some(InspectValue::Text("Draft".into()))
    );
    assert_eq!(
        f.ui.focused(),
        Some(f.other.clone()),
        "the call does not flush"
    );
    settle(&mut f.ui);
    assert_eq!(f.ui.focused(), Some(editor(&f.ui, &f.label)));
    Ok(())
}

/// Rows of single-line labels keyed by their text, each an `InlineText` the
/// test can find.
struct Names(Vec<Signal<String>>);
impl ListItems for Names {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0.len()).then(|| index.to_string())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.0.get(index)?;
        Some(Element::new("title", InlineText::new(name)))
    }
}

#[test]
fn a_queued_begin_edit_from_on_activate_opens_the_focused_rename_in_that_flush() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let names = vec![
        ui.signal("First".to_owned()),
        ui.signal("Second".to_owned()),
    ];
    let rows = ui.signal(ListSource::new(Names(names.clone())));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let labels: Rc<RefCell<HashMap<String, WidgetId>>> = Rc::default();
    let targets = labels.clone();
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(rows.read_only(), 32.0)?
            .selection(selection.clone())
            .on_activate(move |cx, key| {
                if let Some(label) = targets.borrow().get(key) {
                    cx.request_text_command(label, TextCommand::BeginEdit)?;
                }
                Ok(())
            }),
    ))?;
    settle(&mut ui);
    for key in ["0", "1"] {
        // The list keys each row's label with the row's key.
        let label = ui.find(&list, &[key])?.ok_or("a row label")?;
        labels.borrow_mut().insert(key.to_owned(), label);
    }
    ui.focus(&list)?;
    support::press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(selection.active().get()?.as_deref(), Some("0"));
    // The driver's Enter ends with exactly one flush, which delivers the
    // queued command: when the call returns, with no other input or flush,
    // the rename is open and its editor focused.
    let label = labels.borrow()["0"].clone();
    {
        let mut driver = testing::Driver::new(&mut ui);
        driver.key(KeyCode::Enter)?;
        let ui = driver.runtime();
        assert!(editing(ui, "0"), "the activation began the rename");
        assert_eq!(ui.focused(), Some(editor(ui, &label)));
    }
    ui.text_input("Renamed")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(names[0].get()?, "Renamed");
    assert_eq!(names[1].get()?, "Second");
    Ok(())
}

/// A component that asks to leave at its update and answers every text
/// command by changing itself and queuing the command for itself again.
struct Leaver {
    leave: Signal<bool>,
    commands: Rc<Cell<usize>>,
}
impl Component for Leaver {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx.read_paint(&self.leave)? {
            cx.request_unmount();
        }
        Ok(())
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        self.commands.set(self.commands.get() + 1);
        cx.invalidate(Invalidation::Paint);
        let own = cx.id();
        cx.request_text_command(&own, command)?;
        Ok(TextCommandOutcome::Handled)
    }
}

#[test]
fn a_requested_unmount_applies_before_queued_commands_repeat_their_updates() -> TestResult {
    // Running a command's updates before focus must not starve removal: the
    // component leaves in the flush, and the command it queued for itself is
    // dropped with it.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let leave = ui.signal(false);
    let commands = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("other", Button::new("Other", |_| Ok(()))),
            Element::new(
                "leaver",
                Leaver {
                    leave: leave.clone(),
                    commands: commands.clone(),
                },
            ),
        ]),
    ))?;
    settle(&mut ui);
    let other = widget(&ui, &root, "other");
    let leaver = widget(&ui, &root, "leaver");
    leave.set(true)?;
    ui.event(&other, |cx| {
        cx.request_text_command(&leaver, TextCommand::SelectAll)
    })??;
    ui.flush(ui.now())?;
    assert!(support::find(&ui, &root, "leaver").is_none(), "it left");
    assert_eq!(commands.get(), 1, "its re-queued command was dropped");
    Ok(())
}

#[test]
fn begin_edit_reaches_the_label_itself_or_as_the_focused_descendant() -> TestResult {
    let mut f = named("Draft")?;
    // A resting label has no focused descendant, so its row is no editor.
    f.ui.focus(&f.other)?;
    assert_eq!(
        f.ui.text_command(&f.row, TextCommand::BeginEdit)?,
        TextCommandOutcome::NotAnEditor
    );
    assert!(!editing(&f.ui, "name"));
    // With the resting label focused, the row passes the command on to it.
    f.ui.focus(&f.label)?;
    assert_eq!(
        f.ui.text_command(&f.row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(editing(&f.ui, "name"));
    Ok(())
}

#[test]
fn a_direct_begin_edit_refuses_hidden_and_modal_blocked_labels() -> TestResult {
    let mut f = named("Draft")?;
    f.ui.set_visible(&f.label, false)?;
    settle(&mut f.ui);
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit),
        Err(RuntimeError::Hidden)
    );
    f.ui.set_visible(&f.label, true)?;
    f.dialog.open()?;
    settle(&mut f.ui);
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit),
        Err(RuntimeError::ModalBlocked)
    );
    assert!(!editing(&f.ui, "name"));
    Ok(())
}

#[test]
fn a_queued_begin_edit_is_dropped_for_a_hidden_target_and_fails_for_a_blocked_one() -> TestResult {
    let mut f = named("Draft")?;
    let label = f.label.clone();
    f.ui.event(&f.other, |cx| {
        cx.request_text_command(&label, TextCommand::BeginEdit)
    })??;
    f.ui.set_visible(&f.label, false)?;
    settle(&mut f.ui);
    f.ui.set_visible(&f.label, true)?;
    settle(&mut f.ui);
    assert!(
        !editing(&f.ui, "name"),
        "the hidden target's request is dropped"
    );

    f.dialog.open()?;
    settle(&mut f.ui);
    f.ui.event(&f.inside, |cx| {
        cx.request_text_command(&label, TextCommand::BeginEdit)
    })??;
    assert_eq!(f.ui.flush(f.ui.now()), Err(RuntimeError::ModalBlocked));
    assert!(!editing(&f.ui, "name"));

    // An unmounted target's request is dropped too.
    let mut f = named("Draft")?;
    let label = f.label.clone();
    f.ui.event(&f.other, |cx| {
        cx.request_text_command(&label, TextCommand::BeginEdit)
    })??;
    f.ui.unmount(&f.row)?;
    f.ui.flush(f.ui.now())?;
    assert_eq!(f.name.get()?, "Draft");
    Ok(())
}

#[test]
fn begin_edit_is_unavailable_while_editing_disabled_or_refused() -> TestResult {
    let mut f = named("Draft")?;
    f.ui.text_command(&f.label, TextCommand::BeginEdit)?;
    settle(&mut f.ui);
    f.ui.text_input("Typed")?;
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    // The draft is not selected again: the next text appends.
    f.ui.text_input("!")?;
    assert_eq!(
        property(&f.ui, "name", "draft"),
        Some(InspectValue::Text("Typed!".into()))
    );

    let ui = Runtime::new();
    let name = ui.signal("Fixed".to_owned());
    let mut f = fixture(InlineText::new(name.clone()).disabled(), name, ui)?;
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!editing(&f.ui, "name"));

    // A bound value the editor does not accept is refused.
    let mut f = named("Draft")?;
    f.name.set("Two\nlines".to_owned())?;
    settle(&mut f.ui);
    assert!(property(&f.ui, "name", "binding_error").is_some());
    assert_eq!(
        f.ui.text_command(&f.label, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!editing(&f.ui, "name"));
    assert_eq!(f.name.get()?, "Two\nlines");
    Ok(())
}

/// Mounts an inline number row as "field" above a button.
fn number_row(ui: &mut Runtime, row: impl Component + 'static) -> Result<WidgetId, RuntimeError> {
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("field", row),
            Element::new("next", Button::new("Next", |_| Ok(()))),
        ]),
    ))?;
    settle(ui);
    Ok(ui.children(&root)?[0].clone())
}

fn text_editing(ui: &Runtime) -> bool {
    property(ui, "field", "text_editing") == Some(InspectValue::Bool(true))
}

/// A point on the row's value, where a drag steps it.
fn value_point(ui: &Runtime, id: &WidgetId) -> Result<Point, Box<dyn std::error::Error>> {
    let [x, y, w, h] = ui.bounds(id)?.ok_or("laid out")?.values();
    Ok(Point::new(x + w - 12.0, y + h / 2.0)?)
}

#[test]
fn begin_edit_enters_an_inline_number_rows_text_mode_at_rest() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(8_i64);
    let row = number_row(&mut ui, InlineInteger::new(count.clone()).range(0, 100)?)?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    settle(&mut ui);
    ui.text_input("42")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    settle(&mut ui);
    assert_eq!(count.get()?, 42);

    let mut ui = Runtime::new();
    let level = ui.signal(0.25_f64);
    let row = number_row(&mut ui, InlineDecimal::new(level.clone()).range(0.0, 1.0)?)?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    Ok(())
}

#[test]
fn begin_edit_waits_for_an_inline_number_rows_step_gesture_to_end() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(10_i64);
    let row = number_row(&mut ui, InlineInteger::new(count.clone()).range(0, 100)?)?;
    // A step drag.
    let start = value_point(&ui, &row)?;
    ui.pointer_input(PointerEvent::new(
        start,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    let moved = Point::new(start.x() + 8.0, start.y())?;
    ui.pointer_input(PointerEvent::new(moved, PointerKind::Move))?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!text_editing(&ui));
    ui.pointer_input(PointerEvent::new(
        moved,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    settle(&mut ui);
    assert_eq!(count.get()?, 12, "the drag saves on release as before");
    assert!(!text_editing(&ui));

    // A held step key: a parameter row's one session per held key, which
    // its release commits.
    let mut ui = Runtime::new();
    let (voices, _host) = ui.parameter(
        ParameterId::new(1, 1, "voices")?,
        ParameterSpec::stepped("Voices", 0, 16, 4)?,
    )?;
    let row = number_row(&mut ui, InlineInteger::for_parameter(voices.clone()))?;
    ui.focus(&row)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!text_editing(&ui));
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    settle(&mut ui);
    assert_eq!(
        voices.snapshot().get()?.base,
        5,
        "the release commits as before"
    );
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled,
        "the gesture has ended"
    );
    Ok(())
}

#[test]
fn begin_edit_is_unavailable_on_a_disabled_inline_number_row() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(8_i64);
    let row = number_row(&mut ui, InlineInteger::new(count.clone()).disabled())?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!text_editing(&ui));
    Ok(())
}

#[test]
fn begin_edit_enters_read_only_text_mode_without_edit_authority() -> TestResult {
    let mut ui = Runtime::new();
    let count = ui.signal(8_i64);
    let row = number_row(&mut ui, InlineInteger::new(count.clone()).read_only())?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    settle(&mut ui);
    ui.text_input("42")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    settle(&mut ui);
    assert_eq!(count.get()?, 8, "a read-only row saves nothing");

    let mut ui = Runtime::new();
    let (voices, _host) = ui.parameter(
        ParameterId::new(1, 1, "voices")?,
        ParameterSpec::stepped("Voices", 0, 16, 4)?,
    )?;
    let row = number_row(
        &mut ui,
        InlineInteger::for_parameter(voices.clone()).read_only(),
    )?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    settle(&mut ui);
    ui.text_input("9")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    settle(&mut ui);
    assert_eq!(voices.snapshot().get()?.base, 4);
    Ok(())
}

fn amount(
    ui: &mut Runtime,
    spec: ParameterSpec<f32>,
) -> Result<(Parameter<f32>, ParameterHost<f32>), RuntimeError> {
    ui.parameter(ParameterId::new(1, 1, "amount")?, spec)
}

#[test]
fn begin_edit_enters_an_inline_sliders_text_mode_at_rest() -> TestResult {
    let mut ui = Runtime::new();
    let (amount, _host) = amount(&mut ui, ParameterSpec::new("Amount", 0.0, 100.0, 50.0)?)?;
    let row = number_row(&mut ui, InlineSlider::new(amount.clone()))?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(editor(&ui, &row)), "focus at the flush");
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable,
        "already in text mode"
    );
    ui.text_input("42.5")?; // replaces the selected text
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    settle(&mut ui);
    assert_eq!(amount.snapshot().get()?.base, 42.5);
    assert!(!text_editing(&ui));
    Ok(())
}

#[test]
fn begin_edit_is_unavailable_where_an_inline_sliders_enter_does_nothing() -> TestResult {
    // Disabled, and a parameter that takes no edit: Enter and F2 do nothing.
    for case in ["disabled", "read-only"] {
        let mut ui = Runtime::new();
        let spec = ParameterSpec::new("Amount", 0.0, 100.0, 50.0)?;
        let (row, _host) = if case == "disabled" {
            let (amount, host) = amount(&mut ui, spec)?;
            (
                number_row(&mut ui, InlineSlider::new(amount).disabled())?,
                host,
            )
        } else {
            let (amount, host) = amount(&mut ui, spec.read_only())?;
            (number_row(&mut ui, InlineSlider::new(amount))?, host)
        };
        for code in [KeyCode::Enter, KeyCode::F2] {
            ui.focus(&row)?;
            support::press(&mut ui, code);
            assert!(!text_editing(&ui), "{case}: {code:?} does nothing");
        }
        assert_eq!(
            ui.text_command(&row, TextCommand::BeginEdit)?,
            TextCommandOutcome::Unavailable,
            "{case}"
        );
        assert!(!text_editing(&ui), "{case}");
    }
    Ok(())
}

#[test]
fn begin_edit_waits_for_an_inline_sliders_gesture_to_end() -> TestResult {
    // A drag on the value.
    let mut ui = Runtime::new();
    let (amount, _host) = amount(&mut ui, ParameterSpec::new("Amount", 0.0, 100.0, 50.0)?)?;
    let row = number_row(&mut ui, InlineSlider::new(amount.clone()))?;
    let start = value_point(&ui, &row)?;
    ui.pointer_input(PointerEvent::new(
        start,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable,
        "a pressed value, before it moves"
    );
    let moved = Point::new(start.x() + 16.0, start.y())?;
    ui.pointer_input(PointerEvent::new(moved, PointerKind::Move))?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!text_editing(&ui));
    ui.pointer_input(PointerEvent::new(
        moved,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    settle(&mut ui);
    assert!(
        (amount.snapshot().get()?.base - 60.0).abs() < 0.0001,
        "the drag saves on release as before"
    );
    assert!(!text_editing(&ui));

    // A held step key: one edit per held key, which its release commits.
    let mut ui = Runtime::new();
    let (amount, _host) = self::amount(&mut ui, ParameterSpec::new("Amount", 0.0, 100.0, 50.0)?)?;
    let row = number_row(&mut ui, InlineSlider::new(amount.clone()))?;
    ui.focus(&row)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert!(!text_editing(&ui));
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    settle(&mut ui);
    assert_eq!(amount.snapshot().get()?.base, 51.0, "the release commits");
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled,
        "the gesture has ended"
    );
    Ok(())
}

#[test]
fn begin_edit_opens_an_inline_readouts_read_only_text_mode() -> TestResult {
    let mut ui = Runtime::new();
    let load = ui.signal("42 %".to_owned());
    let row = number_row(&mut ui, InlineReadout::new(load.read_only()))?;
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    assert!(text_editing(&ui));
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(editor(&ui, &row)), "focus at the flush");
    assert_eq!(
        ui.text_command(&row, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable,
        "already in text mode"
    );
    ui.text_input("typed")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    settle(&mut ui);
    assert_eq!(load.get()?, "42 %", "nothing is written");
    Ok(())
}

#[test]
fn a_slider_without_text_mode_is_no_editor() -> TestResult {
    // A compatibility check: a slider has no resting text mode to begin.
    let mut ui = Runtime::new();
    let (amount, _host) = amount(&mut ui, ParameterSpec::new("Amount", 0.0, 100.0, 50.0)?)?;
    let slider = number_row(&mut ui, Slider::new(amount))?;
    assert_eq!(
        ui.text_command(&slider, TextCommand::BeginEdit)?,
        TextCommandOutcome::NotAnEditor
    );
    Ok(())
}

#[test]
fn editors_without_a_resting_state_have_nothing_to_begin() -> TestResult {
    // A compatibility check: these answers are the same without the command's
    // resting-state handling.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let text = ui.signal("Text".to_owned());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("field", TextField::new(text.clone())),
            Element::new("selectable", SelectableText::new("Selectable")),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let field = widget(&ui, &root, "field");
    let selectable = widget(&ui, &root, "selectable");
    assert_eq!(
        ui.text_command(&field, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert_eq!(
        ui.text_command(&selectable, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    assert_eq!(text.get()?, "Text");
    Ok(())
}
