
From the checkout, run it and the gallery with:

```sh
cargo run --locked -p pert-ui --example getting_started
cargo run --locked -p pert-ui-gallery
```

[`App::run`] runs on the main thread, once per process. Its initializer builds
content once. It returns initialization, graphics and unrecovered application
errors to the caller. [`App::size`] can change the initial window size; the default
window is 960 × 640 logical pixels.

## Compose content and preserve identity

An [`Element`] pairs a stable sibling key with a component. Keys identify content,
so use `"name-field"` rather than its position in a list or its translated label.
[`Runtime::reconcile_children`] matches keys and component types; survivors retain
identity and local state. Removing an element releases its subscriptions and
cancels its active input. Recreating it produces a new incarnation.

Use [`Column`], [`Row`] and [`Wrap`] for ordinary flow, [`Form`] for adaptive field
columns, and [`Field`] to associate a label, help and application-owned status
with any control. [`FormFields`] coordinates errors and can focus the first one;
it never decides whether your application data is valid. [`SplitPane`] and [`Tabs`]
compose larger views. Inactive tabs retain local state and suspend presentation.

`Form` uses its allocated width, so it also works inside a resizable side panel.
Its labels stay above their controls. A row's column count changes without
remounting editors or submitting their drafts. Use [`ScrollArea`] around standalone
forms with `.axes(ScrollAxes::Both)`; tab and region content already has
two-axis scrolling by default. A plain [`ScrollArea::new`] scrolls vertically.

## Choose the right data contract

| Need | Public contract | Ownership |
|---|---|---|
| Local search text, toggles or form values | `Signal<T>` and `ReadSignal<T>` | UI thread; writes invalidate subscribed components. |
| An editable parameter with preview and commit/cancel | `Parameter<T>` and `ParameterHost<T>` | Host retains the authority; controls share the binding. |
| Worker results or application messages | [`EventContext::spawn`] from a callback, [`Runtime::mailbox`], `Mailbox<T>`, `UiSender<T>` | Background work returns an owned result, or a worker sends owned values; either is delivered on the UI thread. |
| Lossy, high-rate presentation | [`SeriesSource`] or [`Telemetry`] | Producer and UI have separate endpoints and explicit gap/freshness semantics. |

Do not send UI handles to a worker or mutate a shared value invisibly. Mailboxes
can be full or closed; `send` returns the undelivered value in its error. They
take locks and can wake the event loop, so they are not time-critical callback endpoints.
The live-data endpoints have their own documented bounded-work contracts.

Text fields write strings immediately. [`IntegerInput`] and [`DecimalInput`] instead
keep a local draft and save on Enter or focus loss; hiding/removal cancels the
draft. String bindings must pass the editors' character rules and
`pert_ui::TextEditError::MAX_TEXT_BYTES` UTF-8 byte limit. Use `str::len()` to check that limit;
passing it does not guarantee the shaped text fits the raster limits.

Here two sliders share one local parameter. The host owner stays alive for the
whole example; dropping it removes the parameter.

```
use pert_ui::{parameter::*, prelude::*};
use std::time::Duration;
let mut ui = Runtime::new();
let (amount, host) = ui.parameter(
    ParameterId::new(1, 1, "amount")?,
    ParameterSpec::new("Amount", 0.0, 100.0, 25.0)?,
)?;
ui.mount_tree(Element::new("controls", Column::new([
    Element::new("first", Slider::new(amount.clone())),
    Element::new("second", Slider::new(amount.clone())),
])))?;
ui.flush(Duration::ZERO)?;
host.write_base(50.0)?;
ui.flush(Duration::ZERO)?;
assert_eq!(amount.snapshot().get()?.base, 50.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

In a native application, retain parameter hosts in application-owned state or a
component for as long as those parameters exist. A local `Committed` outcome
means local acceptance. For external engine acceptance use `remote_parameter`
and [`RemoteHost`], and distinguish submitted, accepted, rejected and unknown
outcomes. The application owns transport, domain units and undo history.

## Save the workspace separately from your project

The library captures its arrangement under application keys. The document has
its own schema version; it contains settings and supported container state,
not application data, focus, active gestures or detached native windows.

```
use pert_ui::{prelude::*, windowing::*};
use pert_ui::Workspace;
let mut ui = Runtime::new();
ui.mount_tree(Element::new("workspace", SplitPane::new(
    SplitAxis::LeftRight,
    Element::new("navigation", Label::new("Navigation")),
    Element::new("editor", Label::new("Editor")),
)))?;
let text = ui.capture_workspace()?.encode();
// The host chooses when and where to store this owned string.
let saved = Workspace::decode(&text)?;
let report = ui.restore_workspace(&saved)?;
assert!(report.is_complete());
# Ok::<(), Box<dyn std::error::Error>>(())
```

In a real application inspect `applied`, `pending`, `ignored` and `rejected` on
the returned report. Missing views may mount later; incompatible components
and busy gestures can refuse entries. Malformed/unsupported documents return
typed errors. Keep your default workspace available and report read/decode
failures; do not silently discard them. For capture/restore initiated by a
button, use [`EventContext::defer`] to access the runtime outside event dispatch
and retain its task receipt if the result matters to the application.

## Write a custom widget

Custom components use the same public measurement, binding and painting hooks
as built-in controls. This indicator records only a colored rectangle. Its
binding affects paint, so the update uses `read_paint` and retains that value.

```
use pert_ui::{component::*, prelude::*};
use std::time::Duration;

struct Indicator { source: ReadSignal<bool>, active: bool }
impl Component for Indicator {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.active = cx.read_paint(&self.source)?;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let side = cx.metric(MetricRole::ControlHeight);
        Ok(Size::new(side, side)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let role = if self.active { ColorRole::Primary } else { ColorRole::Muted };
        let color = painter.color(role);
        painter.fill_rect(Rect::from_size(painter.size()), color);
        Ok(())
    }
}
let mut ui = Runtime::new();
let active = ui.signal(false);
ui.mount_tree(Element::new("indicator", Indicator {
    source: active.read_only(), active: false,
}))?;
ui.flush(Duration::ZERO)?;
active.set(true)?;
assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
ui.paint()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Do not write bindings from update, measurement, paint or inspection. Handle
input through event callbacks, and keep drawing independent of GPU/backend
types. If a custom widget is interactive, also implement focus, cancellation
and typed inspection semantics; the [custom widgets](crate::guide::custom_widgets)
chapter covers these for a widget that is dragged, clicked and typed into. Only widgets that actually animate need
[`UpdateContext::animate`]; ordinary bindings already schedule their own work.

For executable compositions see the gallery. The crate reference documents
the supported API, including lifecycle and errors; repository design notes
also contain future proposals and should not be treated as available methods.
