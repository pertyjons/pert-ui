Where state lives decides which contract to use. UI-local values are signals
that components read and the runtime tracks. Values the application owns
authoritatively are parameters, edited through gestures that the host observes.
Results from other threads arrive through mailboxes, and lossy high-rate data
through live-data sources.

# Local state and retained components

```rust
use std::time::Duration;
use pert_ui::prelude::*;

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

A signal's value type needs [`PartialEq`] to be written, since a write that
leaves the value equal notifies nobody, and [`Clone`] to be read, since
[`ReadSignal::get`] returns a copy. Creating a signal needs neither.

This example drives runtime directly. The default native [`App`] supplies elapsed
time and waits for work/deadlines automatically. Custom hosts drive the same API. Components retain their own presentation
data. Every update recollects all dependencies; ordinary reads invalidate layout
and paint, while `read_paint` is for data confined to fixed geometry. No writes
are allowed during update, paint or inspection. Changed signals only enqueue dependents,
and hidden components retain dirtiness without evaluation. `animate()` uses the
per-window presentation ceiling (60 Hz when unknown); no catch-up frames are generated.
A timed action that must reach application state uses the separate event-phase
timer instead: [`EventContext::set_timer`] arms one deadline for the component and
[`Component::timer_elapsed`] runs in the event phase of the flush that reaches it,
so a held repeat or auto-scroll may write bindings and commit edits like ordinary
input. The timer is one-shot, replaces a pending one, and is cancelled with the
component's input context on focus loss, hiding, unmount or `cancel_input`.
Components record solid rectangles, shapes, vector paths, text and images through
[`Painter`]; runtime retains their commands and copies the visible scene on `paint`.
Unchanged paint callbacks are not repeated. GPU resources belong to the selected graphics adapter; text preparation
and coverage are cached separately from GPU resources.

# Parameters

```rust
use pert_ui::prelude::*;
use pert_ui::ParameterError;

let amount = ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?;
let id = ParameterId::new(1, 42, "amount")?;
let position = NormalizedValue::try_from(0.75)?;
assert_eq!(amount.denormalize(position), 50.0);
assert_eq!(amount.normalize(&f32::NAN), Err(ParameterError::NonFinite));
assert_ne!(id, id.next_incarnation()?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Positions are dimensionless; canonical numeric values have application-defined
meaning. Linear mapping and editable metadata are defaults. An optional
`with_mapping(ParameterMapping::Logarithmic)` selects a strictly positive
logarithmic domain, and `with_mapping(ParameterMapping::Power(exponent))` with
a [`PowerExponent`] gives part of any range more travel, also one that
includes zero or negative values: an exponent above one spreads the low end. Metadata does not hold a current value or enforce host edits.
The library ships no unit presets: units and their meaning stay with the
application. Four closed parameter families share one protocol: continuous
`f32` ([`ParameterSpec::new`]), exact stepped `i64` ([`ParameterSpec::stepped`]),
`bool` ([`ParameterSpec::boolean`]) and choices identified by stable [`ChoiceId`]s
([`ParameterSpec::choices`]); nothing is erased into floats.

Parameter IDs include session, object, stable key and runtime incarnation. The
host assigns identifiers, retains the next incarnation after recreation and must
match the complete ID before applying edits. IDs do not provide a registry or
message routing on their own. [`Runtime::parameter`] registers each complete identity
once and gives controls a shared binding plus a separate authoritative host owner.

# Local parameter gestures

```rust
use std::time::Duration;
use pert_ui::{prelude::*, EditOutcome};

struct Control;
impl Component for Control {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
}
let mut runtime = Runtime::new();
let (amount, host) = runtime.parameter(
    ParameterId::new(1, 1, "amount")?, ParameterSpec::new("Amount", 0.0, 10.0, 2.0)?,
)?;
let control = runtime.mount_tree(Element::new("control", Control))?;
runtime.flush(Duration::ZERO)?;
let revision = amount.snapshot().get()?.revision;
let edit = runtime.try_event(&control, |cx| amount.begin_edit(cx, revision))??;
edit.update(4.0)?;
assert_eq!(amount.snapshot().get()?.base, 2.0);
assert_eq!(amount.snapshot().get()?.preview, Some(4.0));
let committed = edit.commit(5.0)?;
assert!(matches!(committed, EditOutcome::Committed { value: 5.0, .. }));
drop(edit); // acknowledges the reserved terminal receipt
host.write_base(6.0)?;
assert_eq!(amount.snapshot().get()?.base, 6.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The host owner must stay alive: dropping it removes the parameter. Gesture drop,
Escape, hiding, focus loss and unmount cancel unfinished work. External base or
metadata changes reject conflicting edits. Effective scalar overlays remain
separate and may exceed the editable range. The local protocol retains up to
64 terminal receipts per parameter until acknowledged; guard drop acknowledges
automatically. Commit replay cannot write twice. Undo policy stays with the host.
Lifecycle cleanup may cancel during evaluation; ordinary writes cannot.

# Parameter families and host notifications

Boolean, choice and stepped parameters bind to the standard and inline controls
through `for_parameter`; choice controls take their options from the
specification. A host that owns undo or forwards edits registers a terminal sink
and keeps its delivery monitor. Each changed activation is one begin/commit pair,
delivered once after the new state is published.

```rust
use std::{cell::RefCell, rc::Rc};
use pert_ui::prelude::*;
use pert_ui::{EditOutcome, KeyAction, KeyCode};

let mut ui = Runtime::new();
let list = ChoiceId::new("list")?;
let (view_mode, host) = ui.parameter(
    ParameterId::new(1, 7, "view-mode")?,
    ParameterSpec::choices(
        "View mode",
        [
            ChoiceOption::new(list.clone(), "List"),
            ChoiceOption::new(ChoiceId::new("grid")?, "Grid"),
        ],
        list,
    )?,
)?;
let committed = Rc::new(RefCell::new(Vec::new()));
let log = committed.clone();
let monitor = host.on_outcome(move |event| {
    if let EditOutcome::Committed { value, .. } = &event.outcome {
        log.borrow_mut().push(value.clone()); // application undo, forwarding, ...
    }
    Ok(())
})?;
let group = ui.mount_tree(Element::new(
    "view-mode",
    SegmentedControl::for_parameter(view_mode.clone()),
))?;
ui.focus(&group)?;
ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
assert_eq!(view_mode.snapshot().get()?.base.as_str(), "grid");
assert_eq!(committed.borrow().len(), 1);
assert!(!monitor.snapshot().is_faulted());
# Ok::<(), Box<dyn std::error::Error>>(())
```

A sink error is retained in the monitor, the parameter refuses new edits and
its controls show as disabled until the host retries or resolves the delivery.
`Committed` is the local store's acceptance, not an external authority acknowledgement.
Sinks run on the UI thread and cannot write to the runtime while they run.

# Updating bindings after a sink

A readout that depends on what a sink records, such as the depth of the
application's undo history, is refreshed through a [`Mailbox`] of capacity one
whose handler rereads the application's state. The sink records the outcome
first, then sends a token as its last step and returns `Ok` whether or not the
token fitted: a full mailbox already holds a refresh, which will read the new
outcome, so pending refreshes collapse. The handler runs at the start of the
next flush, where writes are allowed, and an error it returns comes from that
flush, separately from the parameter's delivery report. Undo and reset call
[`ParameterHost::write_base`], which ends an active edit before it returns and
reaches no sink otherwise; the application forwards the written value to its
engine itself, where it makes the call.

```rust
use std::{cell::RefCell, rc::Rc};
use pert_ui::prelude::*;
use pert_ui::{Capacity, EditOutcome, KeyAction, KeyCode};
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let (gain, host) = ui.parameter(
    ParameterId::new(1, 1, "gain")?,
    ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
)?;
let history: Rc<RefCell<Vec<f32>>> = Rc::default();
let depth = ui.signal(0usize);
let (shown, read) = (depth.clone(), history.clone());
let refresh = ui.mailbox(Capacity::new(1)?, move |()| shown.set(read.borrow().len()))?;
let (sender, record) = (refresh.sender(), history.clone());
let _monitor = host.on_outcome(move |event| {
    if let EditOutcome::Committed { .. } = &event.outcome {
        record.borrow_mut().push(event.begin_value); // the undo entry
    }
    let _ = sender.send(()); // full: a refresh is already pending
    Ok(())
})?;
let fader = ui.mount_tree(Element::new("gain", Slider::new(gain)))?;
ui.flush(Duration::ZERO)?;
ui.focus(&fader)?;
for _ in 0..2 {
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
}
ui.flush(Duration::ZERO)?;
assert_eq!(depth.get()?, 2);
// Undo: the application writes the base, forwards the value to its engine
// itself and, since no sink runs, updates the readout it owns directly.
let previous = history.borrow_mut().pop().ok_or("an entry")?;
host.write_base(previous)?;
depth.set(history.borrow().len())?;
assert_eq!(depth.get()?, 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```
Descriptor-driven panels use the family-tagged [`AnySpec`], [`AnyValue`],
[`AnyParameter`] and [`AnyParameterHost`]; [`HostValue`] translates an application's
own value types without narrowing them.

# Effective overlays and remote outcomes

Built-in parameter controls present editable preview/base. The read-only effective
overlay has its own validation and can lie outside a numeric editable domain; a
custom display uses [`ParameterSnapshot::effective`] and indicates clipping without
normalization or writeback. [`ParameterHost::set_effective`] includes a complete
mapped [`InlineReadout`] example. The snapshot effective field already falls back
to preview/base when no overlay is supplied.

[`RemoteHost`] includes a runnable headless example for failed submission handoff,
explicit retry, corrected acceptance, duplicate responses and disconnect/recovery.
A successful submission or transport callback is not external acceptance. Retain
an [`AuthorityReceipt`] per full token and connection generation, instead of reading
only the host's latest receipt. A local [`TerminalEvent`] can reject a superseded
edit while that submission's authority receipt remains [`AuthorityStatus::Unknown`].
Reconcile the external store before resynchronization; stale responses cannot
reopen the old local edit. External deduplication, unknown outcomes and undo belong
to the application. The packaged dynamic application example below demonstrates a
worker and UI mailbox; the library starts neither for the host.

Capture the host's latest receipt after each submission/input dispatch and before
another submission or supersession replaces it. A connected host write or
connected resynchronization can locally produce `Resolved` with
`Rejected(Conflict)`; that is local supersession, not an authority refusal or proof
that the external mutation never ran. Disconnect before recovery when its external
outcome is unknown, retain its receipt and reconcile the actual external store.

A first authority response after a same-generation host write can return
`AlreadyResolved`, even when the authority accepted the mutation. The UI adapter
does not apply that response; reconcile its content with the actual external store.
Resynchronization instead changes the generation and returns `Stale` for old
responses. Neither disposition alone proves a duplicate or external refusal.

# Worker results and live data

Occasional work that must not block the interface, such as reading or writing
a file, runs through [`Runtime::spawn`]: the work runs on a new thread and its
result reaches a delivery callback on the UI thread, where it may write
bindings. A button or component callback receives an [`EventContext`], not the
runtime, so it starts the work with [`EventContext::spawn`], which queues the
start for the beginning of the next flush.

```rust
use std::time::Duration;
use pert_ui::prelude::*;
use pert_ui::{KeyAction, KeyCode};

let mut ui = Runtime::new();
let status = ui.signal("Ready".to_owned());
let shown = status.clone();
let load = Button::new("Count words", move |cx| {
    shown.set("Counting".to_owned())?;
    // Real work would read a file; the result carries any failure.
    let text = String::from("one two three");
    let shown = shown.clone();
    let _receipt = cx.spawn(
        move || text.split_whitespace().count(),
        move |words| shown.set(format!("{words} words")),
    )?;
    Ok(())
});
let button = ui.mount_tree(load.key("load"))?;
ui.flush(Duration::ZERO)?;
ui.focus(&button)?;
ui.key_input(KeyCode::Space, KeyAction::Press)?;
ui.key_input(KeyCode::Space, KeyAction::Release)?;
// A native host flushes when the result wakes it; a headless one polls.
while status.get()? == "Counting" {
    std::thread::sleep(Duration::from_millis(1));
    ui.flush(Duration::ZERO)?;
}
assert_eq!(status.get()?, "3 words");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The work receives no UI handles. The start belongs to the button's window, so
it still runs when the callback hides or removes the button, as a Save button
that closes its dialog does, and is cancelled only when the window closes or
the runtime shuts down first; the receipt reports that start, not the result.
To be able to discard a result that has not arrived, queue [`Runtime::spawn`]
with [`EventContext::defer`] and keep the [`BackgroundTask`] it returns.
[`Runtime::spawn`] lists the limits and what happens to a result after
cancellation or shutdown.

A long-lived worker thread hands owned results to the UI thread through a
[`Runtime::mailbox`]: its [`UiSender`] is [`Send`], a full or closed mailbox returns
the undelivered value in [`SendError`], and the handler runs on the UI thread,
where it may write bindings. Mailboxes take locks and can wake the event loop, so
they are not endpoints for time-critical producer callbacks.

High-rate, lossy data such as levels and traces travels separately. A
`series_bridge` splits into a `Send` producer [`SeriesEndpoint`], which publishes
with bounded work and no allocation into a ring preallocated when the bridge is
created, and a
[`SeriesConsumer`] that [`Runtime::series_source`] turns into a UI-thread
[`SeriesSource`]. A visible reader drains it at the window's presentation rate
([`Delivery::Polled`]), or when a [`LiveWaker`] signals new data
([`Delivery::Notified`]); hidden readers schedule nothing. Components read the
latest item with its [`Freshness`] as [`Telemetry`], and older items as a window.
[`LevelMeter`] and [`Trace`] are the built-in readers; custom components use the
same `SeriesSource` API.

# Bounded updates and recovery

[`Runtime::flush`] limits update/settlement feedback to [`Runtime::MAX_FLUSH_WAVES`],
including internal focus/hover/layout follow-ups. [`RuntimeError::UpdateLimit`]
preserves pending work for repair and explicit retry; development validation names
pending components. This does not bound the duration of a user callback or tree depth.
Custom hosts can call [`Runtime::cancel_input`] after a failed operation to clear
held actions and capture while preserving focus and numeric drafts without saving.

With the default desktop feature, component input/update/drawing errors are reported
to stderr and retried after 100/200/400 ms. A fourth consecutive failure returns
from [`App::run`]; only successful presentation resets the budget. [`App::on_error`]
can repair state and choose [`ErrorAction::Retry`] or [`ErrorAction::Exit`] on the UI
thread. Failed input actions are never replayed. Native editing events during
backoff are discarded; close and lifecycle events remain available. Closing during
backoff returns its pending error. Initialization/native/GPU failures bypass this
callback and keep their separate recovery rules.

# Dynamic applications and external authority

The packaged
[dynamic documents example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/dynamic_documents.rs)
opens
controls and parameter resources from a button through [`EventContext::defer`],
uses derived caption/enabled bindings, transports an owned external proposal and
response through a worker and UI mailbox, and releases the document on close.
Run it headlessly from a checkout with
`cargo run -p pert-ui --example dynamic_documents --no-default-features`.
`Submitted` is a proposal, not authoritative acceptance; retain and inspect its
[`AuthorityReceipt`] when delivery can be delayed or disconnected.

The packaged
[live mixer example](https://github.com/pertyjons/pert-ui/blob/main/crates/pert-ui/examples/live_mixer.rs)
puts the live-data and parameter paths together over an application-owned
simulation: a producer thread behind a bounded command queue publishes levels
through a [`series_bridge`] to [`LevelMeter`]s. Parameter sinks send the base
each edit leaves without blocking, so a cancelled audition is undone in the
engine too; on a full or closed queue they fail, the parameter keeps the
outcome, and a Retry button delivers it with [`ParameterHost::retry_outcome`]
together with any undo or reset that did not reach the engine, sending current
values so nothing stale overwrites a newer write. Undo and reset also work
during a drag, which they end. The layout is saved and restored as a
workspace, with decode failures and partial restores shown to the user. Dropping the engine stops and joins
its thread. It runs a deterministic headless session with
`cargo run -p pert-ui --example live_mixer --no-default-features`, and opens a
window with `--window` when built with the `native` feature. The simulation is
an ordinary thread and makes no real-time claim.
