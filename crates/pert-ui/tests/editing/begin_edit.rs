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
    // The press runs the activation, which only queues the command:
    // `key_input` does not flush.
    let label = labels.borrow()["0"].clone();
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert!(!editing(&ui, "0"), "nothing is delivered before a flush");
    // The one flush that delivers it opens the rename and focuses its
    // editor, before the release or any other flush.
    ui.flush(ui.now())?;
    assert!(editing(&ui, "0"), "the activation began the rename");
    assert_eq!(ui.focused(), Some(editor(&ui, &label)));
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
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
    // Removal applies in the pass that delivered the commands, so the
    // component leaves in the flush however its handler re-queues, and the
    // command it queued for itself is dropped with it.
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

/// A focusable owner that leaves when focus leaves it: it requests its own
/// unmount at an update that finds focus outside it after having had it.
struct Owner {
    child: Vec<Element>,
    had_focus: bool,
}
impl Component for Owner {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let within = cx.focus_within();
        if self.had_focus && !within {
            cx.request_unmount();
        }
        self.had_focus = within;
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.child)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 40.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        rows(cx, size)
    }
}

/// Places a container's children in equal rows, one below the other.
fn rows(cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
    let row = size.height() / cx.len().max(1) as f32;
    for index in 0..cx.len() {
        cx.place(
            index,
            Rect::new(0.0, row * index as f32, size.width(), row)?,
        )?;
    }
    Ok(())
}

/// A focusable child that requests its own unmount while `leave` is true,
/// and answers a text command by invalidating itself.
struct Departing {
    leave: Signal<bool>,
}
impl Component for Departing {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if cx.read_paint(&self.leave)? {
            cx.request_unmount();
        }
        Ok(())
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        cx.invalidate(Invalidation::Paint);
        Ok(TextCommandOutcome::Handled)
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 20.0)?)
    }
}

/// An `Owner` around a focused `Departing` child and `others`, with the
/// child's `leave`.
fn owner_with_focused_child(
    ui: &mut Runtime,
    others: Vec<Element>,
) -> Result<(WidgetId, WidgetId, Signal<bool>), Box<dyn std::error::Error>> {
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let leave = ui.signal(false);
    let mut children = vec![Element::new(
        "child",
        Departing {
            leave: leave.clone(),
        },
    )];
    children.extend(others);
    let owner = ui.mount_tree(Element::new(
        "owner",
        Owner {
            child: children,
            had_focus: false,
        },
    ))?;
    settle(ui);
    let child = widget(ui, &owner, "child");
    ui.focus(&child)?;
    settle(ui);
    Ok((owner, child, leave))
}

#[test]
fn a_removal_without_commands_restores_focus_before_the_parent_updates() -> TestResult {
    // Removing the focused child restores focus to its owner before the
    // owner's next update, so the owner still finds focus within it.
    let mut ui = Runtime::new();
    let (owner, _, leave) = owner_with_focused_child(&mut ui, Vec::new())?;
    leave.set(true)?;
    ui.flush(ui.now())?;
    assert_eq!(ui.focused(), Some(owner.clone()), "focus fell back to it");
    assert!(
        support::find(&ui, &owner, "child").is_none(),
        "the child left"
    );
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(owner), "the owner stayed");
    Ok(())
}

#[test]
fn a_command_whose_work_the_removal_ends_keeps_removal_after_focus() -> TestResult {
    // The command invalidates the departing child, and the removal ends
    // that work with the child: what is left is the removal's own, which
    // follows focus restoration as without the command.
    let mut ui = Runtime::new();
    let (owner, child, leave) = owner_with_focused_child(&mut ui, Vec::new())?;
    leave.set(true)?;
    // `event` focuses its target, so the focused child queues it itself.
    ui.event(&child, |cx| {
        cx.request_text_command(&child, TextCommand::SelectAll)
    })??;
    ui.flush(ui.now())?;
    assert_eq!(ui.focused(), Some(owner.clone()), "focus fell back to it");
    assert!(
        support::find(&ui, &owner, "child").is_none(),
        "the child left"
    );
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(owner), "the owner stayed");
    Ok(())
}

#[test]
fn a_command_for_a_surviving_sibling_keeps_removal_after_focus() -> TestResult {
    // A command whose target survives the removal leaves its own update
    // pending; the owner, which the removal invalidated, still finds focus
    // restored to it at its update.
    let mut ui = Runtime::new();
    let stays = ui.signal(false);
    let (owner, child, leave) = owner_with_focused_child(
        &mut ui,
        vec![Element::new("sibling", Departing { leave: stays })],
    )?;
    let sibling = widget(&ui, &owner, "sibling");
    leave.set(true)?;
    // `event` focuses its target, so the focused child queues it itself.
    ui.event(&child, |cx| {
        cx.request_text_command(&sibling, TextCommand::SelectAll)
    })??;
    ui.flush(ui.now())?;
    assert_eq!(ui.focused(), Some(owner.clone()), "focus fell back to it");
    assert!(
        support::find(&ui, &owner, "child").is_none(),
        "the child left"
    );
    settle(&mut ui);
    assert_eq!(ui.focused(), Some(owner), "the owner stayed");
    Ok(())
}

#[test]
fn a_queued_begin_edit_on_the_focused_label_moves_focus_into_its_editor() -> TestResult {
    // The focused label stops being focusable as the command begins the
    // edit; its focus still passes to the editor in the delivering flush.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let name = ui.signal("Draft".to_owned());
    let label = ui.mount_tree(Element::new("name", InlineText::new(name.clone())))?;
    settle(&mut ui);
    // `event` focuses the label, which queues the command for itself.
    let target = label.clone();
    ui.event(&label, |cx| {
        cx.request_text_command(&target, TextCommand::BeginEdit)
    })??;
    assert_eq!(ui.focused(), Some(label.clone()));
    ui.flush(ui.now())?;
    assert!(editing(&ui, "name"));
    assert_eq!(ui.focused(), Some(editor(&ui, &label)));
    ui.text_input("Final")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    assert_eq!(name.get()?, "Final");
    Ok(())
}

/// What a [`Reveal`] does: whether it is focusable (until its text command,
/// which ends that), whether its next update shows its children, whether
/// that update fails, which child its text command requests focus for, and
/// which component the command then passes a command on to.
#[derive(Default)]
struct Plan {
    focusable: Cell<bool>,
    show: Cell<bool>,
    fail: Cell<bool>,
    target: RefCell<Option<WidgetId>>,
    then: RefCell<Option<WidgetId>>,
}

/// A container whose children are hidden until an update finds
/// `plan.show`. Its text command requests focus for `plan.target` and
/// invalidates the container, so the request waits for that update: an
/// ordinary focus request, not a begun edit.
struct Reveal {
    children: Vec<Element>,
    plan: Rc<Plan>,
    shown: bool,
}
impl Component for Reveal {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.plan.fail.get() {
            return Err(RuntimeError::EmptyKey);
        }
        self.shown = self.plan.show.get();
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        self.shown
    }
    fn focusable(&self) -> bool {
        self.plan.focusable.get()
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        // As a label that begins its edit, it hands focus to a child.
        self.plan.focusable.set(false);
        if let Some(target) = self.plan.target.borrow().as_ref() {
            cx.request_focus_for(target)?;
        }
        if let Some(next) = self.plan.then.borrow().as_ref() {
            cx.request_text_command(next, TextCommand::SelectAll)?;
        }
        cx.invalidate(Invalidation::Paint);
        Ok(TextCommandOutcome::Handled)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 20.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        rows(cx, size)
    }
}

/// A component focusable while `focusable` holds, which answers a text
/// command by requesting focus for itself.
struct Target {
    focusable: Rc<Cell<bool>>,
}
impl Component for Target {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.focusable.get()
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        cx.request_focus();
        Ok(TextCommandOutcome::Handled)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 20.0)?)
    }
}

/// A button, a [`Reveal`] holding a hidden [`Target`] keyed "inner", a
/// second `Target` keyed "grabber" and a focusable [`Departing`] keyed
/// "leaver"; the button has focus.
struct Retention {
    ui: Runtime,
    plan: Rc<Plan>,
    inner_focusable: Rc<Cell<bool>>,
    leave: Signal<bool>,
    button: WidgetId,
    reveal: WidgetId,
    inner: WidgetId,
    grabber: WidgetId,
    leaver: WidgetId,
}
impl Retention {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let plan = Rc::new(Plan::default());
        let inner_focusable = Rc::new(Cell::new(true));
        let leave = ui.signal(false);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("button", Button::new("Button", |_| Ok(()))),
                Element::new(
                    "reveal",
                    Reveal {
                        children: vec![Element::new(
                            "inner",
                            Target {
                                focusable: inner_focusable.clone(),
                            },
                        )],
                        plan: plan.clone(),
                        shown: false,
                    },
                ),
                Element::new(
                    "grabber",
                    Target {
                        focusable: Rc::new(Cell::new(true)),
                    },
                ),
                Element::new(
                    "leaver",
                    Departing {
                        leave: leave.clone(),
                    },
                ),
            ]),
        ))?;
        settle(&mut ui);
        let button = widget(&ui, &root, "button");
        let reveal = widget(&ui, &root, "reveal");
        let inner = widget(&ui, &root, "inner");
        *plan.target.borrow_mut() = Some(inner.clone());
        ui.focus(&button)?;
        settle(&mut ui);
        Ok(Self {
            grabber: widget(&ui, &root, "grabber"),
            leaver: widget(&ui, &root, "leaver"),
            ui,
            plan,
            inner_focusable,
            leave,
            button,
            reveal,
            inner,
        })
    }
    /// Queues the reveal's command from `from`, which `event` focuses.
    fn command_from(&mut self, from: &WidgetId) -> TestResult {
        let reveal = self.reveal.clone();
        self.ui.event(from, |cx| {
            cx.request_text_command(&reveal, TextCommand::SelectAll)
        })??;
        Ok(())
    }
    fn inner_shown(&self) -> Result<bool, RuntimeError> {
        Ok(self.ui.bounds(&self.inner)?.is_some())
    }
}

#[test]
fn a_focus_request_waits_for_the_pending_update_that_shows_its_target() -> TestResult {
    // An ordinary request, not a begun edit: the rule is general.
    let mut r = Retention::new()?;
    r.plan.show.set(true);
    let button = r.button.clone();
    r.command_from(&button)?;
    r.ui.flush(r.ui.now())?;
    assert!(r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.inner.clone()), "in the same flush");
    Ok(())
}

#[test]
fn a_kept_request_survives_a_failed_flush_and_applies_on_the_retry() -> TestResult {
    let mut r = Retention::new()?;
    r.plan.show.set(true);
    r.plan.fail.set(true);
    let button = r.button.clone();
    r.command_from(&button)?;
    assert_eq!(r.ui.flush(r.ui.now()), Err(RuntimeError::EmptyKey));
    assert_eq!(r.ui.focused(), Some(r.button.clone()));
    r.plan.fail.set(false);
    r.ui.flush(r.ui.now())?;
    assert_eq!(
        r.ui.focused(),
        Some(r.inner.clone()),
        "the retry applies it"
    );
    Ok(())
}

#[test]
fn a_kept_request_ends_when_the_update_leaves_its_target_hidden() -> TestResult {
    let mut r = Retention::new()?;
    let button = r.button.clone();
    r.command_from(&button)?; // `show` is false: the update keeps it hidden
    r.ui.flush(r.ui.now())?;
    assert!(!r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.button.clone()));
    // A later reveal does not bring the request back.
    r.plan.show.set(true);
    r.ui.invalidate(&r.reveal, Invalidation::Paint)?;
    settle(&mut r.ui);
    assert!(r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.button.clone()));
    Ok(())
}

#[test]
fn an_abandoned_handover_restores_focus_from_the_unfocusable_owner() -> TestResult {
    // The focused reveal stops being focusable as it requests focus for its
    // hidden child, so it keeps focus while the request waits. Its update
    // leaves the child hidden: the request ends, and the owner's loss of
    // focusability restores focus as the ordinary path does.
    let mut r = Retention::new()?;
    r.plan.focusable.set(true);
    let reveal = r.reveal.clone();
    r.ui.focus(&reveal)?;
    settle(&mut r.ui);
    r.command_from(&reveal)?; // `show` is false
    r.ui.flush(r.ui.now())?;
    assert!(!r.inner_shown()?);
    assert_eq!(
        r.ui.focused(),
        Some(r.button.clone()),
        "focus restored to the first focusable component"
    );
    // A later reveal does not bring the request back.
    r.plan.show.set(true);
    r.ui.invalidate(&r.reveal, Invalidation::Paint)?;
    settle(&mut r.ui);
    assert!(r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.button.clone()));
    Ok(())
}

#[test]
fn a_focus_restoration_cancels_a_kept_request() -> TestResult {
    // The focused leaver departs in the pass that keeps the request: focus
    // restoration takes over, and the shown target gets no focus.
    let mut r = Retention::new()?;
    r.plan.show.set(true);
    r.leave.set(true)?;
    let leaver = r.leaver.clone();
    r.command_from(&leaver)?;
    r.ui.flush(r.ui.now())?;
    assert!(r.inner_shown()?);
    assert!(r.ui.focused().is_some(), "focus was restored");
    assert_ne!(r.ui.focused(), Some(r.inner.clone()));
    Ok(())
}

#[test]
fn a_newer_request_replaces_a_kept_one() -> TestResult {
    // A compatibility check: the latest request wins, kept or not.
    let mut r = Retention::new()?;
    r.plan.show.set(true);
    *r.plan.then.borrow_mut() = Some(r.grabber.clone());
    let button = r.button.clone();
    r.command_from(&button)?;
    r.ui.flush(r.ui.now())?;
    assert!(r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.grabber.clone()));
    Ok(())
}

#[test]
fn a_kept_request_for_a_target_no_longer_focusable_moves_nothing() -> TestResult {
    // A compatibility check: the kept request is applied under the usual
    // rules, so a target that stopped being focusable gets no focus.
    let mut r = Retention::new()?;
    r.plan.show.set(true);
    r.inner_focusable.set(false);
    let button = r.button.clone();
    r.command_from(&button)?;
    r.ui.flush(r.ui.now())?;
    assert!(r.inner_shown()?);
    assert_eq!(r.ui.focused(), Some(r.button.clone()));
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
