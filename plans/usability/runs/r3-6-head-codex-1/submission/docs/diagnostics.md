[pert_ui](../../index.html)::[guide](../index.html)

# Module diagnostics Copy item path

[Source](../../../src/pert_ui/guide.rs.html#166)

Expand description

Diagnostics: keyboard input, inspection, screenshots, serialization and
profiling.

Input arrives as physical key transitions, committed text and pointer
events. Diagnostics are owned snapshots the application asks for:
component trees, semantics, validation reports, profiles and captured
frames. Nothing here runs a server or schedules polling; an application
builds its own tooling transport on these contracts.

## [§](#keyboard-input)Keyboard input

Keys have two entry points. The native host delivers each key through
[`Runtime::command_key_input`](../../struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input"),
which routes it in stages: a key already owned by a binding or
registration keeps going to its owner, shown help takes Escape, then
text editing, then explicit capture such as
[`Runtime::register_held_input`](../../struct.Runtime.html#method.register_held_input "method pert_ui::Runtime::register_held_input")
registrations, then Escape’s surface dismissal and the Tab offer to
[`Component::tab_navigation`](../../trait.Component.html#method.tab_navigation "method pert_ui::Component::tab_navigation"),
then ordinary bubbling from the component that ended text editing, and
last the view, window and application command bindings. A stage that
consumes the key ends routing, so a focused editor that uses a chord
keeps it from an application shortcut; the method’s documentation gives
the complete order.
[`Runtime::key_input`](../../struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input")
delivers to components only and routes neither commands nor held input,
and runs the text-editing stage only for an Escape while a popup, menu
or dialog is open, so it suits driving one component directly. On both
paths an unmodified Tab press or repeat is offered to
[`Component::tab_navigation`](../../trait.Component.html#method.tab_navigation "method pert_ui::Component::tab_navigation")
and otherwise moves focus, before the focused component’s ordinary key
handler sees it; its release is delivered as any key’s. A test of what a
user sees from a key, such as a shortcut, sends it through
`command_key_input` with
[`Driver::chord`](../../testing/struct.Driver.html#method.chord "method pert_ui::testing::Driver::chord")
or
[`Driver::key`](../../testing/struct.Driver.html#method.key "method pert_ui::testing::Driver::key").
A chord presses each modifier’s physical key and reports the held
modifiers through
[`Runtime::native_modifiers`](../../struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers"),
as the native host does, then presses and releases the principal key
carrying the chord’s identity, and releases the modifiers again:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::Driver;

let mut ui = Runtime::new();
let saved = ui.signal(false);
let mark = saved.clone();
// Ctrl+S, or Cmd+S on macOS.
ui.register_command("Save", CommandScope::Application, vec![KeyChord::primary('s')],
    move |_, _| {
        mark.set(true)?;
        Ok(())
    })?;
Driver::new(&mut ui).chord(KeyChord::primary('s'), KeyCode::KeyS)?;
assert!(saved.get()?);
```

The driver’s keys are synthetic: each names one key, carries no text and
no composition, and its modifiers are pressed in a fixed order. A test
of a keyboard layout, of text a key carries, of AltGr, a dead key or an
input method builds a
[`CommandKeyInput`](../../struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput")
itself and reports modifiers with `native_modifiers`, the low-level
entry points the driver uses; the [testing
chapter](../testing/index.html "mod pert_ui::guide::testing") shows the
rest of the driver.

Both entry points track every held physical key and classify presses,
repeats and releases. A repeat for a key that is not held never
establishes a press. Focused components receive the full pressed set and
can implement general simultaneous-key actions. A key bubbles through
the focused component’s ancestors until one returns `Handled` from
`keyboard_event`, starting at the focused component, or on
`command_key_input` at the first component that is not an editor; the
default returns `Ignored` after calling `key_input`, so a component that
only observes keys never swallows Escape or shortcuts. `input_reset`
releases component-owned actions on context loss, called by the default
`cancel_input`; a host’s
[`HostIntegration::clear_focus`](../../host/trait.HostIntegration.html#tymethod.clear_focus "method pert_ui::host::HostIntegration::clear_focus")
also clears window keys. Committed UTF-8 text uses `text_input`,
independently of physical key positions. The default native adapter maps
physical keys and delivers native committed text and IME preedit through
the public text input contract.

## [§](#inspection)Inspection

`Runtime::inspect()` returns a versioned owned
[`InspectionSnapshot`](../../struct.InspectionSnapshot.html "struct pert_ui::InspectionSnapshot"),
safe to send to the application’s own inspector or tooling thread.
[`Component::inspect`](../../trait.Component.html#method.inspect "method pert_ui::Component::inspect")
and `inspect_with` explicitly contribute component and host state.
[`Runtime::inspection_id`](../../struct.Runtime.html#method.inspection_id "method pert_ui::Runtime::inspection_id")
gives a mounted component’s snapshot id, so a test finds the entry of a
component it holds without matching keys, which repeat across a real
application;
[`Runtime::query`](../../struct.Runtime.html#method.query "method pert_ui::Runtime::query")
with
[`Selector::id`](../../enum.Selector.html#method.id "associated function pert_ui::Selector::id")
of that id answers with that entry alone, one lookup instead of a whole
snapshot. Unmeasured box-model values stay absent, including
[`BoxModelInspection::border`](../../struct.BoxModelInspection.html#structfield.border "field pert_ui::BoxModelInspection::border"),
which reports a painted outline’s width and color only after the paint
that drew it.
[`DebugMode::Inspect`](../../enum.DebugMode.html#variant.Inspect "variant pert_ui::DebugMode::Inspect")
enables a 256-event history.
[`DebugOverlay`](../../enum.DebugOverlay.html "enum pert_ui::DebugOverlay")
separately draws validation markers or component boxes into the scene in
builds with debug assertions. Development validation checks geometry
only for windows laid out with a viewport that has area, so content
flushed before the first viewport, as an
[`App`](../../struct.App.html "struct pert_ui::App") initializer runs,
or while a window is minimized to an empty extent reports no geometry
issue;
[`Runtime::validation_report`](../../struct.Runtime.html#method.validation_report "method pert_ui::Runtime::validation_report")
has the rule. The library starts no server or network listener and ships
no MCP support; see the [inspection design
note](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md)
for building one in the application. History is off by default and
causes no periodic scheduling. Success/failure and cancellation are
recorded; generic `event` only records dispatch, while `try_event`
records the application’s returned result. Text input contents are not
logged.

A tool on another thread asks through an
[`InspectionService`](../../struct.InspectionService.html "struct pert_ui::InspectionService"):
its
[`Send`](https://doc.rust-lang.org/1.99.1/core/marker/trait.Send.html "trait core::marker::Send")
client queues a snapshot, a `query` for the components a
[`Selector`](../../enum.Selector.html "enum pert_ui::Selector") finds by
key path, incarnation, key or semantics, a profile, a validation report
or a workspace capture, the request wakes the host, and the reply is
answered after the next flush from the state that flush presents. A
failing flush still answers, with its error beside the value. Diagnostic
commands (debug mode, overlay, profiling, clearing) are refused unless
the application enables them, and nothing the application owns can be
changed through it.

``` rust
use pert_ui::{
    Capacity, Column, Detail, Element, InspectionQuery, Label, Runtime, Selector, WaitError,
};
use std::time::Duration;
let mut ui = Runtime::new();
ui.mount_tree(Element::new(
    "main",
    Column::new([Element::new("title", Label::new("Mixer"))]),
))?;
let service = ui.inspection_service(Capacity::new(8)?)?;
let client = service.client();
let pending = std::thread::spawn(move || {
    client.query(InspectionQuery::components(Selector::children(["main"])).detail(Detail::Semantics))
})
.join()
.expect("the tool thread finishes")?;
// The request woke the host; a headless host sees it in `next_deadline` and flushes.
assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
// Unanswered until the flush: `wait` here would block forever.
assert!(matches!(pending.try_reply(), Err(WaitError::Timeout)));
ui.flush(Duration::ZERO)?;
// This flush answered the request; on the UI thread take the reply, never wait.
let reply = pending.try_reply()?;
assert_eq!(reply.value.components[0].key, "title");
assert_eq!(reply.value.components[0].path.as_deref(), Some(["main".to_owned(), "title".to_owned()].as_slice()));
```

## [§](#screenshots)Screenshots

A tool can also ask for a screenshot:
`client.capture(CaptureRequest::new( CaptureTarget::Path(..)))` is
answered with a
[`Capture`](../../struct.Capture.html "struct pert_ui::Capture") holding
the target’s visible region as an RGBA
[`FrameImage`](../../struct.FrameImage.html "struct pert_ui::FrameImage")
at physical scale, which
[`FrameImage::encode_png`](../../struct.FrameImage.html#method.encode_png "method pert_ui::FrameImage::encode_png")
encodes without any dependency. A target is a whole window
([`CaptureTarget::Window`](../../enum.CaptureTarget.html#variant.Window "variant pert_ui::CaptureTarget::Window")),
a rectangle of one
([`CaptureTarget::Region`](../../enum.CaptureTarget.html#variant.Region "variant pert_ui::CaptureTarget::Region")),
a component
([`CaptureTarget::Component`](../../enum.CaptureTarget.html#variant.Component "variant pert_ui::CaptureTarget::Component"))
or a key path
([`CaptureTarget::Path`](../../enum.CaptureTarget.html#variant.Path "variant pert_ui::CaptureTarget::Path")).
On the UI thread,
[`CaptureTarget::window`](../../enum.CaptureTarget.html#method.window "associated function pert_ui::CaptureTarget::window")
builds a window target from a borrowed
[`NativeWindowId`](../../struct.NativeWindowId.html "struct pert_ui::NativeWindowId"),
so the main window is `CaptureTarget::window(&ui.primary_window())`
([`Runtime::primary_window`](../../struct.Runtime.html#method.primary_window "method pert_ui::Runtime::primary_window")),
and
[`Runtime::request_capture`](../../struct.Runtime.html#method.request_capture "method pert_ui::Runtime::request_capture")
asks for the same capture. The frame is rendered between flushes, so
[`Pending::wait`](../../struct.Pending.html#method.wait "method pert_ui::Pending::wait")
on the UI thread blocks forever when the host has not served the request
yet; take the reply there with
[`Pending::try_reply`](../../struct.Pending.html#method.try_reply "method pert_ui::Pending::try_reply")
once it has. The [`App`](../../struct.App.html "struct pert_ui::App")
renders the frame offscreen with the same renderer that presents; a
headless runtime answers
[`CaptureError::Unsupported`](../../enum.CaptureError.html#variant.Unsupported "variant pert_ui::CaptureError::Unsupported")
unless a host declares that it renders captures with
[`Runtime::set_capture_support`](../../struct.Runtime.html#method.set_capture_support "method pert_ui::Runtime::set_capture_support")
and completes the jobs
[`HostIntegration::pending_captures`](../../host/trait.HostIntegration.html#tymethod.pending_captures "method pert_ui::host::HostIntegration::pending_captures")
lists.

With the `cpu` feature,
[`CpuRenderer`](../../cpu/struct.CpuRenderer.html "struct pert_ui::cpu::CpuRenderer")
is such a host for a headless runtime:
[`CpuRenderer::serve_captures`](../../cpu/struct.CpuRenderer.html#method.serve_captures "method pert_ui::cpu::CpuRenderer::serve_captures")
renders each pending job on the CPU, with no window or GPU, and
completes it. Call it after each flush, where a native host would
present.

``` rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::cpu::CpuRenderer;
    use pert_ui::{CaptureRequest, CaptureTarget, Element, Label, Runtime, Viewport, WaitError};
    use std::time::Duration;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    ui.mount_tree(Element::new("title", Label::new("Hello")))?;
    ui.flush(Duration::ZERO)?;
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "title".into(),
    ])));
    ui.flush(Duration::ZERO)?;
    // The flush only queued the job: unanswered until the renderer serves it.
    assert!(matches!(pending.try_reply(), Err(WaitError::Timeout)));
    CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.try_reply()?.value?; // served above
    std::fs::write(std::env::temp_dir().join("title.png"), capture.image.encode_png())?;
    // The whole main window.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
        &ui.primary_window(),
    )));
    ui.flush(Duration::ZERO)?;
    assert!(matches!(pending.try_reply(), Err(WaitError::Timeout)));
    CpuRenderer::new().serve_captures(&mut ui);
    let window = pending.try_reply()?.value?;
    assert_eq!((window.image.width(), window.image.height()), (300, 200));
    Ok(())
}
```

## [§](#serialization)Serialization

The `serde` feature, off by default, adds `Serialize` and `Deserialize`
to the snapshot, the profile, the validation report and the workspace
document, and to everything they carry. The application chooses the
format and the transport; deserialization validates through the same
constructors as the ordinary API, and a non-finite diagnostic number is
written as `"NaN"`, `"inf"` or `"-inf"` rather than lost. Enable it with
`pert-ui = { version = "0.3", features = ["serde"] }`; the [feature
table](../../index.html#cargo-features "mod pert_ui") lists every
feature.

A snapshot or a
[`QueryResult`](../../struct.QueryResult.html "struct pert_ui::QueryResult")
of another schema revision than
[`InspectionSnapshot::SCHEMA_VERSION`](../../struct.InspectionSnapshot.html#associatedconstant.SCHEMA_VERSION "associated constant pert_ui::InspectionSnapshot::SCHEMA_VERSION")
does not decode: deserializing it fails with “inspection schema version
N is not supported”, so a tool built against another release captures
again rather than reading reinterpreted fields. The version is the first
field a serializer that keeps field order, such as `serde_json`, writes,
and decoding such a document checks it first; a document whose fields
were reordered can fail on a field of the other revision first. A tool
that reads several revisions compares `schema_version` before decoding.

``` rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::{Element, InspectionSnapshot, Label, Runtime, Viewport};
    use std::time::Duration;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    ui.mount_tree(Element::new("title", Label::new("Hello")))?;
    ui.flush(Duration::ZERO)?;
    // On the UI thread: capture, then encode wherever the tool needs it.
    let text = serde_json::to_string(&ui.inspect())?;
    let snapshot: InspectionSnapshot = serde_json::from_str(&text)?;
    assert_eq!(snapshot.schema_version, InspectionSnapshot::SCHEMA_VERSION);
    assert!(snapshot.components.iter().any(|c| c.key == "title"));
    // A document of another revision is rejected, not reinterpreted.
    let current = format!("{{\"schema_version\":{}", InspectionSnapshot::SCHEMA_VERSION);
    let older = text.replacen(&current, "{\"schema_version\":1", 1);
    let error = serde_json::from_str::<InspectionSnapshot>(&older).unwrap_err();
    assert!(error.to_string().contains("inspection schema version 1 is not supported"));
    Ok(())
}
```

## [§](#measuring-ui-work)Measuring UI work

Profiling is off by default and adds no scheduled work. Enable it while
exercising an interaction, then pause and capture completed scopes.
Total includes nested work; self time excludes measured descendants.
These are elapsed UI-thread times, including measurement overhead and
waiting, not CPU percentages or GPU timings.

[`Runtime::paint`](../../struct.Runtime.html#method.paint "method pert_ui::Runtime::paint")
fails with a
[`DrawError`](../../enum.DrawError.html "enum pert_ui::DrawError"),
which does not convert into
[`RuntimeError`](../../enum.RuntimeError.html "enum pert_ui::RuntimeError").
A helper that flushes and paints, as a test of a scene does, returns
`Result<_, DrawError>`, into which `RuntimeError` and every leaf error
convert with `?`; with the `native` feature,
[`AppError`](../../enum.AppError.html "enum pert_ui::AppError") accepts
both.

``` rust
use pert_ui::{Runtime, Element, Label, Viewport, ProfileOperation};
use std::time::Duration;
let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
ui.set_profiling(true)?;
ui.mount_tree(Element::new("content", Label::new("Profile this view")))?;
ui.flush(Duration::ZERO)?;
ui.paint()?;
ui.set_profiling(false)?;
let report = ui.profile();
assert!(report.stats.iter().any(|row| row.operation == ProfileOperation::Paint));
for row in &report.stats {
    println!("{:?} {}: {} calls, {:?} total, {:?} self, {:?} max",
        row.operation, row.key, row.calls, row.total, row.self_time, row.maximum);
}
ui.clear_profile()?;
```
