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

# Imports

Application code globs [`prelude`]. A widget module globs [`component`],
which holds every name implementing [`Component`] writes: its contexts,
drawing, input events, drag and drop, receipts such as a
[`ShownTaskReceipt`], native windows, editor sessions, the [`DraftEditor`] a
widget embeds, [`Element`] and the signals. A module that also uses built-in components or
runs a [`Runtime`], such as a container that describes built-in controls or
the widget's tests, adds [`prelude`]; the two globs combine, since they
re-export the same items. A module globs only what it uses: a glob none of
whose names the module uses is an `unused import` warning. This widget needs
only [`component`]:

```
use pert_ui::component::*;

struct Marker;

impl Component for Marker {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Image).name("marker")),
            ..ComponentDetails::default()
        }
    }
}
# let _ = Marker;
```

The [`component`] page has an example that mounts such a widget, with both
globs. The examples below glob the modules they use and import the few other
names they write from the crate root by name, never by a crate-root glob.

# A component that follows a binding

A component reads the bindings it shows in [`Component::update`] and retains
what it read; the runtime updates it again only when one of them changes, and
not while it is hidden:

```rust
use std::time::Duration;
use pert_ui::{component::*, prelude::*};

// A custom component that follows a text binding.
struct Status {
    text: ReadSignal<String>,
    retained: String,
}
impl Component for Status {
    fn update(&mut self, context: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.retained = context.read(&self.text)?;
        Ok(())
    }
}

let mut runtime = Runtime::new();
let text = runtime.signal(String::from("Ready"));
let status = Status { text: text.read_only(), retained: String::new() };
let id = runtime.mount_tree(Element::new("status", status))?;
assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
assert_eq!(runtime.next_deadline(), None);
text.set(String::from("Working"))?;
text.set(String::from("Done"))?;
assert_eq!(runtime.flush(Duration::ZERO)?.components_updated, 1);
runtime.set_visible(&id, false)?;
text.set(String::from("Hidden update"))?;
assert_eq!(runtime.next_deadline(), None);
runtime.unmount(&id)?;
# Ok::<(), RuntimeError>(())
```

Components retain their own presentation data. Every update recollects all
dependencies; ordinary reads invalidate layout and paint, while `read_paint` is
for data confined to fixed geometry. No writes are allowed during update, paint
or inspection. Changed signals only enqueue dependents, and hidden components
retain dirtiness without evaluation. `animate()` uses the per-window
presentation ceiling (60 Hz when unknown); no catch-up frames are generated.
A timed action that must reach application state uses the separate event-phase
timer instead: [`EventContext::set_timer`] arms one deadline for the component and
[`Component::timer_elapsed`] runs in the event phase of the flush that reaches it,
so a held repeat or auto-scroll may write bindings and commit edits like ordinary
input. The timer is one-shot, replaces a pending one, and is cancelled with the
component's input context on focus loss, hiding, unmount or `cancel_input`.
Components record solid rectangles, shapes, vector paths, text and images through
[`Painter`]; runtime retains their commands and copies the visible scene on `paint`.
Unchanged paint callbacks are not repeated. GPU resources belong to the selected
graphics adapter; text preparation and coverage are cached separately from GPU
resources.

# Who may write, and when

A component runs in phases. [`Component::update`] reads bindings through its
[`UpdateContext`] and records what it will show; measurement, arrangement,
painting and inspection read retained state. None of these may write a
binding: a write there fails with an error such as
[`RuntimeError::WriteDuringUpdate`]. Input callbacks
([`Component::pointer_input`], [`Component::keyboard_event`]), command and
button actions, mailbox handlers, [`Component::deliver_notifications`] and the
work queued with [`EventContext::defer`] or, for a component's work on itself
that ends when it is hidden or removed, [`EventContext::defer_while_shown`]
run in event phases, where writes are allowed. [`Component::cancel_input`] also runs outside the evaluation
phases: it may restore the widget's own bindings but must not run application
actions, and its rustdoc says how a failed restore is reported.

A widget that must react to a binding it does not own, for example to commit a
value another control wrote, records what it saw in `update` and asks for an
event phase with a [`Notifier`]; [`Component::deliver_notifications`] then
writes. Prefer a control's own user-write callback where it has one (such as
[`ColorPicker::on_change`]), which reports only the user's writes:

```
use pert_ui::{component::*, prelude::*};
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
runtime calls when the widget loses its input context (hiding, removal, window
focus loss, a modal surface, a host's
[`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input)), and on
[`Component::focus_lost`] where the gesture should not outlive focus. Escape's
ordinary delivery never calls `cancel_input`: when no drag session, delayed
help, editor or open surface takes it first ([`Runtime::key_input`] has the
order), the runtime ends the focused widget's pointer capture, timer, drag and
parameter edit sessions and then delivers Escape to
[`Component::keyboard_event`] as an ordinary key. When one of them takes it,
that cleanup is skipped, and only the consumer's own lifecycle cleans up:
cancelling a drag releases its source's capture and reports the cancellation
to [`Component::drag_finished`], and closing a surface hides what it showed,
which calls `cancel_input` on each hidden widget. A widget whose gesture
Escape should cancel cancels it in `keyboard_event`, as [`ColorPicker`] does
and the next section's control does.

# A control that edits a value

A control that edits a binding decides first when it writes. With **live
writes** each step of a gesture writes the binding, so everything bound to the
value follows the drag, and the control remembers the value from before the
gesture to write it back when the gesture is cancelled. With a **commit** the
gesture changes only a preview the control keeps and writes once at its end,
as [`ColorPicker`] does; cancelling then drops the preview and has nothing to
restore. The control below writes live and follows the rest of the gesture
contract:

- **Escape** reaches it as an ordinary key in [`Component::keyboard_event`],
  after the runtime has ended its pointer capture; the control restores the
  value there, in an event phase, and a failed restore is returned from the
  key's input call.
- **[`Component::cancel_input`]** restores it as well. The runtime calls it
  when the control loses its input context, such as on hiding, removal, focus
  loss or a modal surface, and it is the only restore that also runs on
  removal. It must not run application actions, so it never calls
  `on_change`.
- **`on_change`** reports each write the control makes from an input
  callback: a drag step, a key step and Escape's restore. The restore in
  `cancel_input` is not reported; an owner that must see every change follows
  the binding itself.
- **A restore can fail,** with the errors of [`Signal::set`], or be refused by
  the control, as here when the application lowered the limit during the drag
  below the value it started from. `cancel_input` returns nothing, so it
  records the error and requests a [`Notifier`], and
  [`Component::deliver_notifications`] returns it from a later flush. That
  route reports only while the control stays mounted: a removed control is not
  called there, so a restore that fails during removal is not reported.
- **Readings are validated** before they replace what the control shows.
  It supports a value and a limit in 0..=1; `update` refuses a reading
  outside that range, NaN included, with an error, which fails the flush, and
  keeps its previous presentation, so pointer and key input never compute
  with an invalid limit. The next valid reading is shown again.
- **The focus ring** is the built-in one, drawn as
  [`ColorRole::FocusRing`] describes: [`Painter::stroke_shape`] on the
  control's bounds at [`MetricRole::FocusWidth`] while
  [`Painter::focus_within`] holds.

```
use pert_ui::{component::*, prelude::*};
use std::{cell::RefCell, rc::Rc, time::Duration};

/// Edits `value` from 0 up to `limit`, which the application may lower; both
/// lie in 0..=1.
struct Level {
    value: Signal<f32>,
    limit: ReadSignal<f32>,
    shown: (f32, f32),
    /// The value from before the drag, while one is open.
    origin: Option<f32>,
    /// A restore `cancel_input` could not make, for `deliver_notifications`.
    failed: Option<RuntimeError>,
    on_change: Box<dyn FnMut(&mut EventContext, f32) -> Result<(), RuntimeError>>,
}
impl Level {
    /// Writes a user's value and reports it; called only from input callbacks.
    fn write(&mut self, cx: &mut EventContext, value: f32) -> Result<(), RuntimeError> {
        if self.value.set_changed(value)? {
            (self.on_change)(cx, value)?;
        }
        Ok(())
    }
    /// Ends the drag and writes the value from before it back, if it still fits.
    fn restore(&mut self) -> Result<Option<f32>, RuntimeError> {
        let Some(origin) = self.origin.take() else { return Ok(None) };
        if origin > self.shown.1 {
            return Err(ApplicationError::from("the value before the drag exceeds the limit").into());
        }
        self.value.set(origin)?;
        Ok(Some(origin))
    }
    fn at(&self, cx: &EventContext, event: PointerEvent) -> Option<f32> {
        let (local, size) = (cx.to_local(event.position())?, cx.size()?);
        Some((local.x() / size.width()).clamp(0.0, self.shown.1))
    }
}
impl Component for Level {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let (value, limit) = (cx.read(&self.value.read_only())?, cx.read(&self.limit)?);
        // Refused readings, NaN included, keep what the control shows.
        if !(0.0..=1.0).contains(&value) || !(0.0..=1.0).contains(&limit) {
            return Err(ApplicationError::from("a level and its limit lie in 0..=1").into());
        }
        self.shown = (value, limit);
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(160.0, cx.metric(MetricRole::ControlHeight))?)
    }
    fn focusable(&self) -> bool { true }
    fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
        -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                cx.request_focus();
                self.origin = Some(self.value.get()?);
            }
            PointerKind::Move if self.origin.is_some() => {}
            PointerKind::Release(PointerButton::Primary) if self.origin.is_some() => {
                self.origin = None; // the drag's last write stands
                cx.release_pointer();
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        if let Some(value) = self.at(cx, event) {
            self.write(cx, value)?; // a repeated position writes nothing
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(&mut self, cx: &mut EventContext, event: KeyEvent, _: &KeyboardState)
        -> Result<InputOutcome, RuntimeError> {
        if event.transition == KeyTransition::Released {
            return Ok(InputOutcome::Ignored);
        }
        let current = self.value.get()?;
        match event.code {
            KeyCode::Escape => match self.restore()? {
                Some(origin) => (self.on_change)(cx, origin)?, // the user's write too
                None => return Ok(InputOutcome::Ignored),
            },
            KeyCode::ArrowLeft => self.write(cx, (current - 0.1).max(0.0))?,
            KeyCode::ArrowRight => self.write(cx, (current + 0.1).min(self.shown.1))?,
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        if let Err(error) = self.restore() {
            self.failed = Some(error);
            cx.notifier().request();
        }
        self.input_reset();
        cx.invalidate(Invalidation::Paint);
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.failed.take().map_or(Ok(()), Err)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let bounds = Rect::from_size(painter.size());
        painter.fill_rect(bounds, painter.color(ColorRole::Surface));
        let filled = Rect::new(0.0, 0.0, bounds.width() * self.shown.0, bounds.height())?;
        painter.fill_rect(filled, painter.color(ColorRole::Primary));
        if painter.focus_within() {
            let width = Length::new(painter.metric(MetricRole::FocusWidth))?;
            painter.stroke_shape(bounds, Shape::Rectangle, width, painter.color(ColorRole::FocusRing))?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Slider).name("Level")
                .value(SemanticValue::Number {
                    value: f64::from(self.shown.0),
                    minimum: Some(0.0),
                    maximum: Some(f64::from(self.shown.1)),
                })
                .actions([SemanticAction::Increment, SemanticAction::Decrement])),
            ..ComponentDetails::default()
        }
    }
}

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
let (value, limit) = (ui.signal(0.8_f32), ui.signal(1.0_f32));
let reported = Rc::new(RefCell::new(Vec::new()));
let level = |reported: Rc<RefCell<Vec<f32>>>| Level {
    value: value.clone(), limit: limit.read_only(), shown: (0.0, 0.0), origin: None, failed: None,
    on_change: Box::new(move |_, value| Ok(reported.borrow_mut().push(value))),
};
let root = ui.mount_tree(Element::new("form", Column::new([
    Element::new("level", level(reported.clone())),
    Element::new("other", TextField::new(ui.signal(String::new()))),
])))?;
ui.flush(Duration::ZERO)?;
let control = ui.find(&root, &["level"])?.ok_or("the level")?;
let other = ui.find(&root, &["other"])?.ok_or("the field")?;
let [x, y, ..] = ui.bounds(&control)?.ok_or("laid out")?.values();
let at = |dx: f32, kind| Point::new(x + dx, y + 4.0).map(|point| PointerEvent::new(point, kind));
let press = |dx| at(dx, PointerKind::Press(PointerButton::Primary));

// A drag writes live; Escape writes the value from before it back.
ui.pointer_input(press(80.0)?)?;
ui.pointer_input(at(40.0, PointerKind::Move)?)?;
assert_eq!(value.get()?, 0.25);
ui.key_input(KeyCode::Escape, KeyAction::Press)?;
assert_eq!(value.get()?, 0.8);
assert_eq!(*reported.borrow(), [0.5, 0.25, 0.8]);
ui.key_input(KeyCode::Escape, KeyAction::Release)?;

// The focused control draws the focus ring: four sides in the ring's color,
// made distinct here from `Primary`, which built-in palettes match.
let ring = Color::from_linear(1.0, 0.0, 0.5, 1.0)?;
ui.theme().set_color(ui.theme().mode()?, ColorRole::FocusRing, ring)?;
ui.flush(Duration::ZERO)?;
let sides = ui.paint()?.commands().iter()
    .filter(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == ring))
    .count();
assert_eq!(sides, 4);

// Focus loss restores in `cancel_input`, which reports nothing to `on_change`.
ui.pointer_input(press(40.0)?)?;
ui.focus(&other)?;
assert_eq!(value.get()?, 0.8);
assert_eq!(reported.borrow().len(), 4);

// A restore that fails while the control stays mounted is reported by a later flush.
ui.pointer_input(press(40.0)?)?;
limit.set(0.5)?;
ui.flush(Duration::ZERO)?; // the control reads the lower limit
ui.focus(&other)?;
assert!(ui.flush(Duration::ZERO).is_err());
assert_eq!(value.get()?, 0.25); // the drag's value stands
ui.flush(Duration::ZERO)?; // reported once

// A limit outside 0..=1 is refused, and the control keeps the one it shows.
let shown_limit = |ui: &Runtime| ui.inspect().components.into_iter()
    .find(|component| component.key == "level")
    .and_then(|component| component.details.semantics?.value);
limit.set(f32::NAN)?;
assert!(ui.flush(Duration::ZERO).is_err());
assert!(matches!(shown_limit(&ui), Some(SemanticValue::Number { maximum: Some(0.5), .. })));
limit.set(0.5)?;
ui.flush(Duration::ZERO)?;

// One that fails while the control is removed is not reported.
value.set(0.5)?;
ui.pointer_input(press(40.0)?)?;
limit.set(0.4)?;
ui.flush(Duration::ZERO)?;
ui.unmount(&control)?;
ui.flush(Duration::ZERO)?;
assert_eq!(value.get()?, 0.25);
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
[`Component::focus_within_changed`] reports that focus left. The policy is the
owner's own: the built-in inline controls follow the library's draft policy,
and a custom owner chooses what to save, refuse or discard. To follow its own
editing permission without ending the session, the owner calls
[`DraftEditorController::set_read_only`], which keeps the draft, selection and
undo history. A [`TextField`] instead writes a binding and suits a form.

A composite control decides how Tab moves through it. Returning false from
[`Component::descendant_tab_stops`] keeps its content out of traversal, so a
grid of cells with checkboxes is one Tab stop while pointer presses still
reach the checkboxes; give keyboard users another route to them. To move
focus on Tab yourself, for example to the next cell of an editor, answer
[`Component::tab_navigation`]: the runtime offers it the Tab or Shift+Tab
before it would move focus, and a claim stops the default traversal.

A host that opens one editor at a time, as a table editing a cell does, owns
an [`EditorSession`]. It calls [`EditorSession::begin`] with an
[`EditorStart`] (the value selected, the caret at the end, or the text the
user typed), builds the editor with its `session` builder, such as
[`TextField::session`], and focuses it. The editor keeps Enter, Tab and
Escape, saves or refuses as it always does, and reports an
[`EditorCompletion`]; the host reads it with
[`EditorSession::take_completion`] in [`Component::deliver_notifications`]
and closes the editor, moves to the next cell on Tab, or leaves it open after
[`EditorOutcome::Refused`]. A custom editor takes part through
[`EditorSession::attach`] and the returned [`EditorAttachment`].

# Inspection

[`Component::inspect`] describes the widget to tooling: a [`Semantics`] role,
name, value and state, and for a widget whose children are its own content,
semantics for each of them. A control without a visible label gets a name from
the application with [`Element::with_label`] or a [`Field`]. Static text is a
name; the value is an editable or stateful control's content. A value made of
several numbers, such as a color, is [`SemanticValue::Text`] presenting the
whole value plus one [`InspectValue::Number`] property per component, as
[`ColorPicker`] reports its `hue`, `saturation`, `value` and `alpha`.

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
application applies it to the document as one undoable step. A close request
releases focus before [`App::on_close`] runs, which saves the draft being typed
by the field's own focus-loss rules, so close admission sees it as an unsaved
change; a test checks the policy with [`HeadlessApp::admit_close`].

```
use pert_ui::{KeyAction, Viewport, prelude::*};
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
closing dialog starts is queued with [`EventContext::defer`], which runs it
after the dialog has closed. The initializer returns the application as
its [state][app-state]: close admission
(`App::on_close`), error handling (`App::on_error`) and window failures
(`App::on_window_error`) receive it beside the [`Runtime`], and it lives
until the application ends. A failing initializer never reaches them: its
error returns from `run`. The example's error handler shows a component
failure on a status line the state holds and keeps the window running:

```no_run
# #[cfg(feature = "native")]
fn main() -> Result<(), pert_ui::AppError> {
    use pert_ui::{App, ErrorAction};
    // The state is what the error handler needs from the application: here
    // its status line.
    App::new("Editor", |ui| Ok(ui.signal(String::new())))
        .on_error(|_, status, error| {
            match status.set(format!("Something went wrong: {error}")) {
                Ok(()) => ErrorAction::Retry,
                Err(_) => ErrorAction::Exit,
            }
        })
        .run()?;
    Ok(())
}
# #[cfg(not(feature = "native"))]
# fn main() {}
```

# Exporting a picture of a widget

A widget is exported as an image by rendering it on a runtime of its own:
mount it alone, size the viewport to it, flush, and answer a capture
request with the CPU renderer (the `cpu` feature), which needs no window or
GPU. [`CaptureTarget::window`] names the runtime's window and
[`FrameImage::encode_png`] gives the file's bytes. The example's
`export_png` does this with a light theme and a scale of two, over a copy of
the document, so the editor's own window is not disturbed; the
[diagnostics chapter](crate::guide::diagnostics) covers captures of a running
application.
