A value is authoritative when the application, not the control, decides
what it is: a host validates each edit, keeps revisions and observes the
gestures that produce it, and may forward them to an engine or to an external
authority that accepts or refuses them. Such values are parameters. A value
that a control simply writes is UI-local state, a signal, which the
[state chapter](crate::guide::state) covers.

The names this chapter uses beyond the prelude are in
[`parameter`](crate::parameter): specifications, identities, hosts, gestures,
delivery and authority records, and the parameter-only controls [`Slider`],
[`Knob`] and [`InlineSlider`]. Its examples glob both, and add
[`component`](crate::component) where they implement a component and
[`testing`](crate::testing) where they drive input by hand.

# Parameters

```rust
use pert_ui::{parameter::*, prelude::*};

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
use pert_ui::{component::*, parameter::*, prelude::*};

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
use pert_ui::{parameter::*, prelude::*};
use pert_ui::KeyAction;

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
use pert_ui::{parameter::*, prelude::*};
use pert_ui::testing::*;
use pert_ui::Capacity;
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
