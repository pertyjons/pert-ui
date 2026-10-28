[pert_ui](../../index.html)::[guide](../index.html)

# Module getting_started Copy item path

[Source](../../../src/pert_ui/guide.rs.html#44)

Expand description

Getting started: a first window, composition, data contracts, a saved
workspace and a custom widget.

pert-ui is an unreleased desktop GUI library. Applications use `pert_ui`
for composition, bindings and native startup; no renderer, event loop or
repaint timer needs to be supplied. Native execution has been exercised
on Linux Wayland/Vulkan. Windows and macOS are targets with native
verification still outstanding. Platform accessibility adapters and
plugin hosting remain deferred.

### [§](#run-your-first-window)Run your first window

Add the facade to your application’s `Cargo.toml`:

``` language-toml
[dependencies]
pert-ui = { version = "0.3" }
```

Before the first registry release, or to follow unreleased changes,
depend on a checkout of the repository instead, pointing to the facade
inside **your** checkout:

``` language-toml
[dependencies]
pert-ui = { path = "../pert-ui/crates/pert-ui" }
```

pert-ui requires Rust 1.99.1 or later. Either dependency uses the
default `desktop` feature; a headless consumer adds
`default-features = false`. The package includes the sources of its
examples, but `cargo run -p pert-ui --example …`, as the guide runs
them, works only in a checkout.

This complete program opens a profile form. Editing writes the local
signals; Reset changes the same bindings, so only their dependents
refresh. Resize the window to see the form change columns. The scroll
area keeps controls reachable when the window is too small for one
usable column.

The program is the package’s `examples/getting_started.rs`:

``` rust
//! The getting-started guide's profile form: two text fields in an adaptive
//! form, and a button that resets both through their bindings.
//!
//! Needs the default `desktop` feature: `cargo run -p pert-ui --example getting_started`.
use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    App::new("Profile", |ui| {
        let name = ui.signal("Ada".to_owned());
        let city = ui.signal("Stockholm".to_owned());
        let (reset_name, reset_city) = (name.clone(), city.clone());
        let fields = Form::new([
            Field::new(TextField::new(&name).key("name"))
                .label("Name")
                .help("Shown to other people")
                .key("name-field"),
            Field::new(TextField::new(&city).key("city"))
                .label("City")
                .key("city-field"),
        ]);
        let reset = Button::new("Reset", move |_| {
            reset_name.set("Ada".to_owned())?;
            reset_city.set("Stockholm".to_owned())
        });
        let content = Column::new([
            Label::new("Your profile").key("heading"),
            fields.key("fields"),
            reset.key("reset"),
        ]);
        ui.mount_tree(
            ScrollArea::new(content.key("content"))
                .axes(ScrollAxes::Both)
                .key("profile-viewport"),
        )?;
        Ok(())
    })
    .run()
}
```

From the checkout, run it and the gallery with:

``` language-sh
cargo run --locked -p pert-ui --example getting_started
cargo run --locked -p pert-ui-gallery
```

[`App::run`](../../struct.App.html#method.run "method pert_ui::App::run")
runs on the main thread, once per process. Its initializer builds
content once. It returns initialization, graphics and unrecovered
application errors to the caller.
[`App::size`](../../struct.App.html#method.size "method pert_ui::App::size")
can change the initial window size; the default window is 960 × 640
logical pixels.

### [§](#compose-content-and-preserve-identity)Compose content and preserve identity

An [`Element`](../../struct.Element.html "struct pert_ui::Element")
pairs a stable sibling key with a component. Keys identify content, so
use `"name-field"` rather than its position in a list or its translated
label.
[`Runtime::reconcile_children`](../../struct.Runtime.html#method.reconcile_children "method pert_ui::Runtime::reconcile_children")
matches keys and component types; survivors retain identity and local
state. Removing an element releases its subscriptions and cancels its
active input. Recreating it produces a new incarnation.

Use [`Column`](../../struct.Column.html "struct pert_ui::Column"),
[`Row`](../../struct.Row.html "struct pert_ui::Row") and
[`Wrap`](../../struct.Wrap.html "struct pert_ui::Wrap") for ordinary
flow, [`Form`](../../struct.Form.html "struct pert_ui::Form") for
adaptive field columns, and
[`Field`](../../struct.Field.html "struct pert_ui::Field") to associate
a label, help and application-owned status with any control.
[`FormFields`](../../struct.FormFields.html "struct pert_ui::FormFields")
coordinates errors, can focus the first one and discard every field’s
draft; it never decides whether your application data is valid.
[`SplitPane`](../../struct.SplitPane.html "struct pert_ui::SplitPane")
and [`Tabs`](../../struct.Tabs.html "struct pert_ui::Tabs") compose
larger views. Inactive tabs retain local state and suspend presentation.

`Form` uses its allocated width, so it also works inside a resizable
side panel. Its labels stay above their controls. A row’s column count
changes without remounting editors or submitting their drafts. Use
[`ScrollArea`](../../struct.ScrollArea.html "struct pert_ui::ScrollArea")
around standalone forms with `.axes(ScrollAxes::Both)`; tab and region
content already has two-axis scrolling by default. A plain
[`ScrollArea::new`](../../struct.ScrollArea.html#method.new "associated function pert_ui::ScrollArea::new")
scrolls vertically.

### [§](#choose-the-right-data-contract)Choose the right data contract

| Need | Public contract | Ownership |
|----|----|----|
| Local search text, toggles or form values | `Signal<T>` and `ReadSignal<T>` | UI thread; writes invalidate subscribed components. |
| An editable parameter with preview and commit/cancel | `Parameter<T>` and `ParameterHost<T>` | Host retains the authority; controls share the binding. |
| Worker results or application messages | [`EventContext::spawn`](../../struct.EventContext.html#method.spawn "method pert_ui::EventContext::spawn") from a callback, [`Runtime::mailbox`](../../struct.Runtime.html#method.mailbox "method pert_ui::Runtime::mailbox"), `Mailbox<T>`, `UiSender<T>` | Background work returns an owned result, or a worker sends owned values; either is delivered on the UI thread. |
| Lossy, high-rate presentation | [`SeriesSource`](../../struct.SeriesSource.html "struct pert_ui::SeriesSource") or [`Telemetry`](../../struct.Telemetry.html "struct pert_ui::Telemetry") | Producer and UI have separate endpoints and explicit gap/freshness semantics. |

Do not send UI handles to a worker or mutate a shared value invisibly.
Mailboxes can be full or closed; `send` returns the undelivered value in
its error. They take locks and can wake the event loop, so they are not
time-critical callback endpoints. The live-data endpoints have their own
documented bounded-work contracts.

Text fields write strings immediately.
[`IntegerInput`](../../struct.IntegerInput.html "struct pert_ui::IntegerInput")
and
[`DecimalInput`](../../struct.DecimalInput.html "struct pert_ui::DecimalInput")
instead keep a local draft and save on Enter or focus loss;
hiding/removal cancels the draft. String bindings must pass the editors’
character rules and `pert_ui::TextEditError::MAX_TEXT_BYTES` UTF-8 byte
limit. Use `str::len()` to check that limit; passing it does not
guarantee the shaped text fits the raster limits.

Here two sliders share one local parameter. The host owner stays alive
for the whole example; dropping it removes the parameter.

``` rust
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
```

In a native application, retain parameter hosts in application-owned
state or a component for as long as those parameters exist. A local
`Committed` outcome means local acceptance. For external engine
acceptance use `remote_parameter` and
[`RemoteHost`](../../struct.RemoteHost.html "struct pert_ui::RemoteHost"),
and distinguish submitted, accepted, rejected and unknown outcomes. The
application owns transport, domain units and undo history.

### [§](#save-the-workspace-separately-from-your-project)Save the workspace separately from your project

The library captures its arrangement under application keys. The
document has its own schema version; it contains settings and supported
container state, not application data, focus, active gestures or
detached native windows.

``` rust
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
```

In a real application inspect `applied`, `pending`, `ignored` and
`rejected` on the returned report. Missing views may mount later;
incompatible components and busy gestures can refuse entries.
Malformed/unsupported documents return typed errors. Keep your default
workspace available and report read/decode failures; do not silently
discard them. For capture/restore initiated by a button, use
[`EventContext::defer`](../../struct.EventContext.html#method.defer "method pert_ui::EventContext::defer")
to access the runtime outside event dispatch and retain its task receipt
if the result matters to the application.

### [§](#write-a-custom-widget)Write a custom widget

Custom components use the same public measurement, binding and painting
hooks as built-in controls. This indicator records only a colored
rectangle. Its binding affects paint, so the update uses `read_paint`
and retains that value.

``` rust
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
```

Do not write bindings from update, measurement, paint or inspection.
Handle input through event callbacks, and keep drawing independent of
GPU/backend types. If a custom widget is interactive, also implement
focus, cancellation and typed inspection semantics; the [custom
widgets](../custom_widgets/index.html "mod pert_ui::guide::custom_widgets")
chapter covers these for a widget that is dragged, clicked and typed
into. Only widgets that actually animate need
[`UpdateContext::animate`](../../struct.UpdateContext.html#method.animate "method pert_ui::UpdateContext::animate");
ordinary bindings already schedule their own work.

For executable compositions see the gallery. The crate reference
documents the supported API, including lifecycle and errors; repository
design notes also contain future proposals and should not be treated as
available methods.
