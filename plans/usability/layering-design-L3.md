# Part L3 design checkpoint: one way where history left two

Part L3 of [the layering plan](layering-plan.md#part-l3-one-way-where-history-left-two-0373)
(N3, N6 and N7 in the [complexity inventory](complexity-inventory.md#needs-a-decision))
changes public API shape, and N7 also changes the address contract, so its
shape is settled here before any code. Codex reviews it, then the maintainer
approves it. Line references are to the committed tree at 2a406a4 (L1,
and L4a as 0.3.72); revision 1 cited eea2daa, and every reference was
checked again at 2a406a4. The parts now land in the order L4a (0.3.72), L2
(0.3.73), L3 (0.3.74), so this part is 0.3.74 although the plan's heading
still names 0.3.73.

Status: **revision 4, approved 2026-10-27** with the recommended answers to
D1 to D3, by the author together with Codex's review of revision 1 and its
rechecks of revisions 2 and 3, under the maintainer's delegation of
2026-10-26. Revision 4 applies the third recheck's one should-fix point and
nit exactly as Codex specified them (the overlap fixture's migration and one
sentence); L3's implementation review verifies them.

## What users and the library built by hand

- **N3, the `Runtime` page.** `Runtime` has 137 public methods. L1's
  "Methods by audience" index (`runtime.rs:3682-3810`) lists 33 of them under
  "Hosts": only code that drives the runtime in place of `App` calls them.
  They still sit on `Runtime`'s page and in every application's completion
  list (inventory S4 and N3, `complexity-inventory.md:61`, `:465`, `:475`).
  L1's review fix moved seven operations a headless test performs while
  standing in for a host into the tests group (`runtime.rs:3757-3761`), so
  the Hosts group is now exactly the host-only set. The inventory's open
  question 3 (`complexity-inventory.md:504`) asked whether custom hosts are a
  supported audience before 1.0. This part answers yes: they get their own
  module and a public example, and applications no longer see their methods.
- **N6, two ways to mount a root.** `Runtime::mount`
  (`runtime.rs:4522-4553`) mounts a *legacy root* that is never measured,
  arranged or reconciled; its own rustdoc says "Prefer `mount_tree` for
  anything that is laid out". Yet `App`'s first rustdoc example teaches it
  (`app.rs:293`), and so do three runtime rustdoc examples and the CPU
  renderer's crate example (section 2 lists every user).
- **N7, three lookups and a hidden segment.** `find` takes the addressable
  path that inspection and workspaces use (`runtime/tree.rs:278-397`),
  `find_descendant` one application key (`:205-276`), and `find_path` the
  raw keys of every level, "for code that knows how the built-ins it passes
  through are composed" (`:151-203`). `Each`'s default container adds an
  `items` segment to every item's address (`described.rs:274-291`, `:339`).
  That segment is round 1's F9 (`developer-usability.md:422`): both task-4
  testers hit it, and the task kit's README described rows directly under
  `list` while the real path was `list/items/<id>` (triage group E,
  `runs/triage/group-E.md:34`, `:53`, `:90`). The library then documented the
  segment instead of removing it: an example of the lookup on `Each`
  (CHANGELOG 0.3.55, "Smaller additions") and a paragraph in the testing
  chapter (`guide/testing.md:201-205`). The inventory groups F9 with F26 and
  F31 as the address rules that recurred (`complexity-inventory.md:479`).

## 1. N3: host integration in a `host` module

### What moves

Exactly the 33 methods of L1's Hosts group move from inherent `Runtime`
methods to the trait `HostIntegration` in a new `host` module. Their
signatures, behavior and errors do not change. Consumers outside
`pert-ui-runtime`'s sources are listed per method:

- **F:** the facade's sources (`crates/pert-ui/src`); `*_smoke.rs` are the
  opt-in native checks (`platform/mod.rs:2-21`, all `#[cfg(test)]`).
- **G:** the gallery (`apps/gallery/src`), **E:** the packaged examples,
  **C:** `crates/pert-ui-cpu`, **D:** `tests/diagnostics/ui`.
- **Facade tests** by area binary (`crates/pert-ui/tests/<area>`), **runtime
  tests** by file (`crates/pert-ui-runtime/tests`), with call counts.

`pert-ui-wgpu` and `apps/native-checks` call none of the 33 (the wgpu
crate's checks call `paint`, `set_viewport` and `mount`, the focus probe
`set_viewport`, `flush` and `paint`; `pert-ui-focus-probe.rs:168-175`).

| Method | Defined at | Outside consumers | Facade tests | Runtime tests |
|---|---|---|---|---|
| `flush_windows` | `runtime/native_windows.rs:1539` | G:tests/layout.rs 3, G:tests/shell.rs 3; F:app/close.rs 1, F:app/headless.rs 1 | audit 10, collections 11, controls 2, foundation 16, surfaces 10 | live_sources 4 |
| `flush_commands` | `runtime/commands.rs:1682` | F:platform/mod.rs 1 | collections 3 | — |
| `has_command_requests` | `runtime/commands.rs:1687` | F:platform/mod.rs 1 | collections 2 | — |
| `application_deadline` | `runtime/native_windows.rs:1500` | F:platform/multi_window_smoke.rs 1 | audit 10, collections 3, foundation 6, surfaces 1 | live_sources 3 |
| `interval_deadline` | `runtime/application.rs:356` | F:platform/mod.rs 1 | — | — |
| `service_intervals` | `runtime/application.rs:374` | F:platform/mod.rs 1 | foundation 5 | — |
| `service_inspection` | `runtime.rs:4498` | F:platform/schedule.rs 3 | foundation 1 | — |
| `set_wake_delivery` | `runtime.rs:4518` | F:platform/mod.rs 1 | foundation 2 | live_sources 9, mailbox 1 |
| `select_window` | `runtime/native_windows.rs:325` | G:interaction_check.rs 2, G:tests/layout.rs 1, G:tests/shell.rs 1; F:app/close.rs 4, F:app/headless.rs 5, F:platform/windows.rs 4, F:platform/multi_window_smoke.rs 2 | audit 37, collections 25, controls 2, foundation 7, surfaces 61 | live_sources 3, testing 4 |
| `current_window` | `runtime/native_windows.rs:298` | F:app/close.rs 3, F:app/headless.rs 1, F:platform/windows.rs 1 | audit 15, collections 8, controls 2, surfaces 15 | live_sources 1, testing 2 |
| `set_window_visible` | `runtime/native_windows.rs:383` | G:interaction_check.rs 2, G:workspace_check.rs 2; C:tests/oracle.rs 1; F:platform/mod.rs 1 | audit 11, collections 3, editing 2, foundation 6, surfaces 7 | live_sources 1 |
| `set_presentation_rate` | `runtime/presentation.rs:79` | F:platform/schedule.rs 1 | foundation 1 | live_sources 2 |
| `presentation_rate` | `runtime/presentation.rs:110` | F:platform/pacing_smoke.rs 1 | — | — |
| `needs_paint` | `runtime/compose.rs:338` | G:live.rs 6, G:interaction_check.rs 1, G:tests/data.rs 2, G:tests/text.rs 1, G:tests/tools.rs 4; F:platform/mod.rs 1, F:platform/schedule.rs 1, F:platform/smoke.rs 6, F:platform/drag_smoke.rs 1 | controls 3, foundation 7, presentation 29 | debug_overlay 7 |
| `invalidate` | `runtime.rs:5561` | G:interaction_check.rs 2 | audit 9, editing 2, foundation 10, presentation 3 | debug_overlay 1, input_contracts 2, runtime 1, validation 2 |
| `has_presentation_requests` | `runtime/native_windows.rs:1315` | F:platform/mod.rs 1 | — | — |
| `take_presentation_requests` | `runtime/native_windows.rs:1202` | G:interaction_check.rs 1, G:tests/layout.rs 4, G:tests/shell.rs 1; F:platform/windows.rs 1 | collections 4, controls 1, foundation 1, surfaces 43 | — |
| `prepare_detach` | `runtime/native_windows.rs:427` | G:interaction_check.rs 1, G:tests/layout.rs 2, G:tests/shell.rs 1; F:platform/windows.rs 2 | audit 6, collections 7, controls 6, foundation 3, surfaces 47 | live_sources 2, testing 2 |
| `commit_detach` | `runtime/native_windows.rs:660` | G:interaction_check.rs 1, G:tests/layout.rs 1, G:tests/shell.rs 1; F:platform/windows.rs 2 | audit 6, collections 6, controls 6, foundation 3, surfaces 39 | live_sources 2, testing 2 |
| `redock` | `runtime/native_windows.rs:770` | G:tests/layout.rs 1, G:tests/shell.rs 1; F:platform/windows.rs 2 | audit 3, collections 1, foundation 1, surfaces 18 | — |
| `close_detached_window` | `runtime/native_windows.rs:1297` | G:interaction_check.rs 1; F:platform/windows.rs 1 | foundation 1, surfaces 5 | — |
| `native_window_options` | `runtime/native_windows.rs:859` | — | — | — |
| `text_preedit` | `runtime.rs:5262` | F:platform/ime.rs 2 | audit 1, controls 5, editing 23 | — |
| `text_input_state` | `runtime.rs:5226` | G:interaction_check.rs 2; F:platform/ime.rs 1, F:platform/mod.rs 1, F:platform/inline_smoke.rs 1 | controls 12, editing 16 | — |
| `cursor_icon` | `runtime/pointer.rs:43` | G:tests/data.rs 2; F:platform/mod.rs 1, F:platform/drag_smoke.rs 1 | collections 4, controls 3, editing 4, presentation 12, surfaces 21 | — |
| `clear_focus` | `runtime.rs:5413` | G:main.rs 1, G:interaction_check.rs 2, G:tests/controls.rs 3, G:tests/text.rs 1; E:diagram_editor/session.rs 2; F:app/close.rs 1, F:platform/mod.rs 2, F:platform/motion.rs 2, F:platform/smoke.rs 3, F:platform/multi_window_smoke.rs 1; `guide/controls.md:452`; runtime `README.md:945` | audit 12, collections 7, controls 23, editing 28, foundation 4, presentation 2, surfaces 14 | diagnostics 5, runtime 3 |
| `cancel_input` | `runtime.rs:5352` | G:tests/controls.rs 1; F:platform/schedule.rs 1 | audit 1, collections 2, controls 3, editing 1, foundation 4, surfaces 2 | — |
| `captures_supported` | `runtime/capture.rs:44` | — | foundation 1 | — |
| `pending_captures` | `runtime/capture.rs:279` | C:src/lib.rs 1; F:platform/mod.rs 1, F:platform/schedule.rs 2 | foundation 25 | — |
| `capture_scene` | `runtime/capture.rs:314` | C:src/lib.rs 1; F:platform/mod.rs 1, F:platform/schedule.rs 1 | foundation 12 | — |
| `complete_capture` | `runtime/capture.rs:352` | C:src/lib.rs 1; F:platform/mod.rs 3 | foundation 19 | — |
| `set_clipboard` | `runtime.rs:5162` | E:inventory/session.rs 1, E:log_explorer/session.rs 1; F:platform/mod.rs 1; D:k01_clipboard_not_impl.rs 1 | audit 1, controls 3, editing 34, foundation 3 | — |
| `take_shutdown_errors` | `runtime.rs:6794` | E:diagram_editor/session.rs 1; F:platform/mod.rs 2 | audit 5, surfaces 8 | — |

In total about 1,080 call sites in 126 files outside the runtime's sources
(L4a's new `foundation/deferral.rs` and `surfaces/deferral.rs` included).
The CPU renderer serves captures through three of them
(`pert-ui-cpu/src/lib.rs:264-276`). Rust code inside `pert-ui-runtime`'s own
sources (the driver, the tasks, the runtime's own modules) is not changed
(see [implementation](#implementation)); its rustdoc examples are, because
they compile as external crates
([doctests](#rustdoc-examples-that-call-a-moved-method)).

### What stays on `Runtime`

Everything in L1's "Applications" and "Tests and inspection" groups stays
inherent, including the seven that L1's review fix put under "Standing in
for a host": `set_viewport`, `viewport`, `paint`, `command_key_input`,
`native_modifiers`, `keyboard` and `set_capture_support`. The evidence is
that ordinary code and prelude-only documentation call them:

- `set_viewport` and `paint` are how a headless runtime lays out and shows
  its drawing: the guide's application chapters call them with the prelude
  or explicit root imports only (`guide/layout.md:41`, `:52`,
  `guide/appearance.md:64`, `guide/controls.md:27`, `:37` and some fifty
  more), as do the facade's `testing` example (`lib.rs:171`) and the runtime
  crate's README.
- `set_capture_support` is called by application code that renders its own
  PNG with the CPU renderer (`examples/diagram_editor/app.rs:225`) and by the
  diagnostics chapter's capture example (`guide/diagnostics.md:164`). Its
  query, `captures_supported`, moves with the serving methods as L1 placed
  it: only a host that serves captures reads it (one test calls it,
  `foundation/capture.rs`).
- `command_key_input`, `native_modifiers` and `keyboard` are what the
  testing and diagnostics chapters teach for shortcut tests
  (`guide/testing.md:134-165`, `guide/diagnostics.md:10-55`).

The plan named "`paint` for hosts" among the moves; `paint` stays (see
[corrections](#corrections-to-the-plan-and-the-inventory)).

### An extension trait, not a handle

The plan left the choice open. The evidence decides for a sealed extension
trait implemented by `Runtime`:

1. **Mixed receivers.** 17 of the 33 take `&self` (`has_command_requests`,
   `application_deadline`, `interval_deadline`, `set_wake_delivery`,
   `current_window`, `set_presentation_rate`, `presentation_rate`,
   `needs_paint`, `invalidate`, `has_presentation_requests`,
   `take_presentation_requests`, `native_window_options`,
   `text_input_state`, `cursor_icon`, `captures_supported`,
   `pending_captures`, `complete_capture`) and 16 `&mut self`. A handle
   borrowed from `&mut Runtime` cannot be made where a host holds only
   `&Runtime`, which the facade does: `platform/ime.rs:12`
   (`text_input_state`) and `platform/schedule.rs:103-111`
   (`pending_captures`). A handle would need a shared and a mutable variant.
2. **Borrowed results.** `native_window_options` returns
   `Option<&NativeWindowOptions>` borrowed from the runtime; through a
   handle it would borrow from the handle's lifetime instead, one more
   lifetime for a host to manage.
3. **Interleaving.** Hosts alternate host and ordinary calls
   (`app/close.rs:24-90` mixes `primary_window`, `clear_focus`,
   `select_window` and `flush_windows`); a handle must be re-borrowed for
   each host call.
4. **Churn.** With a trait the 1,080 call sites stay as written and each
   file gains one import (none for tests that glob `testing`, below); a
   handle rewrites every call (`ui.host().select_window(...)`).
5. **Precedent.** `StateOwner` is already a sealed trait implemented by
   `Runtime` (`signal.rs:24-30`, `:87`, `:111-125`).

The cost of the trait: the methods are documented on the trait's page, and
the methods with `impl` arguments (`set_wake_delivery`, `set_clipboard`) make
it unusable as a trait object, which no host needs; the rustdoc says so.

### Public API

A new module in `pert-ui-runtime`, `src/host.rs`, declared `pub mod host;`
in `lib.rs`:

```rust
pub mod host {
    /// The methods a host calls to drive a `Runtime` in place of `App`.
    pub trait HostIntegration: sealed::HostIntegrationSeal {
        // Event loop
        fn flush_windows(&mut self, now: Duration) -> Result<FlushReport, RuntimeError>;
        fn flush_commands(&mut self) -> Result<(), RuntimeError>;
        fn has_command_requests(&self) -> bool;
        fn application_deadline(&self) -> Option<Duration>;
        #[must_use]
        fn interval_deadline(&self) -> Option<Duration>;
        fn service_intervals(&mut self, now: Duration) -> Result<(), RuntimeError>;
        fn service_inspection(&mut self) -> Result<(), RuntimeError>;
        fn set_wake_delivery(&self, deliver: impl Fn() -> bool + Send + Sync + 'static);
        // Windows and presentation
        fn select_window(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError>;
        fn current_window(&self) -> NativeWindowId;
        fn set_window_visible(&mut self, visible: bool) -> Result<(), RuntimeError>;
        fn set_presentation_rate(&self, rate: Option<PresentationRate>) -> Result<(), RuntimeError>;
        fn presentation_rate(&self) -> Option<PresentationRate>;
        #[must_use]
        fn needs_paint(&self) -> bool;
        fn invalidate(&self, id: &WidgetId, kind: Invalidation) -> Result<(), RuntimeError>;
        fn has_presentation_requests(&self) -> bool;
        fn take_presentation_requests(&self) -> Result<Vec<PresentationRequest>, RuntimeError>;
        fn prepare_detach(
            &mut self,
            view: &WidgetId,
            options: NativeWindowOptions,
        ) -> Result<PreparedDetach, RuntimeError>;
        fn commit_detach(&mut self, prepared: PreparedDetach) -> Result<NativeWindowId, RuntimeError>;
        fn redock(&mut self, window: &NativeWindowId) -> Result<(), RuntimeError>;
        fn close_detached_window(&mut self) -> Result<PresentationStatus, RuntimeError>;
        fn native_window_options(&self) -> Option<&NativeWindowOptions>;
        // Native input
        fn text_preedit(&mut self, preedit: &TextPreedit) -> Result<(), RuntimeError>;
        fn text_input_state(&self) -> Option<(WidgetId, TextInputState)>;
        fn cursor_icon(&self) -> CursorIcon;
        fn clear_focus(&mut self);
        fn cancel_input(&mut self) -> Result<(), RuntimeError>;
        // Captures
        #[must_use]
        fn captures_supported(&self) -> bool;
        #[must_use]
        fn pending_captures(&self) -> Vec<CaptureJob>;
        fn capture_scene(&mut self, job: &CaptureJob) -> Result<DisplayList, CaptureError>;
        fn complete_capture(&self, job: &CaptureJob, frame: Result<FrameImage, CaptureError>) -> bool;
        // Services and shutdown
        fn set_clipboard(&mut self, clipboard: impl Clipboard + 'static) -> Result<(), RuntimeError>;
        #[must_use]
        fn take_shutdown_errors(&mut self) -> Vec<RuntimeError>;
    }

    impl HostIntegration for Runtime { /* forwards, see below */ }

    // `#[doc(no_inline)]` re-exports of the names a host writes:
    pub use crate::{
        CaptureError, CaptureJob, Clipboard, ClipboardError, CommandKey, CommandKeyInput,
        CursorIcon, DisplayList, DrawError, FlushReport, FrameImage, Invalidation, KeyAction,
        KeyCode, KeyDispatch, KeyboardState, LimitError, Magnification, Modifiers,
        NativeWindowId, NativeWindowOptions, PhysicalExtent, Point, PointerButton,
        PointerEvent, PointerKind, PreparedDetach, PresentationAction, PresentationRate,
        PresentationRequest, PresentationStatus, Runtime, RuntimeError, TextInputState,
        TextPreedit, Viewport, WidgetId, WindowError,
    };
}
```

- **The `#[must_use]` attributes** are today's (`application.rs:355`,
  `compose.rs:337`, `capture.rs:43`, `:278`, `runtime.rs:6793`); no
  attribute is added or dropped.
- **The re-exports** follow G5-26's operational test: a name is listed when
  a host writes it (constructs, matches or keeps it) while calling the trait
  or the inherent methods a host also calls (`set_viewport`, `paint`,
  `pointer_input`, `text_input`, `command_key_input`, `native_modifiers`,
  `set_capture_support`, `flush`, `shutdown`). The set was audited against
  what the native host writes in its two jobs:
  - **event translation:** pointer input builds `PointerEvent`,
    `PointerKind`, `PointerButton`, `Point` and, for a pinch,
    `Magnification` (`platform/motion.rs:2`, `:36-49`;
    `platform/mod.rs:278-320`: `PointerKind::Scroll(Point)`,
    `PointerKind::Pinch`); keys build `CommandKeyInput`, `CommandKey`,
    `KeyCode`, `KeyAction` and `Modifiers` and keep `KeyDispatch`
    (`platform/keyboard.rs:2`); IME builds `TextPreedit` and keeps the
    `TextInputState` target (`platform/ime.rs:2`, `:12`); a window's size
    builds `PhysicalExtent` and `Viewport` (`platform/mod.rs:25`);
  - **frame production and services:** `paint` returns `DrawError`, which
    the host matches (`DrawError::PendingUpdates` means flush first); a
    capture is completed with a `FrameImage` or a `CaptureError`, whose
    frame-size case the host builds as `CaptureError::Limit(LimitError)`
    (`platform/presenter.rs:32`, `:254`); presentation requests are matched
    by `PresentationAction` and completed with `PresentationStatus` or
    `WindowError`; a clipboard is a `Clipboard` that returns
    `ClipboardError`.

  **Excluded,** with where they stay: `KeyEvent` and `KeyTransition` (a
  host reads `KeyDispatch::event` and passes text on; `component` and the
  root), `GeometryError` (only propagated from `Viewport`, `Point` and
  `Magnification` constructors; root), `Size` and `Rect` (read from
  `NativeWindowOptions` and `TextInputState`; root and `component`),
  `FocusDirection` (used by `App`'s tests, not its host code; root), and
  the draw-command types a renderer matches (`component` and root; a
  renderer is not a host). L2's draft
  checkpoint leaves the presentation plumbing (`PresentationRequest`,
  `PresentationAction`, `PreparedDetach`, `PresentationRate`) and
  `CaptureJob` out of its thematic modules for this part to place
  (`layering-design-L2.md:257-259`, `:275-276`); they are here. All the
  names stay at the crate root as well.
- **Seal.** `sealed::HostIntegrationSeal` follows `StateOwner`'s pattern
  (`signal.rs:111-125`): a distinct name and a
  `#[diagnostic::on_unimplemented]` message saying that `HostIntegration`
  is sealed and cannot be implemented for the type, with a note that only
  `Runtime` implements it and that a host imports the trait to call its
  methods. Sealing keeps later additions non-breaking.
- **Rustdoc intent.** The trait's page states: what a host is (code that
  drives the runtime in place of `App`, including a headless test that
  stands in for one); that only `Runtime` implements it and why it is
  sealed; how to import it (`pert_ui::host::HostIntegration`, or
  `pert_ui::testing::*` in a test); that it is not a trait object; the
  inherent methods a host also calls, by link; the method groups as L1's
  index had them; and a link to contracts.md's
  [custom hosts](../../docs/contracts.md#custom-hosts). Each method keeps its
  current rustdoc, errors, panics, timing and threading text unchanged in
  meaning (`set_wake_delivery`'s callback runs on the notifying thread,
  `runtime.rs:4508-4518`), with two mechanical edits: intra-doc links written
  `Self::flush` and similar now resolve against the trait, so links to
  inherent methods become `Runtime::...`, and links between moved methods
  become `Self::...` on the trait.

### Exposure

- **`pert_ui_runtime::host`** holds the trait and the re-exports. The trait
  is **not** re-exported at the runtime's crate root or in its `prelude`
  or `component`.
- **`pert_ui::host`.** The facade's root glob (`lib.rs:29`) would re-export
  the runtime's `host` module; the facade instead defines its own documented
  `pub mod host`, as it does for `component` and `testing` (`lib.rs:143`,
  `:186`), with `#[doc(inline)] pub use pert_ui_runtime::host::HostIntegration;`
  and the same `#[doc(no_inline)]` re-exports. Its page carries the
  [custom-host example](#the-public-custom-host-example). The facade adds
  no host names of its own; `App` is not in it.
- **`testing` re-exports the trait** (`#[doc(no_inline)] pub use
  crate::host::HostIntegration;`), in `pert_ui_runtime::testing` and in the
  facade's `testing` module (`lib.rs:186-199`). A headless test drives the
  runtime in place of `App`, so it is a host; the module documentation says
  so in one sentence. The two sentences that say every re-exported name
  stays at the crate root, `lib.rs:161` in the facade and
  `pert-ui-runtime/src/testing.rs:20-21` in the runtime ("These names stay
  at the crate root too; the module only gathers them"), except the trait,
  which is reached through `host` and `testing` only. This keeps L2's
  guarantee: tests of the
  representative programs compile with `prelude` and `testing` whichever of
  the 33 they call.
- **Application code no longer sees the methods.** Neither
  `use pert_ui::prelude::*` nor `use pert_ui::*` (as in
  `examples/dynamic_documents.rs:2`) brings the trait into scope; calling a
  moved method then fails to compile with rustc's "trait not in scope" help
  naming `HostIntegration`. A custom component (`prelude` and `component`)
  needs none of them: its hooks receive contexts, not a `Runtime`. A
  deferred task receives `&mut Runtime` (`EventContext::defer`) for
  application operations; a task that drives host behavior is acting as a
  host and imports the trait. The custom-widget chapter calls none of the
  33.
- **Collision review.** `HostIntegration` is the only new name, and no item
  of that name exists in the workspace or its dependencies' preludes. The
  module's other names are the same items the root, `prelude`, `component`
  and `testing` re-export (and L2's thematic modules, which hold the same
  items), so overlapping globs of `host` with any of them are valid. A
  consumer-local `HostIntegration` is not a common name.

### Implementation

- **One body per method, forwarded.** Each moved method stays an inherent
  method of the same name with `pub(crate)` visibility and its body
  unchanged; `impl HostIntegration for Runtime` forwards each with
  `Runtime::name(self, ...)`. The runtime's own callers (its driver,
  windows, commands, tasks and capture code) therefore do not change.
  Outside the crate, rustc's method probe skips the inaccessible inherent
  method and resolves the trait method when the trait is in scope; the
  doctests in [section 6](#n3-new-behavior-tests) prove both directions. If
  the probe did not skip it, the fallback is to rename the inherent methods
  (`*_host`), which changes only crate-internal code.
- **Documentation moves with the methods.** The rustdoc of each moved
  method moves to the trait declaration; the `pub(crate)` method keeps a
  one-line comment pointing to it.
- **References elsewhere.** About 115 mentions of `Runtime::<moved>` or
  `Self::<moved>` in rustdoc, the guide, the READMEs and `docs/` (26 files;
  a few `Self::invalidate` and `Self::cancel_input` name the contexts'
  methods of the same name and stay) become `HostIntegration::<moved>`
  links or plain names. Warning-free
  rustdoc catches every intra-doc link; the plain-text mentions are found by
  `rg` over the same 33 names, listed in the review record.
- **`Runtime`'s type documentation:** the "Hosts" group of L1's index
  (`runtime.rs:3777-3810`) is replaced by one paragraph: a host imports
  `host::HostIntegration`, whose page lists its methods; the "Standing in
  for a host" group stays and says that a host calls these as well. The
  overview sentence "Custom hosts select a root before local input, flush
  and paint, and coordinate roots through application_deadline/flush_windows"
  (`runtime.rs:3655-3657`) links the trait.
- **No privileged path.** `App` and its platform adapter are in another
  crate than the trait, so they call it through the same public import a
  custom host writes; whatever `App` does with the runtime, a custom host
  can do.
- **No dependency, feature or backend type changes.** Every signature names
  only pert-ui types; nothing of winit, wgpu or softbuffer appears.

### Rustdoc examples that call a moved method

Every rustdoc example compiles as an external crate, so an example that
calls one of the 33 needs the trait in scope, though the Rust code around
it does not change. The rule by audience: an example of a host operation
itself imports `pert_ui::host::HostIntegration`; an example that shows a
component's behavior and stands in for a host to do so imports it through
`testing` (`use pert_ui::testing::HostIntegration;`), as a test would. A
search of every `///` and `//!` example line in the runtime, facade and
CPU crates and of the runtime README for a call of the 33 at 2a406a4 finds:

| Example | Calls | Import |
|---|---|---|
| `prepare_detach`'s own (`runtime/native_windows.rs:402-416`) | `prepare_detach`, `commit_detach`, `redock` | moves with the method to the trait; `host` |
| `cancel_input`'s own (`runtime.rs:5336-5348`) | `cancel_input` | moves to the trait; `host` |
| `take_shutdown_errors`'s own (`runtime.rs:6766-6792`) | `take_shutdown_errors` | moves to the trait; `host` |
| `Element::with_detachable` (`layout.rs:534-546`) | `prepare_detach` (the refusal) | `testing` |
| `EventContext::now` (`runtime.rs:3087-3132`) | `service_intervals` | `testing` |
| `Runtime::set_clipboard_text` (`runtime.rs:5195-5216`) | `set_clipboard` (a memory clipboard) | `testing` |
| `SelectableText` (`selectable_text.rs:83-107`) | `set_clipboard` | `testing` |
| `DecimalInput` (`text_fields/number_inputs.rs:202-216`) | `clear_focus` | `testing` |
| `ToolWindowMode` (`tool_windows/mod.rs:127-156`) | `take_presentation_requests` | `testing` |
| runtime README, number inputs (`README.md:929-948`) | `clear_focus` | rewritten as the guide's copy (section 4): `ui.focus(&fields[0])?` |

The facade's sources and the CPU crate's rustdoc call none of the 33; the
guide's calls are listed in section 4. The implementation repeats the
search after the move; warning-free doctests are the check that none was
missed.

### The public custom-host example

The facade's `host` module page carries a complete program, importing only
`pert_ui::host::*` and `pert_ui::prelude::*` (plus `std`), that runs as an
ordinary doctest in every supported feature configuration, since it needs no
window or GPU. It is a host without a window system:

1. installs wake delivery before any work (`set_wake_delivery` with a flag a
   loop would wait on);
2. supplies a viewport from a physical surface size, as a window system
   reports it: `Viewport::from_physical(PhysicalExtent::new(640, 400), 2.0)`;
3. mounts a content tree with two buttons and a view;
4. runs one turn of its loop: `flush_windows(now)`; completes every
   `take_presentation_requests` request, answering the view's detach (made by
   `request_detach`) with `PresentationStatus::Failed` because it has no
   windows, and asserts the receipt's status; paints when `needs_paint` and
   "presents" the `DisplayList` by counting its commands; reads
   `application_deadline` to decide how long to sleep;
5. translates a native key: `command_key_input` with Space on the focused
   button, delivering text only when the dispatch says so, and asserts the
   button's effect after the next `flush_windows`;
6. translates a native pointer click: `pointer_input` with
   `PointerEvent::new(point, PointerKind::Press(PointerButton::Primary))`
   and the matching release at the second button's centre (from
   `Runtime::bounds`), reads `cursor_icon` as a host would after a move, and
   asserts the second button's effect after the next `flush_windows`;
7. shuts down and drains `take_shutdown_errors` before dropping the
   runtime, as contracts.md's custom-host section requires.

Prose beside it names the remaining duties with links: IME through
`text_input_state` and `text_preedit`, window deactivation through
`clear_focus`, captures through `set_capture_support`, `pending_captures`,
`capture_scene` and `complete_capture` (or `cpu::CpuRenderer::serve_captures`
with the `cpu` feature), the clipboard through `set_clipboard`, and multiple
roots through `select_window` and `flush_windows`. The runtime crate's `host`
module page has a shorter doctest (one flush, the requests taken, a paint)
for direct users of `pert-ui-runtime` such as the CPU renderer.

The packaged-consumer check compiles the same program in an isolated
consumer built from the extracted packages, beside the documented examples
and the custom widget it already builds (`docs/verification.md:393-399`).

## 2. N6: `Runtime::mount` is removed

- **The rule.** `Runtime::mount` (`runtime.rs:4522-4553`) is removed. Every
  root is mounted with `mount_tree`; nothing replaces the legacy root in
  `testing`. The plan allowed a `testing` form if a test needs an unlaid
  root. The only test that does is the one that checks the driver's error
  for the legacy root itself
  (`crates/pert-ui-runtime/tests/testing.rs:358-395`, `TestError::NotLaidOut`
  for a root "mounted without a tree"); it tests the removed feature.
- **The migration.** `ui.mount(key, component)` becomes
  `ui.mount_tree(Element::new(key, component))`, with the same key rules and
  errors (`DuplicateKey` for a root key in use, `EmptyKey`, `Exhausted`;
  `runtime/tree.rs:86-96`). A sole root with the default size limits draws
  the same pixels: a composed root is measured against the viewport and
  arranged into the whole viewport rectangle at the window origin
  (`runtime/tree.rs:1968-1993`), so its local coordinates are the window
  coordinates the legacy root painted in, and `painter.viewport()` stays
  available.
- **What a laid-out root adds**, which a migrated call may meet:
  - bounds, so it takes part in hit testing (`Component::hit_test` defaults
    to true over the body, `runtime.rs:2200-2237`) and in focus navigation;
  - development validation's peer overlap check: two visible roots of one
    window are peers with no parent, so two roots with area that intersect
    report `UnexpectedOverlap` (`validation/development.rs:678-693`). With
    no viewport, or an empty one, development validation withholds
    geometric checks (`runtime.rs:4152`);
  - the runtime's allocation replaces a component's self-reported border box
    in inspection and validation (`runtime/inspect.rs:186`, `runtime.rs:6206`
    apply only to composed entries), and the development overlay outlines
    the root.

  It adds no update: `FlushReport::components_updated` counts update calls
  (`runtime.rs:3640-3647`), and a root's first update is scheduled once
  either way (`runtime.rs:4533`, `runtime/tree.rs:84`).
- **Migration criteria.** A migrated test keeps every assertion,
  including update counts, deadlines, scheduling, root order and keys, and
  the identity of each returned `WidgetId`. So the default is fixed:
  **one `mount_tree` root per former `mount`, with the same key, in the
  same order**, which keeps the root topology and adds no component (no
  container is introduced to group roots). The exceptions, each named
  here so that no assertion is weakened during the implementation:
  1. **Roots with area that overlap, in a test that checks validation:**
     each such component reports `OverlapPolicy::Allow` in
     `ComponentDetails::validation`. Found by a search of every function
     that mounts two or more roots with a viewport and reads validation:
     the `debug_overlay.rs` and `validation.rs` tests of item 3 below; the
     other multi-root tests have no viewport (`tests/runtime.rs:41-68`,
     whose three probes and "3 updated" assertion stay as they are,
     `recovery.rs`, `remote_authority.rs`) or do not read validation.
  2. **Overlay probes above interactive content** (the gallery's interaction
     and restart probes, `interaction_check.rs:1108`, `:1172`,
     `workspace_check.rs:196`, `:257`): `hit_test` returns false so pointer
     input reaches the gallery, and `OverlapPolicy::Allow`. Their
     hidden-window reporting (TODO.md NATIVE-04) must keep working; the
     native checks cover it.
  3. **Tests whose components report their own geometry,** which only a
     legacy root could do: `Observed::at([x, y, w, h])` in
     `crates/pert-ui-runtime/tests/debug_overlay.rs` (all 7 calls) and the
     `border_box` probes of `validation.rs:51-90` and `:110-131`. They
     migrate to one `mount_tree` of a test-local `Place` container, a
     custom `Component` whose `arrange` places each child at its listed
     rectangle with `ArrangeContext::place` or `place_clipped`
     (`layout.rs:1073-1102`; public API, as the `Pinned` container in the
     same file already does, `debug_overlay.rs:222-240`). Real layout then
     supplies the bounds and clips the fixtures used to report, and
     development validation outlines only what is visible inside them
     (`validation/development.rs:496-508`). So the migration is not
     count-preserving by itself; per test:
     - **`outlines_stop_at_the_cap_and_the_badge_counts_the_rest`**
       (`debug_overlay.rs:134-169`): its markers sit 10 pixels apart, and
       with real clipping only those inside the 200 × 100 viewport keep
       drawable bounds, so the test would no longer reach the cap. The
       viewport becomes 200 × 700, which holds all
       `MAX_OVERLAY_MARKERS + 3` (67) markers; the badge's position
       (top-right, `plate.x() + plate.width() == 200`) and every other
       assertion stay. The `Place` root has no issue, so it adds no marker.
       The "never measured" component (`:145-148`) is placed at an empty
       rectangle: `visible_rect` returns `None` for it (`:500-503`), so it
       keeps its issue and a marker without bounds, which is what the
       assertion at `:154` states.
     - **`boxes_outline_border_and_content_boxes_without_markers`**
       (`:176-193`) and **`boxes_stop_at_their_cap`** (`:197-218`): the
       `Boxes` overlay outlines every box, the `Place` root's too. Their
       assertions are **authorized to change** for that one box: the
       first test expects 16 commands instead of 12 (`:187`), the root's
       four edges first, and its indexed assertions move by four
       (`drawn[4]` and `drawn[8]` for today's `drawn[0]` and `drawn[4]`);
       the second keeps `MAX_OVERLAY_BOXES * 4`, since the root is one of
       the boxes and the cap still holds (its grid, 384 × 102 pixels, fits
       the 400 × 400 viewport).
     - **`overlap_and_shortcut_markers_follow_fixes_visibility_and_scope`**
       (`validation.rs:51-104`) is split, an **authorized exception** for
       its scheduling and key assertions:
       - The overlap and shortcut coverage stays under `Place`: probes `a`
         and `b` are placed at the rectangle they reported
         (`[0, 0, 20, 20]`) in the 100 × 100 viewport, and the issue,
         marker, fix (`OverlapPolicy::Allow`, `FocusedComponent` scope) and
         `ShortcutConflict` assertions stay (`:66-90`). As children, hiding
         `b` and unmounting `a` invalidate the surviving parent's layout
         (`runtime.rs:5510`, `:5538`), so where the legacy roots left
         `next_deadline() == None` (`:91`, `:102`) the test asserts that
         layout is scheduled (`next_deadline() == Some(ui.now())`) right
         after `set_visible(&b, false)`, then flushes and asserts the idle
         state (`next_deadline() == None`) and no markers; the same after
         `unmount(&a)`.
       - The duplicate-root admission and marker cleanup (`:92-103`) move
         to a separate fixture with independent `mount_tree` roots,
         because once `a` is a child its key no longer collides with a new
         root's: duplicate detection keys by `(parent, key)`
         (`runtime/tree.rs:1174`). The new fixture mounts root `a` with
         `mount_tree`, asserts that a second `mount_tree` of a root keyed
         `a` fails with `DuplicateKey("a")` and records
         `ValidationCode::DuplicateKey`, then unmounts `a` and asserts no
         markers and `next_deadline() == None`.
     - **`clipping_is_valid_but_zero_sized_interactive_boxes_are_errors`**
       (`validation.rs:110-131`): the probe changes its reported
       `border_box` from `[0, 30, 20, 20]` to zero and expects
       `EmptyHitArea`. Migrated, the `Place` container reads the child's
       rectangle from shared placement state (an `Rc<Cell<Rect>>` the test
       keeps) and places it with `place_clipped(0, rect, [0, 0, 20, 20])`,
       so the first flush has the clipped-away box and no issue; the test
       then sets the shared rectangle to an empty one and invalidates the
       container's layout (`Runtime::invalidate`, through the trait), and
       the assertion `issues[0].code == ValidationCode::EmptyHitArea` stays.
       The probe still reports `interactive` and its negative margin
       itself.
  4. **An assertion of absent geometry** (`diagnostics.rs:53`,
     `border_box == None` for a legacy root after a flush): a composed root
     laid out without a viewport has the empty box that layout assigns
     (contracts.md "Inspection and diagnostics", `contracts.md:1294-1300`).
     The test asserts that empty box after the flush, and `None` on a root
     inspected before its first flush, which is the documented "unknown
     geometry remains absent until measured".
  5. **Several readouts in the gallery's headless check**
     (`gallery/main.rs:228-241`): two roots, no viewport; default rule.
     **A press over a migrated root** (`platform/mod.rs:1163-1196`,
     `pending_shutdown`): the press at (5, 5) now targets the root, whose
     `pointer_input` is the default `Ignored`; the drag the test then starts
     through `Runtime::event` and the shutdown assertions do not depend on
     the press's target, so the default rule holds.
  6. **Rustdoc examples:** `mount_tree`, and the example's component draws
     in its allocation (`app.rs:293`, `drawing.rs:1641`, `geometry.rs:664`,
     `:758`, `pert-ui-cpu/src/lib.rs:106`).
  7. **The test of the legacy root itself** is deleted, not replaced (see
     [tests](#n6-and-find_path-tests)).

- **Every user** (118 test calls, the rest listed by line):

  | Consumer | Calls |
  |---|---|
  | `pert-ui-runtime` rustdoc | `drawing.rs:1641`, `geometry.rs:664`, `:758` |
  | `pert-ui-runtime` unit tests | `parameter_binding.rs:1062`, `:1109`; `shapes.rs:545` |
  | `pert-ui-runtime` integration tests | `debug_overlay.rs` 7, `diagnostics.rs` 5, `drawing.rs` 2, `live_sources.rs` 16, `parameter_delivery.rs` 1, `runtime.rs` 27, `testing.rs` 2, `validation.rs` 5 |
  | facade rustdoc | `app.rs:293` (`App`'s first example) |
  | facade unit and native checks | `platform/mod.rs:1175`, `platform/schedule.rs:342`, `:452`, `:565`, `platform/smoke.rs:950` |
  | facade integration tests | audit 4 (`audit_toasts.rs`, `audit_window_cleanup.rs`), collections 8 (`commands.rs`, `file_browser.rs`, `reveal.rs`, `tree.rs`), controls 3 (`field_status.rs`, `number_inputs.rs`), editing 1 (`text_fields.rs`), foundation 31 (`arc_geometry.rs`, `capture.rs`, `coordinates.rs` 6, `foundation.rs`, `receipts.rs` 2, `recovery.rs` 3, `remote_authority.rs` 10, `semantics.rs`, `transforms.rs` 2, `vector_geometry.rs` 4), presentation 5 (`images.rs` 4, `theme.rs`), surfaces 1 (`tool_window_modes.rs`) |
  | `pert-ui-cpu` | `src/lib.rs:106` (rustdoc), `tests/oracle.rs:45`, `:556`, `examples/frame_cost.rs:72` |
  | `pert-ui-wgpu` (all `#[cfg(test)]`) | `lib.rs:1077`, `:1122`, `:1171`; `cpu_parity.rs` 4; `native_smoke.rs` 13 (native checks); `realtime_probe.rs:210`; `retained.rs:117`, `:164` |
  | gallery | `main.rs:228`, `:235`; `interaction_check.rs:1108`, `:1172`; `workspace_check.rs:196`, `:257`; `tests/text_rendering.rs` 4 |

  The gallery's `Gallery::mount` (`main.rs:344`, `screenshot.rs:100`,
  `tests/support.rs:10` and others) is the gallery's own method and is not
  affected. No guide chapter, README or packaged example calls
  `Runtime::mount`.
- **What goes with it.** `mount_description`'s `composed` parameter
  (`runtime/tree.rs:1164-1168`) is always true afterwards. The parameter,
  the node and entry `composed` fields (`runtime.rs:855`, `:3631`) and
  their non-composed branches (`runtime/tree.rs:506`, `:747-748`, `:1222`,
  `:1269`, `:1547`, `:1564`, `:1650`, `:1975`; `runtime.rs:981`, `:6206`;
  `runtime/inspect.rs:186`) are removed, keeping each composed arm. The
  mentions of legacy roots are removed from `Component::paint`
  (`runtime.rs:2178-2179`), `UpdateContext::size` (`:3236-3237`), `EventContext::bounds` (`:3407`), `Runtime::bounds`
  (`runtime/tree.rs:402`), `reconcile_children` (`:479`), `reparent`
  (`:722`) and `Painter::size` (`drawing.rs:1694`). L1's index drops
  `mount` from "Driving" (`runtime.rs:3756`).
- **After N6 every painted component has an allocation.** The audit:
  - the only `Painter` is built by the runtime's compose step
    (`runtime/compose.rs:114`; its fields are `pub(crate)`,
    `drawing.rs:1394-1416`, and it has no public constructor), with
    `size: entry.node.size.get()` (`compose.rs:129`);
  - it is built only for entries in the composition order, which holds
    visible entries of the selected window (`runtime/tree.rs:1388-1396`,
    `Node::is_presented`, `runtime.rs:902-906`);
  - `paint` refuses with `DrawError::PendingUpdates` while updates or
    layout are pending (`runtime/compose.rs:47-60`), so the selected
    window's layout has run since the last change;
  - that layout arranges every composed root: the viewport supplies its
    initial rectangle (`runtime/tree.rs:1968-1993`), and the final
    allocation honors the root's size limits (`runtime/tree.rs:1766-1770`),
    so a positive minimum can give a non-empty allocation with an empty
    clip; either way the entry gets a size. A container that leaves a
    visible child without a placement fails the layout with
    `LayoutError::InvalidChild` (`runtime/tree.rs:1848-1856`);
  - a drag preview is arranged at its preferred size
    (`runtime/tree.rs:1978-1990`).

  The only `None` was the legacy root, which never takes part in layout.
  Recommended ([decision D3](#decisions-for-the-maintainer)):
  **`Painter::size` returns `Size`.** The compose step reads the entry's
  size and, for an entry without one, which the audit shows unreachable,
  refuses the paint with `DrawError::PendingUpdates` (the documented
  refusal for work layout has not finished) instead of painting with a
  guessed size; no `unwrap` is added. Every caller drops its `None`
  branch: 70 calls, in the library's widgets, the guide's two custom
  components (`guide/getting-started-continued.md:134`,
  `guide/appearance.md:265`), the diagram editor's canvas
  (`examples/diagram_editor/canvas.rs:772`, `:911`), the gallery, the
  facade's SVG component and tests. The rustdoc states that the size is
  the component's allocation in local logical pixels, which may be
  empty.
- **`TestError::NotLaidOut` stays,** for a different reason: `Driver::click`
  reveals first, and the reveal flushes (`runtime/reveal.rs:555`), so after
  N6 a target that passed the reveal has bounds by the same audit. The
  variant is kept as the driver's typed report should that invariant break,
  rather than a panic, because `TestError` is `#[non_exhaustive]`
  (`testing.rs:687-689`) and no consumer matches it; its rustdoc
  (`testing.rs:715-721`) and the testing chapter's sentence
  (`guide/testing.md:61`) say exactly that instead of "never been laid
  out". `Painter::size` is retyped and this variant is not because every
  custom widget reads the size on every paint, while the variant costs a
  consumer nothing.

## 3. N7: lookups and `Each`'s address

### `find_path` leaves the public surface

- **The rule.** `Runtime::find_path` (`runtime/tree.rs:151-203`) is removed.
  No crate-private form is kept: no library code calls it (its callers are
  tests and the gallery), so a private copy would be dead code.
- **Users and their migration:**
  - gallery, application code: `multi_window.rs:205` becomes
    `ui.find(&root, &[&strip, "panel"])`: a tab's content is addressed by
    its tab key below the strip, through the `Tabs` viewport part
    (`tabs.rs:145`). `tests/data.rs:503` becomes `find(&outline, &[key])`.
    Where `find` reaches a different widget, the gallery test uses its own
    key-walking helper instead (below), and the review record says so.
  - facade integration tests (50 calls: collections 3, foundation 42,
    surfaces 5, including `menus.rs` 4 and `views.rs` 1): the tests that
    address an `Each` item (`foundation/described.rs`,
    `foundation/failure_sites.rs:207`) move to `find` with the new address
    (below); the others check a composite's internal parts, which is what
    `find_path` was for, and use a new helper in `tests/support/mod.rs`,
    `key_path(ui, root, &[...]) -> Option<WidgetId>`, built from the public
    `Runtime::children` and `Runtime::key`. It keeps their intent and
    shows that the removed method added nothing a test cannot write.
  - runtime integration tests: `input_contracts.rs` (14) gets the same
    helper locally.
- **Documentation.** `find`'s and `find_descendant`'s rustdoc stop pointing
  to `find_path` (`runtime/tree.rs:208-217`, `:300-301`); the full-key walk
  is described as `children` and `key`. L1's index drops it from "Lookups"
  (`runtime.rs:3743-3745`).

### `Each`'s default container adds no segment

- **The rule.** The default container becomes internal structure:
  `Column::new(children).key("items").structural()` at `described.rs:339`.
  `Element::structural` exists for exactly this (`layout.rs:690-708`: "Use
  it for wrappers, viewports and layout scaffolding a container creates for
  itself"), and `Menu` already keys its own list `items` and marks it
  structural (`menu.rs:1117`).
- **The resulting addresses.** An item's address is the `Each` element's
  address followed by the item's key: `find(&each, &["ship"])`,
  `Selector::path([.., "tasks", "ship"])`. The container keeps its key
  `items`, so `find_descendant(&each, "items")`, `Selector::Key("items")`,
  `Runtime::children` and inspection's `key` field still reach it, but it has
  no address of its own (`ComponentInspection::path` is `None`, as for every
  structural component, `diagnostics.rs:772-777`) and `find` never returns
  it (`runtime/tree.rs:375`).
- **An application's container keeps its segment.** `.within(f)` uses the
  element `f` returns as it is: never marked structural, whatever its key,
  including `items`. So `.within(|rows| Column::new(rows).key("items"))`
  addresses items at `items/<key>` as today. An application that wants the
  default addressing with another layout marks its own container with
  `Element::structural`. `within`'s rustdoc ("The container's key is part of
  the items' path", `described.rs:346-350`) stays true and gains that
  sentence.
- **Addressing is fixed when the container mounts.** Like every element's
  (`layout.rs:704`: "The marking is fixed at mount; reconciliation does
  not change it"), and because reconciliation keeps an existing child
  under the same key without adopting the new description's addressing
  (`runtime/tree.rs:1098-1105`), the container keeps the addressing it
  mounted with when the `Each` is described again with a different
  container under the same key. This part keeps that rule rather than
  remounting on a change of addressing, so both directions are settled:
  - default, then `.within(|c| Column::new(c).key("items"))`: the
    retained container stays structural; items keep their `WidgetId`s,
    state and addresses (`tasks/<key>`), and no `items` segment appears;
  - `.within(|c| Column::new(c).key("items"))`, then default: the retained
    container keeps its `items` segment; items keep their `WidgetId`s,
    state and addresses (`tasks/items/<key>`);
  - a container of another type under the same key, such as
    `.within(|c| Row::new(c).key("items"))` after the default `Column`,
    fails the flush with `RuntimeError::ChildTypeChanged`
    (`runtime/tree.rs:1077-1081`), as any same-key type change does today.

  **Replacement-key usage:** to change the addressing, describe the new
  container under another key (for example `rows`). The old container
  unmounts with its items and the new one mounts with them, so the items
  get new `WidgetId`s and lose their state, and their addresses become
  `tasks/rows/<key>` (or `tasks/<key>` for a structural container).
  `Each`'s and `within`'s rustdoc say: "The container's addressing is
  decided when it first mounts; to change it, give the new container
  another key, which mounts the items again." contracts.md's paragraph on
  children from data says the same.
- **Structural, not a part.** `Element::part("items")` would keep an address
  for the container alone (`layout.rs:722-735`), but an item keyed `items`
  would then share it: `find` would fail with `AmbiguousKey`
  (`runtime/tree.rs:378-381`), and a workspace participant at that address
  would make capture fail with `DuplicatePath`
  (`runtime/workspace.rs:221`). Structural leaves no key reserved.
- **What follows the rule.** Every address-based contract computes the path
  in one place, `workspace_segments` (`runtime/workspace.rs:251-282`), or
  walks the same addressing (`find`, `runtime/tree.rs:347-397`), so all
  change together:
  - `Runtime::find`;
  - workspace capture, restore and pending entries
    (`runtime/workspace.rs:197-227`, `:340-360`);
  - `ComponentInspection::path` (`runtime/inspect.rs:194`);
  - `Selector::Path`, `Selector::Children` and `Selector::Descendants`
    (`runtime/inspect.rs:369-395`);
  - `CaptureTarget::Path`, which capture admission resolves
    (`runtime/capture.rs:94-100`, `:224-229`);
  - content-search hit paths (`runtime/content_search.rs:310-319`).

  Unchanged: `find_descendant`, `Selector::Key`, `Selector::Id`,
  `children`, `key`, `WidgetId` identity, focus and state retention by key.
- **Naming through the container.** A structural level is transparent to
  naming as well as to addresses, and this part applies that rule to the
  default container rather than keeping the old boundary (which would
  need a fourth addressing kind, "structural for addresses, ordinary for
  names"). Three mechanisms walk structural levels:
  - an element label reaches a structural component that reports no name,
    from the nearest non-structural ancestor that reports no semantics
    (`runtime/inspect.rs:112-136`);
  - a naming parent (`Semantics::names_children`, such as a `Field`) names
    a component whose chain up to it consists of structural levels with no
    semantics (`runtime/inspect.rs:456-503`; the climb continues while the
    level below is structural, `:497-500`);
  - content search names a hit by the same element label, then by its
    naming parent while its ancestors are structural
    (`runtime/content_search.rs:265-281`, `:322-335`).

  The walks differ in what they test. The element-label walk starts only
  at a structural component and climbs through structural ancestors; the
  inspection naming walk climbs while the level *below* is structural
  (`runtime/inspect.rs:497-500`), so it starts only at a structural
  component too; content search's naming-parent fallback tests the
  *ancestors'* addressing (`runtime/content_search.rs:323-334`), so it
  starts at any component. The consequences, exactly:
  - **An `Each` that is not itself structural** (the default, and every
    `Each` a maintained consumer builds):
    - an item element that the application marks `Element::structural`
      now reaches the `Each` element's label
      (`Each::new(...).with_label("Batch")`; `Each` reports no semantics)
      and a naming parent directly above the `Each`, so inspection's
      `name` and `labelled_by`, `Selector::Semantic` queries by name, and
      content-search name hits (through the element label) change for it;
    - an ordinary item is unchanged in all three: the label and inspection
      walks do not start at it, and content search's fallback stops at the
      `Each`, which is not structural.
  - **An `Each` that the application marks `Element::structural`:** the
    content-search fallback of an **ordinary** item now climbs through the
    default container and the `Each` to a naming parent above them, so
    `Field -> structural Each -> default container -> unnamed TextField`
    makes the text field a content-search name hit for the field's name;
    before, the ordinary container stopped it. Inspection and
    `Selector::Semantic` do not change for that ordinary item (the
    inspection walk does not start at it), so content search and
    inspection already differ there, as they do today for any ordinary
    component below a chain of structural wrappers. A structural item
    under a structural `Each` changes in all three, as above.

  The composites' own structural parts inside an item stop at the item, as
  before. No maintained consumer marks an `Each` or an item structural (the
  workspace's `Element::structural` calls outside the library are the
  gallery's shell wrappers, `gallery.rs:400`, `search.rs:131`, and test
  fixtures that are not `Each`s). `Each`'s rustdoc and contracts.md's
  paragraph on children from data state the rule: the default container is
  internal structure, transparent to addresses and naming like any
  structural wrapper.
- **No maintained consumer stores such a path.** The only `Each` in the
  gallery uses `within` (`pages/chips.rs:69-73`); no workspace participant
  sits in a default `Each` in the gallery or the examples.

### Old workspace documents

A version-1 document may hold a path through a default `Each` container,
such as `list/items/7/scroll`. After the change the same text would address
an item keyed `items`, if one exists: old data would be reinterpreted, which
AGENTS.md forbids for persisted schemas. Recommended
([decision D2](#decisions-for-the-maintainer)): **versioned rejection.**

- `Workspace::SCHEMA_VERSION` becomes 2 (`workspace.rs:515`). Decoding
  already rejects every revision other than the supported one, so a
  version-1 document is rejected before a `Workspace` is built, and so
  before anything is restored or applied:
  - the text form checks the header first and returns
    `WorkspaceError::UnsupportedVersion { found: 1, supported: 2 }`
    (`workspace.rs:925-944`);
  - the serde form deserializes the whole representation and then checks
    the version, reporting through `D::Error::custom`
    (`workspace.rs:1724-1731`): a version-1 document fails with that
    variant's message, and one that is also malformed may fail on the
    malformed part first. Either way no `Workspace` is constructed. The
    decoder is not rewritten.
- The format is otherwise unchanged; there is no reader for revision 1.
- `SCHEMA_VERSION`'s rustdoc says "a document from another revision, older
  or newer" instead of "a newer revision" (`workspace.rs:509-514`), and
  contracts.md's "Workspace persistence" (`contracts.md:1245-1256`) states
  that revision 2 changed how `Each`'s default container addresses its
  items and that a revision-1 document is rejected; the host falls back to
  its default workspace as for any unsupported document.
- The doc examples that print or expect revision 1
  (`runtime/workspace.rs:61`, `workspace.rs:483`, `:917`) and the tests'
  literal documents (`workspace.rs` unit tests at `:1339-1552`; facade
  `surfaces/workspace.rs`, `tab_notifications.rs`, `tab_selection.rs`,
  `foundation/serialization.rs`) move to revision 2. The tests that use
  `pert-ui-workspace 2` as the unsupported revision (`workspace.rs:1339`,
  `surfaces/workspace.rs:810-813`) move to 3.
- The gallery's two-process restart check saves and restores in one run of
  one build (`workspace_check.rs:167-240`), so it is unaffected; a document
  a developer saved earlier with `--save` is refused and saved again.

### The inspection schema

`InspectionSnapshot::SCHEMA_VERSION` stays 15. Its rule is that it changes
"when a field is added, removed or reinterpreted" (`diagnostics.rs:996-1004`).
No field changes, and `ComponentInspection::path` keeps its documented
meaning, "application keys and container slots ... without
library-internal structure" (`diagnostics.rs:772-777`); the default
container now is library-internal structure, so the change brings `Each`
into line with that meaning. What changes is the tree a given program
produces, as when any composite restructures. Snapshots are owned
diagnostics that a reader decodes, not documents applied back to a tree;
`Selector` and `CaptureTarget` values a tool keeps are transport forms
outside any versioned schema, as the CHANGELOG already records for serde
forms (0.3.55, `CHANGELOG.md:956-961`). The CHANGELOG's "Changed" entry
names the path consequence.

## 4. Documentation and records

- **contracts.md:** "Custom hosts" (`contracts.md:1765-1785`) opens with the
  `host` module and its trait, and the `testing` re-export; its method
  names become links to the trait. "Workspace persistence" gains the
  revision-2 sentence. The paragraph on children from data
  (`contracts.md:249-251`) adds that the default `Column` adds no segment to
  an item's address and that a `within` container's key does. Mentions of
  moved methods (`:548-552`, `:584`, `:626`, `:816`, `:1042`, `:1058`, `:1086`,
  `:1116`, `:1127`) name the trait where they name the method; the
  mentions of `command_key_input` (`:955-959`, `:1213`) stay.
- **Rustdoc:** the trait and both `host` modules (section 1); `Runtime`'s
  index and overview; `Each`'s type docs: the paragraph and first example at
  `described.rs:274-291` show `find(&root, &["ship"])` and that
  `["task"]` (the element's own key) does not resolve; `Each::new` says
  the default `Column` is internal structure, transparent to addresses and
  naming; `Each` and `within` state the mount-time addressing and the
  replacement-key usage (section 3); `find`,
  `find_descendant` and the legacy-root mentions (section 2).
- **Guide:**
  - `testing.md`'s "List rows" (`:201-235`): the row is
    `driver.find(&root, &["ship"])`, the query is
    `Selector::children(["tasks"])`, and for the rows in display order the
    example names its container with `within(|rows| Column::new(rows).key("rows"))`
    and lists `children` of `find(&root, &["rows"])`; the paragraph
    explains both.
  - `layout.md:417`: `["tasks", "Ship it"]`.
  - `testing.md:60-61`: `NotLaidOut` described as the driver's report of
    a broken layout invariant (section 2), not "never laid out".
  - `controls.md:452` and the runtime README's copy (`README.md:945`):
    the example's `ui.clear_focus()` becomes `ui.focus(&fields[0])?`, which
    also moves focus out of the amount field and shows the same rule
    without a host call.
  - Links to moved methods become trait links: `layout.md:213`,
    `state.md:379`, `custom-widgets.md:180`, `controls.md:391`,
    `diagnostics.md:147`, and the plain mentions in `appearance.md:126-128`,
    `diagnostics.md:67`.
  - The facade's root page (`src/root.md`) gains a "Custom hosts" row
    linking `host`.
- **Other docs:** `docs/manual.md:65` (`take_shutdown_errors`) and
  `docs/architecture.md:22-23` ("A custom host can use the runtime" links
  the `host` module).
- **CHANGELOG:** see [scope](#7-scope-versions-and-evidence).
- **Records:** the layering plan's progress gains L3's entry, with the
  answer to the inventory's question 3; the inventory is not edited (its
  counts are the 6219dda measurement; the plan re-measures after the pass).
- **Not migrated (frozen):** the round 1 submissions, probes and triage
  under `plans/usability/runs/`, the tester kits and prompts under
  `~/pert-ui-usability/`, `plans/archive/`, the API audit's generated inputs
  under `plans/usability/api-audit/inputs/`, and earlier CHANGELOG entries.

## 5. Migration manifest

| Consumer | N3 | N6 | N7 |
|---|---|---|---|
| `pert-ui-runtime` sources | new `host.rs`; `lib.rs` module; inherent methods `pub(crate)`; `testing` re-export and `testing.rs:20-21`; rustdoc, including the ten examples of section 1's table | `mount` and the `composed` flag removed; rustdoc examples; `Painter::size` and the compose step (D3); `TestError::NotLaidOut`'s rustdoc | `described.rs:339`; `SCHEMA_VERSION`; `find_path` removed; rustdoc |
| `pert-ui-runtime` tests | trait via `testing` or `host` in the files of the table | 65 calls in 8 files, by the default rule except section 2's exceptions 1, 3, 4 and 7 (`debug_overlay.rs` and `validation.rs` through a `Place` container, `diagnostics.rs:53`, the deleted `testing.rs:358-395` case); unit tests in `parameter_binding.rs`, `shapes.rs` | `input_contracts.rs` helper; `workspace.rs` unit tests |
| facade sources | `host` module; `testing` re-export; imports in `app/close.rs`, `app/headless.rs`, `platform/{mod, windows, schedule, ime, motion}.rs` and the native checks `platform/*_smoke.rs` | `app.rs:293`; `platform/mod.rs`, `schedule.rs`, `smoke.rs` | — |
| facade integration tests | imports in the files of the table | 53 calls | `key_path` helper; `described.rs`, `failure_sites.rs`; workspace documents |
| `pert-ui-cpu` | `src/lib.rs` imports the trait; `tests/oracle.rs` | rustdoc, `tests/oracle.rs`, `examples/frame_cost.rs` | — |
| `pert-ui-wgpu` | none | 23 calls in test modules, 13 of them native checks | — |
| packaged examples | `diagram_editor/session.rs`, `inventory/session.rs`, `log_explorer/session.rs` import `testing::HostIntegration` | — | — |
| guide, READMEs, docs | links and the `clear_focus` example (section 4) | `testing.md:61` (`NotLaidOut`); with D3, `getting-started-continued.md:134`, `appearance.md:265` | `testing.md`, `layout.md`; contracts.md |
| gallery | imports in `main.rs`, `live.rs`, `interaction_check.rs`, `workspace_check.rs`, `tests/*` | 10 calls (section 2) | `multi_window.rs:205`, `tests/data.rs:503` |
| native checks (`apps/native-checks`) | none | none | none |
| diagnostics fixtures | `k01_clipboard_not_impl.rs` imports `pert_ui::host::HostIntegration`, so it still produces the `Clipboard` diagnostic; its snapshot's "required by a bound in" path changes; new `h01_hostintegration_impl` | — | — |
| `plans/usability/task4-app` | none (calls `mount_tree`, `signal`, `register_command` only) | none | none: its README already describes rows directly under `list`, which becomes exact |
| L2's representative programs and tests (as L2's draft checkpoint names them: the getting-started program, `controls.md`'s settings, `layout.md`'s list recipe, `surfaces.md`'s dialogs and menu bar, `state.md`'s background work, `task4-app`, the testing chapter) | none calls one of the 33 at 2a406a4; were one to, its test reaches the trait through `testing`, with no new import | none calls `mount`; with D3, the getting-started indicator, one of L2's representative custom components, drops its `None` branch (`guide/getting-started-continued.md:134`), with no import change | only the testing chapter's `Each` paths change (`testing.md:222-232`), with no import change |

## 6. Tests

As in the fix plan's checkpoints, tests are of two kinds and each is marked
as one in the code:

- **New-behavior tests** cover what only the new rules do. Each names the
  assertion that detects the old implementation and is checked with a
  compiling mutation that restores it; the named assertion must fail.
- **Contract and compatibility tests** check rules this part keeps. Where
  one guards a contract the new code could break on its own, it is checked
  with a compiling mutation that violates that contract.

### N3 new-behavior tests

1. **Application code cannot call a host method** (`compile_fail` doctests
   on the facade's `host` page): with `use pert_ui::prelude::*;` alone,
   `ui.flush_windows(Duration::ZERO)` does not compile; the same with
   `use pert_ui::*;`. Mutation: keep `flush_windows` as a `pub` inherent
   method; both snippets compile and the doctests fail.
2. **Tests reach it through `testing`** (ordinary doctest on `testing`'s
   page): with `prelude::*` and `testing::*`, `ui.select_window(&main)?`,
   `ui.take_presentation_requests()?` and `ui.clear_focus()` compile and run.
   Mutation: drop the `testing` re-export; the doctest fails to compile.
3. **The trait is sealed** (diagnostics fixture `h01_hostintegration_impl`,
   plus a `compile_fail` doctest on the trait that documents the
   restriction): `impl HostIntegration for Mine {}` reports the seal's
   message. Mutation: remove the supertrait; the fixture then reports only
   rustc's missing-items error and its snapshot fails.
4. **The host vocabulary is complete:** a doctest on the facade's `host`
   page imports only `pert_ui::host::*` (no prelude) and builds every name
   the audit lists for event translation and frame production: a
   `Viewport` from a `PhysicalExtent`, a `PointerEvent` of each
   `PointerKind` with a `PointerButton`, `Point` and `Magnification`, a
   `CommandKeyInput`, a `TextPreedit`, a `CaptureError::Limit(LimitError)`,
   and a match on `DrawError` and `PresentationAction`. Mutation: drop any
   one of the added re-exports (`PointerEvent`, say); the doctest fails to
   compile.

### N7 new-behavior tests

In the facade's `foundation` binary unless stated. Mutation for 1 to 5:
remove `.structural()` from `described.rs:339`.

1. **Lookup:** `find(&each, &["ship"])` returns the item; named assertion
   also `find(&each, &["items", "ship"]) == None`.
2. **Inspection and selectors:** the item's `ComponentInspection::path` is
   `[.., "tasks", "ship"]`, the container's is `None`;
   `Selector::children([.., "tasks"])` returns exactly the items and
   `Selector::path([.., "tasks", "items"])` nothing.
3. **Workspace** (`surfaces` binary): a `ScrollArea` inside each item of a
   default `Each` is captured at `tasks/<key>/scroll`; restoring that
   document scrolls the same item, also when the item mounts after the
   restore (pending entry).
4. **An item keyed `items`:** in a default `Each` whose items include the
   key `items`, named assertion `find(&each, &["items"])` equals that item
   (taken from `children` of the container), and a workspace participant
   inside it captures at `tasks/items/<participant>`. Mutation: the old
   behavior returns the container. Contract mutation: `.part("items")`
   instead of `.structural()` fails with `AmbiguousKey`, which records why
   the container is structural.
5. **Capture** (`foundation/capture.rs`): with capture support on, a
   request for `CaptureTarget::Path([.., "tasks", "ship"])` is admitted;
   admission resolves the path (`runtime/capture.rs:94-100`). Named
   assertion: `pending_captures()` lists exactly one job, and the reply is
   still pending (the old address answers at once with
   `CaptureError::UnknownTarget` and lists no job). Then
   `capture_scene(&job)` composes the whole window (`:314-336`), a frame of
   the window's extent completes the job (`complete_capture`), and the
   reply's `component` is `Some(ui.inspection_id(&ship))` and its `region`
   is the item's bounds: the crop happens at completion.
6. **Version-1 workspace documents are rejected** before any `Workspace`
   is built or applied. Text form (`surfaces`): `Workspace::decode` of
   `pert-ui-workspace 1\n` plus a valid entry returns
   `Err(WorkspaceError::UnsupportedVersion { found: 1, supported: 2 })`.
   Serde form (`foundation/serialization.rs`): `serde_json::from_str::<Workspace>`
   of `{"schema_version":1,...}` with otherwise valid settings and
   entries fails with an error whose message is that variant's `Display`
   ("workspace schema version 1 is not supported (this build writes 2)"),
   since serde reports through `D::Error::custom` (`workspace.rs:1724-1731`)
   rather than a typed `WorkspaceError`. Neither assertion depends on
   whether the version is checked before or after the entries are read.
   Mutation: leave `SCHEMA_VERSION` at 1; both named assertions fail.
7. **Redescription, default then explicit:** a default `Each` is described
   again, same key, with `.within(|c| Column::new(c).key("items"))`. Named
   assertions: `find(&each, &["ship"])` returns the same `WidgetId` as
   before, `find(&each, &["items", "ship"])` is `None`, and a participant in
   the item is captured at `tasks/ship/<participant>`. Mutation as for 1-5.
8. **Naming through the container:** an item element marked
   `Element::structural` (an unnamed `TextField`) in
   `Each::new(...).with_label("Batch")`. Named assertions: its inspected
   `name` is `"Batch"`; `Selector::Semantic { role: TextInput, name: "Batch" }`
   finds it; a content search for "Batch" returns it as a name hit. With a
   `Field` directly above the `Each` (no label on the `Each`), the item's
   name is the field's and `labelled_by` is the field. Mutation as for 1-5:
   the walks stop at the ordinary column and the name is absent.
9. **Content search through a structural `Each`** (`collections` binary,
   beside the content-search tests): `Field` around an `Each` marked
   `Element::structural` whose default container holds an ordinary,
   unnamed `TextField`. Named assertion: a content search for the field's
   name returns the text field as a name hit. Separately, inspection of
   the same text field reports no `name` and `Selector::Semantic` by the
   field's name does not match it (the inspection walk does not start at
   an ordinary component). Mutation as for 1-5: the search returns no hit;
   the inspection assertions hold either way and are recorded as contract
   checks.

### N6 and `find_path` tests

No behavior is added, so there are no new-behavior tests. The legacy
`NotLaidOut` case in `tests/testing.rs:358-395` (a legacy root and the
assertion at `:388-395`) is deleted, not replaced: after N6 no target that
passes the reveal lacks bounds (section 2), so no test can reach the
variant through the public API.

`Painter::size` (if D3 is approved as recommended) is checked by the
type: every caller compiles against `Size`. The compose step's refusal for
an entry without a size is unreachable by the audit and has no test; a
`debug_assert!` beside it records the invariant.

### Contract and compatibility tests

- **The custom-host example** (moved here from the new-behavior list: it
  preserves behavior and has no behavioral mutation of its own) asserts
  the refused detach's receipt, the key's and the click's effects, a painted
  frame, the cursor read and an empty `take_shutdown_errors`.
- **Redescription, explicit then default:** an `Each` mounted with
  `.within(|c| Column::new(c).key("items"))` and described again with the
  default container keeps `tasks/items/<key>` addresses, the same
  `WidgetId`s and the participant's workspace path. Contract mutation:
  reconciliation adopts the new description's addressing; the `items`
  lookup fails.
- **Replacement key:** `.within(|c| Column::new(c).key("rows"))` after the
  default gives `tasks/rows/<key>`, new `WidgetId`s (the old ones report
  `Unmounted`) and fresh item state; a same-key `Row` fails the flush with
  `ChildTypeChanged` and keeps the previous items.
- **Ordinary items keep their naming under an ordinary `Each`:** a
  non-structural item under a labelled `Each`, and under a `Field` above
  the `Each`, has the name it had before (none from either) in inspection,
  `Selector::Semantic` and content search; the content search for the
  label or the field's name returns no hit for it. Contract mutation: let
  content search's fallback climb past a non-structural ancestor; the
  content-search assertion fails.

- **`within` keeps the application's segment:** `.within(|c| Column::new(c).key("items"))`
  addresses an item at `items/<key>`, and `.within(|c| Row::new(c).key("row"))`
  at `row/<key>` (today's `foundation/described.rs:279-290`, migrated to
  `find`). Contract mutation: apply `.structural()` to whatever `within`
  returns; both fail.
- **Retention and ordering:** an item that keeps its key keeps its
  component and state across a reorder and a removal of another item, and a
  duplicate key keeps the previous items (`foundation/described.rs:215-266`,
  `failure_sites.rs:198-215`, migrated to the new addresses).
- **Workspace revision 2:** text and serde round trips and the
  revision-3 rejection (the migrated tests above); a document whose entries
  address paths outside any `Each` restores as before.
- **Migrated tests:** every test that used `Runtime::mount`, `find_path` or
  a moved method keeps its assertions; the review record lists any that had
  to change for a reason in section 2.
- **Doctests:** `Each`'s two examples, `find`, `find_descendant`, the
  migrated `App`, `drawing`, `geometry` and CPU examples, the guide's
  "List rows", "children from data" and number-input examples, the
  `testing` module example and both `host` pages.
- **Diagnostics snapshots:** `k01` keeps its `Clipboard` message; `h01` is
  new; every other snapshot is unchanged.

## 7. Scope, versions and evidence

- **`api/` diff:**
  - added: `pert_ui_runtime::host` with `HostIntegration` (33 methods and
    its impl for `Runtime`) and its re-exports; the facade's `pert_ui::host`;
    `HostIntegration` in both `testing` modules;
  - removed: the 33 inherent `Runtime` methods, `Runtime::mount` and
    `Runtime::find_path`;
  - changed: `Painter::size` returns `Size` (D3);
  - in `api/pert-ui-runtime/{default,serde}.txt` and every facade listing;
    `api/pert-ui-cpu` and `api/pert-ui-wgpu` unchanged (their signatures do
    not change). `Workspace::SCHEMA_VERSION`'s value is not part of the
    listing; the CHANGELOG states it.
- **Breaking:** yes.
- **CHANGELOG** for 0.3.74, citing this checkpoint:
  - `Breaking:` the host-integration methods move from `Runtime` to the
    sealed trait `pert_ui::host::HostIntegration`, which `Runtime`
    implements: the 33 names. Replacement: a custom host adds
    `use pert_ui::host::HostIntegration;` (`pert_ui_runtime::host` for
    direct users of the runtime crate); a test that globs `pert_ui::testing`
    needs nothing; applications using `App` are unaffected.
  - `Breaking:` `Runtime::mount` is removed; use
    `ui.mount_tree(Element::new(key, component))`. A root mounted so is laid
    out into the whole window; an overlay probe returns false from
    `Component::hit_test` and reports `OverlapPolicy::Allow`.
  - `Breaking:` `Painter::size` returns `Size` (if D3 is approved as
    recommended): drop the `None` branch, `let size = painter.size();`.
  - `Breaking:` `Runtime::find_path` is removed; use `Runtime::find` with the
    addressable path, or walk `Runtime::children` and `Runtime::key`.
  - `Breaking:` `Each`'s default container no longer adds `items` to its
    items' addresses: `find(&each, &["items", key])` becomes
    `find(&each, &[key])`, and the same for `Selector` paths,
    `CaptureTarget::Path`, workspace paths and content-search hit paths. A
    container given with `within` keeps its key in the address. A
    container's addressing is decided when it first mounts; to change it,
    give the new container another key. An item the application marks
    `Element::structural` now takes its name from the `Each`'s label or a
    naming parent above the `Each`.
  - `Breaking:` `Workspace::SCHEMA_VERSION` is 2; a revision-1 document is
    rejected: `Workspace::decode` returns
    `WorkspaceError::UnsupportedVersion { found: 1, supported: 2 }`, and
    serde deserialization fails with that error's message. Capture the
    workspace again.
  - `Changed:` `ComponentInspection::path` of an `Each` item omits the
    default container; `InspectionSnapshot::SCHEMA_VERSION` stays 15, since
    no field is added, removed or reinterpreted.
  - `Added:` the `host` modules and their custom-host example; the
    diagnostics fixture for the sealed trait.
  - `Documentation:` `Runtime`'s audience index, contracts.md's custom hosts,
    workspace persistence and children from data, the guide and the
    migrated examples.
- **Version:** 0.3.74 (workspace version, internal requirements and
  `Cargo.lock`), after L2's 0.3.73.
- **Gate,** as the plan states for L3:
  - the public-API row: the workspace checks (formatting, Clippy, tests, the
    packaged examples' headless sessions, doctests, warning-free rustdoc,
    the diagnostics snapshots with `k01` and `h01` reviewed), the API
    listing gate, the focused tests above with their mutations recorded,
    and Codex's implementation review;
  - for N3, the dependency/feature/platform row: MSRV (the pinned
    toolchain), release-profile tests, every CI feature configuration
    (`--no-default-features`, `image-formats`, `png`, `images`, `svg` with
    and without defaults, `serde` for the facade and `pert-ui-core`, `cpu`,
    `desktop-cpu`), the headless gallery, library packaging, and the
    packaged-consumer check compiling the custom-host example and a custom
    widget from extracted packages;
  - the native checks (`cargo run --locked -p pert-ui-native-checks`) on an
    unlocked desktop session, because the native smokes, the gallery's
    interaction and restart probes and the wgpu checks change; the record
    names the platform. Windows and macOS evidence is recorded as missing
    unless run. The Loom models are not required: the live-data protocols
    in `pert-ui-core` do not change.
- **TODO.md:** NATIVE-04 (`interaction_check.rs`, `workspace_check.rs`)
  concerns the probes this part migrates; it is not resolved by it, and its
  native run is part of this gate's evidence. No new entry, unless D3's
  alternative is chosen (then one for `Painter::size`).

## Corrections to the plan and the inventory

- **`paint` is not a host-only method.** The plan lists "`paint` for hosts"
  among the moves. It is the headless drawing result that the guide's
  examples read throughout (about twenty calls, most with the prelude
  alone), and L1's review fix classified it with the test operations
  (`runtime.rs:3757-3761`). It stays, so the
  plan's "renderer checks call `paint`" needs no migration in
  `pert-ui-wgpu`, which calls none of the 33.
- **The "Hosts" inventory is 33 methods,** not the inventory's "about 47"
  (`complexity-inventory.md:61`, `:119`, measured by first rustdoc line,
  which also counted `paint`, `mount` and other operations L1 placed with
  applications and tests) and not the 40 of L1's first index: the review
  fix moved seven test operations out.
- **`find_path` has no internal callers.** The plan expected "internal
  callers keep a crate-private form"; no library code calls it. Its users
  are 64 test calls and two in the gallery (one of them in a test), so no
  private form is kept.
- **`Runtime::mount` has more users than the plan names** ("gallery
  self-checks, facade tests and probes"): also the runtime's tests (65), the
  wgpu checks (23), the CPU crate, three runtime rustdoc examples and
  `App`'s first example, which teaches it.
- **Version:** the plan's heading names 0.3.73 for L3; with L4a landed as
  0.3.72 and L2 next as 0.3.73, L3 is 0.3.74.
- **Citation drift:** `Each`'s default container is at `described.rs:339`
  (cited as `:333`), the selector match at `runtime/inspect.rs:389` (cited
  as `:379`), and `serve_captures` at `pert-ui-cpu/src/lib.rs:264`
  (cited as `:266`).

## Decisions for the maintainer

- **D1: the trait's name.**
  - **Recommended:** `HostIntegration`, the term L1's documentation already
    uses for this audience's callbacks ("host-integration callbacks"), read
    as "`Runtime`'s host integration": `use pert_ui::host::HostIntegration;`.
  - **Alternative A:** `RuntimeHost`. Shorter, but reads as if the runtime
    were the host, which is the opposite of the relation.
  - **Alternative B:** `Host`. Shortest, but `host::Host` stutters, and
    "host" alone already names parameter hosts (`ParameterHost`,
    `RemoteHost`) and `ToastHost`.
- **D2: old workspace documents.**
  - **Recommended:** versioned rejection: revision 2, revision-1 documents
    fail with `UnsupportedVersion` (section 3). No library version is
    published, no maintained consumer stores a path through a default
    `Each`, and the decoder already rejects every other revision.
  - **Alternative:** migration at restore. A decoded `Workspace` cannot be
    migrated on its own, because only the mounted tree knows which segment
    was a default container; the runtime would keep the revision-1 address
    of every mounted component and match revision-1 entries against it at
    restore and for pending entries. `Workspace` would have to carry its
    revision so that `encode` never writes revision-1 paths under a
    revision-2 header. It keeps users' saved layouts, at the cost of a
    second addressing rule kept indefinitely.
- **D3: `Painter::size`.** Changed in revision 2: the reachability audit
  (section 2) shows that N6 removes the only `None`.
  - **Recommended:** return `Size` in L3, as N6's direct consequence. Every
    custom widget, including the guide's representative components, loses
    a branch that can no longer run; the compose step refuses an entry
    without a size with `DrawError::PendingUpdates`. It widens this part's
    break by one signature and 70 mechanical call-site edits, with no
    import change.
  - **Alternative:** keep `Option<Size>` as a deliberate deferral of a
    verified simplification: its rustdoc says `None` is not produced by a
    runtime paint, and TODO.md records the retyping for a later part. This
    keeps L3 within the plan's named items.

## Review record

- **Revision 1 (2026-10-27):** written for Codex's review.
- **Revision 1, Codex review (2026-10-27):** needs revision, with seven
  should-fix points and a nit. The 33-method host set and its signatures,
  the seven retained methods, the facade `host` module with the `testing`
  re-export against L2's import guarantees, workspace revision 2 and
  inspection revision 15 were confirmed. Revision 2 addresses each point,
  and rebases every citation on 2a406a4 (L4a committed as 0.3.72; L3
  becomes 0.3.74 after L2):
  1. **Same-key redescription:** addressing stays fixed at mount; both
     directions between the default and an explicit `items` container,
     the type-change failure and the replacement-key usage are settled and
     documented, with tests of addresses, workspace paths and identity
     (section 3; tests 7 and the contract tests).
  2. **Doctests:** the runtime's rustdoc examples that call a moved method
     are listed with their imports by audience, and `testing.rs:20-21`'s
     root-export sentence is qualified (section 1).
  3. **Naming:** the structural container is transparent to naming as well;
     the exact consequence (only items the application marks structural)
     is designed and documented, the "unchanged" claim is removed, and
     inspection, query and content-search tests cover both kinds of item
     (section 3; test 8 and a contract test).
  4. **D3 and `NotLaidOut`:** the reachability audit is finished: no `None`
     survives N6, so D3 now recommends `Size`; `NotLaidOut` is kept as the
     driver's invariant report and its legacy test is deleted (section 2).
  5. **N6 criteria:** one `mount_tree` root per former `mount` keeps update
     counts, scheduling and root topology; the exceptions (overlap,
     overlay probes, self-reported geometry, absent geometry, a press over
     a root) are named (section 2).
  6. **Capture test:** the named assertion is admission's single pending
     job; the reply's component and crop are then checked; the custom-host
     example moved to the contract tests (section 6).
  7. **Host vocabulary:** the re-export set is audited against event
     translation and frame production and gains `PointerEvent`,
     `PointerKind`, `PointerButton`, `Point`, `Magnification`,
     `PhysicalExtent`, `DrawError` and `LimitError`, with stated
     exclusions; the example adds pointer input and a physical viewport,
     and an import test checks the set (sections 1 and 6).
  8. **Workspace serde:** the promise is rejection before a `Workspace` is
     built or applied; text and serde assertions are distinguished.
- **Revision 2, Codex recheck (2026-10-27):** needs revision, with three
  should-fix points and a nit; points 1, 2, 4, 6, 7 and the serde nit were
  resolved, and the audit was found to support D3's `Size`. Revision 3
  addresses each point (citations at 2a406a4):
  1. **Overlay fixtures:** the marker-cap test gets a 200 × 700 viewport so
     all 67 markers keep drawable bounds under real clipping; the
     `Place` root's outline is an authorized assertion change (16 commands
     instead of 12, indices shifted by four); the count-preservation
     promise is dropped (section 2, exception 3).
  2. **Zero-sized interactive box:** `validation.rs:110-131` joins the
     geometry exceptions, its allocation driven through shared placement
     state and its clip through `place_clipped`, keeping `EmptyHitArea`.
  3. **Naming:** the consequence is qualified for an `Each` that is not
     structural; a structural `Each` is documented, where an ordinary
     item's content-search fallback reaches a `Field` above it while
     inspection and queries do not change; new-behavior test 9 and the
     ordinary-item contract test cover content search and inspection
     separately (sections 3 and 6).
  4. **Root allocation:** the viewport supplies the initial root
     rectangle and the final allocation honors size limits; the proposed
     `Painter::size` rustdoc no longer promises an empty size for an empty
     viewport (section 2).
- **Revision 3, Codex recheck (2026-10-27):** needs revision, with one
  should-fix point and a nit; everything else, including the 200 × 700
  viewport, the 16-command expectation, the shared-placement fixture and
  the structural-`Each` naming test, was found correct. Revision 4:
  1. **Overlap/shortcut fixture:** its whole migration is settled as an
     authorized exception: overlap and shortcut coverage under `Place`,
     with scheduled layout asserted after hiding or removing a child and
     the idle state after flushing; duplicate-root admission and marker
     cleanup move to a separate fixture of independent `mount_tree` roots
     (section 2, exception 3).
  2. **Empty viewport:** the remaining claim now reads "With no viewport,
     or an empty one, development validation withholds geometric checks."
