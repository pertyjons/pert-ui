Input arrives as physical key transitions, committed text and pointer events.
Diagnostics are owned snapshots the application asks for: component trees,
semantics, validation reports, profiles and captured frames. Nothing here runs
a server or schedules polling; an application builds its own tooling transport
on these contracts.

# Keyboard input

Keys have two entry points. The native host delivers each key through
[`Runtime::command_key_input`], which routes it in stages: a key already
owned by a binding or registration keeps going to its owner, shown help
takes Escape, then text editing, then explicit capture such as
[`Runtime::register_held_input`] registrations, then Escape's surface
dismissal and the Tab offer to [`Component::tab_navigation`], then ordinary
bubbling from the component that ended text editing, and last the view,
window and application command bindings. A stage that consumes the key ends routing, so a
focused editor that uses a chord keeps it from an application shortcut; the
method's documentation gives the complete order. [`Runtime::key_input`]
delivers to components only and routes neither commands nor held input, and
runs the text-editing stage only for an Escape while a popup, menu or dialog
is open, so it suits driving one component directly. On both paths an
unmodified Tab press or repeat is offered to [`Component::tab_navigation`] and
otherwise moves focus, before the focused component's ordinary key handler
sees it; its release is delivered as any key's. A test of what a user sees from a key,
such as a shortcut, uses `command_key_input` and reports held modifiers through
[`Runtime::native_modifiers`], as the native host does:

```rust
use pert_ui::{CommandKey, CommandKeyInput, CommandScope, KeyAction, KeyChord, KeyCode,
              Modifiers, Runtime};

let mut ui = Runtime::new();
let saved = ui.signal(false);
let mark = saved.clone();
// Ctrl+S, or Cmd+S on macOS.
ui.register_command("Save", CommandScope::Application, vec![KeyChord::primary('s')],
    move |_, _| {
        mark.set(true)?;
        Ok(())
    })?;
ui.native_modifiers(Modifiers::PRIMARY)?;
for action in [KeyAction::Press, KeyAction::Release] {
    ui.command_key_input(CommandKeyInput::new(KeyCode::KeyS, action, CommandKey::Character('s')))?;
}
ui.native_modifiers(Modifiers::default())?;
assert!(saved.get()?);
# Ok::<(), pert_ui::RuntimeError>(())
```

Both entry points track every held physical key and classify presses, repeats and
releases. A repeat for a key that is not held never establishes a press. Focused
components receive the full pressed set and can implement general
simultaneous-key actions. A key bubbles through the focused component's
ancestors until one returns `Handled` from `keyboard_event`, starting at the
focused component, or on `command_key_input` at the first component that is
not an editor; the default returns
`Ignored` after calling `key_input`, so a component that only observes keys never
swallows Escape or shortcuts. `input_reset` releases component-owned actions on
context loss, called by the default `cancel_input`; `clear_focus` also clears
window keys. Committed UTF-8 text uses `text_input`,
independently of physical key positions. The default native adapter maps physical
keys and delivers native committed text and IME preedit through the public text
input contract.

# Inspection

`Runtime::inspect()` returns a versioned owned [`InspectionSnapshot`], safe to send
to the application's own inspector or tooling thread. [`Component::inspect`] and `inspect_with` explicitly
contribute component and host state. [`Runtime::inspection_id`] gives a mounted
component's snapshot id, so a test finds the entry of a component it holds
without matching keys, which repeat across a real application. Unmeasured box-model values stay absent,
including [`BoxModelInspection::border`], which reports a painted outline's width
and color only after the paint that drew it.
[`DebugMode::Inspect`] enables a 256-event history. [`DebugOverlay`] separately draws
validation markers or component boxes into the scene in builds with debug
assertions. Development validation checks geometry only for windows laid out
with a viewport that has area, so content flushed before the first viewport,
as an [`App`] initializer runs, or while a window is minimized to an empty
extent reports no geometry issue; [`Runtime::validation_report`] has the rule. The library starts no server or network listener and ships no MCP
support; see the
[inspection design note](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md)
for building one in the application.
History is off by default and causes no periodic scheduling. Success/failure and
cancellation are recorded; generic `event` only records dispatch, while `try_event`
records the application's returned result. Text input contents are not logged.

A tool on another thread asks through an [`InspectionService`]: its [`Send`]
client queues a snapshot, a `query` for the components a [`Selector`] finds by
key path, incarnation, key or semantics, a profile, a validation report or a
workspace capture, the request wakes the host, and the reply is answered after
the next flush from the state that flush presents. A failing flush still
answers, with its error beside the value. Diagnostic commands (debug mode,
overlay, profiling, clearing) are refused unless the application enables them,
and nothing the application owns can be changed through it.

```
use pert_ui::{Capacity, Column, Depth, Detail, Element, InspectionQuery, Label, Runtime, Selector};
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
ui.flush(Duration::ZERO)?;
let reply = pending.wait()?;
assert_eq!(reply.value.components[0].key, "title");
assert_eq!(reply.value.components[0].path.as_deref(), Some(["main".to_owned(), "title".to_owned()].as_slice()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Screenshots

A tool can also ask for a screenshot: `client.capture(CaptureRequest::new(
CaptureTarget::Path(..)))` is answered with a [`Capture`] holding the target's
visible region as an RGBA [`FrameImage`] at physical scale, which
[`FrameImage::encode_png`] encodes without any dependency. A target is a whole
window ([`CaptureTarget::Window`]), a rectangle of one
([`CaptureTarget::Region`]), a component ([`CaptureTarget::Component`]) or a
key path ([`CaptureTarget::Path`]). On the UI thread,
[`CaptureTarget::window`] builds a window target from a borrowed
[`NativeWindowId`], so the main window is
`CaptureTarget::window(&ui.primary_window())` ([`Runtime::primary_window`]),
and [`Runtime::request_capture`] asks for the same capture; never wait for its
reply there, since the frame is rendered between flushes. The [`App`] renders the frame
offscreen with the same renderer that presents; a headless runtime answers
[`CaptureError::Unsupported`] unless a host declares that it renders captures
with [`Runtime::set_capture_support`] and completes the jobs
[`Runtime::pending_captures`] lists.

With the `cpu` feature, [`CpuRenderer`] is such a host for a headless runtime:
[`CpuRenderer::serve_captures`] renders each pending job on the CPU, with no
window or GPU, and completes it. Call it after each flush, where a native host
would present.

```
# #[cfg(feature = "cpu")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::cpu::CpuRenderer;
    use pert_ui::{CaptureRequest, CaptureTarget, Element, Label, Runtime, Viewport};
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
    CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    std::fs::write(std::env::temp_dir().join("title.png"), capture.image.encode_png())?;
    // The whole main window.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
        &ui.primary_window(),
    )));
    ui.flush(Duration::ZERO)?;
    CpuRenderer::new().serve_captures(&mut ui);
    let window = pending.wait()?.value?;
    assert_eq!((window.image.width(), window.image.height()), (300, 200));
    Ok(())
}
# #[cfg(not(feature = "cpu"))]
# fn main() {}
```

# Serialization

The `serde` feature, off by default, adds `Serialize` and `Deserialize` to the
snapshot, the profile, the validation report and the workspace document, and to
everything they carry. The application chooses the format and the transport;
deserialization validates through the same constructors as the ordinary API,
and a non-finite diagnostic number is written as `"NaN"`, `"inf"` or `"-inf"`
rather than lost. Enable it with `pert-ui = { version = "0.3", features = ["serde"] }`;
the [feature table](crate#cargo-features) lists every feature.

```
# #[cfg(feature = "serde")]
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
    Ok(())
}
# #[cfg(not(feature = "serde"))]
# fn main() {}
```

# Measuring UI work

Profiling is off by default and adds no scheduled work. Enable it while exercising
an interaction, then pause and capture completed scopes. Total includes nested
work; self time excludes measured descendants. These are elapsed UI-thread times,
including measurement overhead and waiting, not CPU percentages or GPU timings.

[`Runtime::paint`] fails with a [`DrawError`], which does not convert into
[`RuntimeError`]. A helper that flushes and paints, as a test of a scene does,
returns `Result<_, DrawError>`, into which `RuntimeError` and every leaf error
convert with `?`; with the `native` feature, [`AppError`] accepts both.

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```
