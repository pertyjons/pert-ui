A custom widget is a [`Component`]. The [getting-started
chapter](crate::guide::getting_started) shows a minimal one; this chapter covers
what a widget that is dragged, clicked, typed into and inspected needs, and how
an application around such a widget keeps its own document. The packaged
[diagram editor example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/diagram_editor/main.rs)
puts every section together: a canvas container with one child per node, pan
and zoom, selection, dragging, connecting, renaming in place, an inspector and
an outline over an application-owned document with its own undo history. Run
its deterministic headless session from a checkout with
`cargo run -p pert-ui --example diagram_editor --no-default-features --features cpu`,
or open it with `cargo run -p pert-ui --example diagram_editor --features cpu -- --window`.

# Who may write, and when

A component runs in phases. [`Component::update`] reads bindings through its
[`UpdateContext`] and records what it will show; measurement, arrangement,
painting and inspection read retained state. None of these may write a
binding: a write there fails with an error such as
[`RuntimeError::WriteDuringUpdate`]. Input callbacks
([`Component::pointer_input`], [`Component::keyboard_event`]), command and
button actions, mailbox handlers, [`Component::deliver_notifications`] and the
work queued with [`EventContext::defer`] run in event phases, where writes
are allowed.

A widget that must react to a binding it does not own, for example to commit a
value another control wrote, records what it saw in `update` and asks for an
event phase with a [`Notifier`]; [`Component::deliver_notifications`] then
writes. Prefer a control's own user-write callback where it has one (such as
[`ColorPicker::on_change`]), which reports only the user's writes:

```
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

/// Copies each new value of `source` into an application log, from an event phase.
struct Follow {
    source: ReadSignal<u32>,
    seen: Option<u32>,
    pending: Option<u32>,
    notifier: Option<Notifier>,
    log: Rc<RefCell<Vec<u32>>>,
}
impl Component for Follow {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let value = cx.read(&self.source)?;
        if self.seen != Some(value) {
            self.seen = Some(value);
            self.pending = Some(value);
            self.notifier.get_or_insert_with(|| cx.notifier()).request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(value) = self.pending.take() {
            self.log.borrow_mut().push(value); // writes are allowed here
        }
        Ok(())
    }
}
let mut ui = Runtime::new();
let count = ui.signal(1);
let log = Rc::new(RefCell::new(Vec::new()));
ui.mount_tree(Element::new("follow", Follow {
    source: count.read_only(), seen: None, pending: None, notifier: None, log: log.clone(),
}))?;
ui.flush(Duration::ZERO)?;
count.set(2)?;
ui.flush(Duration::ZERO)?;
assert_eq!(*log.borrow(), [1, 2]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Pointer input that converges

A widget that captures the pointer receives every move until the matching
release, also outside its bounds. After each layout or stacking change the
runtime repeats the last position as a move ([`PointerEvent::is_refresh`]), so
that geometry that changed under a stationary pointer is followed. A handler
must invalidate only when the state it derives from the event changed; a drag
that requests layout on every move never settles, and the flush ends in
[`RuntimeError::UpdateLimit`]. [`Component::pointer_input`] has a complete drag
that converges. In debug builds the `UpdateLimit` validation report names a
component whose handling of the repeated move kept scheduling work.

A gesture belongs to the button that started it: end it on that button's
release, not any release. Cancel it in [`Component::cancel_input`], which the
runtime calls when the widget loses its input context (Escape, hiding,
removal, a modal surface), and on [`Component::focus_lost`] where the gesture
should not outlive focus.

# Clicks and double clicks

[`ClickCounter`] counts a widget's presses as single, double and triple clicks
by the rule the built-in editors use: the same button within 500 ms and four
logical pixels of the previous press. Feed it each press the widget accepts
with [`ClickCounter::press_event`], and reset it when the input is cancelled,
when a press became a drag, and after a double click that does one thing such
as opening an item. A widget that cycles through actions with each press of a
series (word, then all text) keeps the series instead. Time is the runtime's:
a headless test advances it by flushing with a later time
([`EventContext::now`]).

# Keys held during a pointer gesture

A pointer handler sees the modifiers ([`EventContext::modifiers`]); other keys
reach the focused component through [`Component::keyboard_event`] with the
full [`KeyboardState`]. For a key that changes what a drag does, such as Space
to pan, [`Runtime::register_held_input`] gives the widget the key's press,
repeats and release, with a cancellation callback when focus leaves the window
or the widget is hidden. Held input is offered keys before the focused
control, so a registration that took Space whenever the widget was visible
would stop Space from pressing a focused button: the diagram editor returns
[`InputOutcome::Ignored`] for the press unless its canvas has focus, which
leaves the key to the focused control.

# Children the widget describes and focuses

A container describes its children with [`UpdateContext::request_children`]
and positions them in [`Component::arrange`]; a child under an unchanged key
keeps its state. Overlapping children must say so, or development validation
reports [`ValidationCode::UnexpectedOverlap`]: a stacking parent
([`Component::stacks_children`]) permits overlap among its children and orders
them by [`Component::child_stacking_priority`], and a component can report
[`OverlapPolicy::Allow`] in its [`ComponentDetails::validation`] instead.

To focus a child from an input callback, keep the child mounted and learn its
identity from [`UpdateContext::children`]; [`EventContext::request_focus_for`]
then moves focus once layout has settled. A child described in the same event
has no identity yet; describe it ahead of time and keep it hidden with
[`Component::child_visible`], as the rename editor below is kept.

To edit text in place, embed a [`DraftEditor`]: its
[`DraftEditorController::begin`] opens a session with the text selected
([`DraftSelection::All`]) or the caret at the end, the editor leaves Enter and
Escape to its owner, and the owner commits or discards the draft, reports
itself unfocusable while editing and applies its own policy when
[`Component::focus_within_changed`] reports that focus left. A [`TextField`]
instead writes a binding and suits a form.

# Inspection

[`Component::inspect`] describes the widget to tooling: a [`Semantics`] role,
name, value and state, and for a widget whose children are its own content,
semantics for each of them. A control without a visible label gets a name from
the application with [`Element::with_label`] or a [`Field`].

# An inspector over an application document

Controls edit signals; the application's document is its own. An inspector
that shows the selected item's properties keeps one signal per field for the
application's lifetime and binds the fields to them. Its structure follows the
selection through a [`Dynamic`]: nothing, many items, or one item, described
under a key that is that item's identity. Selecting another item therefore
mounts new fields, which discards a draft typed for the previous one instead
of saving it to the next, and each field's save callback captures the identity
it edits. Within one item, values change without describing the fields
again.

Every change to the document goes through the application's own operations,
which run in event phases and synchronize the fields there, writing only
values that differ: a draft being typed in one field survives any change that
leaves that field's value alone. An application write to a field replaces its
draft, and hiding or removing the field discards it, as each editor
documents. Each field reports its finished edit through its save callback
(`on_save`, with `.normalize(Ok)` for a text field that should save on Enter or
focus loss, `on_change` on choice controls and [`ColorPicker`]), and the
application applies it to the document as one undoable step. Before closing,
clearing focus ([`Runtime::clear_focus`]) saves the draft being typed, so that
close admission sees it as an unsaved change.

```
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Clone, PartialEq)]
struct Item { name: String }
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let items = Rc::new(RefCell::new(vec![Item { name: "First".into() }]));
let field = ui.signal(String::new());
// The application's operation: change the document, then synchronize.
let sync = {
    let (items, field) = (items.clone(), field.clone());
    move || field.set(items.borrow()[0].name.clone())
};
sync()?;
let (store, resync) = (items.clone(), sync.clone());
let name = TextField::new(&field).normalize(Ok).on_save(move |_, text| {
    store.borrow_mut()[0].name = text.to_owned(); // one undoable step here
    resync()
});
let root = ui.mount_tree(Element::new("name", name))?;
ui.flush(Duration::ZERO)?;
ui.focus(&root)?;
ui.key_input(KeyCode::End, KeyAction::Press)?;
ui.text_input(" item")?;
sync()?; // an unrelated change synchronizes again: the draft stays
ui.key_input(KeyCode::Enter, KeyAction::Press)?;
assert_eq!(items.borrow()[0].name, "First item");
# Ok::<(), Box<dyn std::error::Error>>(())
```

Two views of one selection, such as a canvas and an outline list, share one
[`ListSelection`] keyed by the items' identities instead of copying between
two stores; the list's [`VirtualList::on_selection_change`] tells the
application when the user changed it, so the inspector can follow.

# Building the application

Create the application's state first, then register commands whose actions
capture that state, then mount the tree that presents both; keep the command
ids apart from the state, so nothing waits for a value created later. Work
started on a background thread ([`Runtime::spawn`]) finishes later than the
user moves on: compare what it was started for (a document, a generation of
requests) with what is current before applying its result, and run writes to
one file one at a time. Work a
closing dialog starts belongs to the window rather than the dialog:
[`EventContext::defer_detached`] queues it there, whereas [`EventContext::defer`]
is skipped when its owner is hidden first. Close admission (`App::on_close`)
and error handling receive the [`Runtime`] but not what the initializer
created; share the application with them through a slot the initializer
fills, and decide what the callbacks do while it is still empty.
