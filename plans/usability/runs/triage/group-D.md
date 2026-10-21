# Verification, group D (task 3: custom control)

Probes (throwaway, outside the repository, built with
`CARGO_TARGET_DIR=target/triage-probes`):

- `probes/escape-cancel/` — a focusable (and a nonfocusable) component that
  captures the pointer on press, logs `keyboard_event`, pointer moves and
  `cancel_input`, and writes a signal inside `cancel_input`. Cases:
  `key_input(Escape)`, `command_key_input(Escape)`, `Runtime::cancel_input`,
  `clear_focus`, `set_visible(false)`, `unmount`, `Show` removal in a flush,
  `View::visible_signal` hide in a flush, runtime drop, and a control with no
  action.
- `probes/redescribe/` — `check_redescription` with a `Label`, with and
  without a viewport.
- `probes/pngsize/recompress.py` — re-deflates the IDAT of the testers'
  captures with zlib (filter none).

Probe results that the table relies on:

| Case | `cancel_input` called? | capture after | write in `cancel_input` |
|---|---|---|---|
| no action (control) | only at runtime drop | held (move at (400,400) delivered) | — |
| `key_input(Escape)` press+release | **no** (only at drop) | **released** (move not delivered) | — |
| `command_key_input(Escape)` | **no** (only at drop) | **released** | — |
| `Runtime::cancel_input` | yes, focused=true | released | Ok |
| `clear_focus` | yes | released | Ok |
| `set_visible(false)` | yes | released | Ok |
| `unmount` | yes | — | Ok |
| `Show` removal inside a flush | yes | — | Ok |
| `View::visible_signal(false)` inside a flush | yes | — | Ok |
| runtime drop | yes | — | Ok |
| nonfocusable capturer, nothing focused, `command_key_input(Escape)` | **no** | **still held** (move delivered) | — |

In both Escape cases `keyboard_event(Escape, Pressed)` and `(Escape, Released)`
reach the component after capture is already gone.

## Table

| id | verified | class | item or page | severity (tester) | suggested direction | destination if not fixed |
|---|---|---|---|---|---|---|
| r1-3-claude#5 (and the r1-3-claude acceptance note on Escape) | **yes.** Guide `crates/pert-ui/src/guide/custom-widgets.md:86-89`: "Cancel it in [`Component::cancel_input`], which the runtime calls when the widget loses its input context (Escape, hiding, removal, a modal surface)". The code never calls `cancel_input` for Escape: `key_transition` (`crates/pert-ui-runtime/src/runtime.rs:4679-4685`) and `command_key_input_inner` stage 8 (`crates/pert-ui-runtime/src/runtime/commands.rs:1484-1493`) only call `Node::lose_context` on the focused node (`runtime.rs:994-1008`: stops its timer, ends a drag it sourced, clears its pointer capture, cancels attached `ParameterBinding::begin_edit` sessions) and then deliver Escape to `keyboard_event` as an ordinary key. Probe confirms: no `cancel_input`, capture silently released before delivery. The per-method rustdoc is consistent with the code: `Component::cancel_input` (`runtime.rs:2417-2427`) lists "focus/context loss, hiding, removal and shutdown", not Escape; `command_key_input` stage 8 (`commands.rs:1251-1253`) says Escape "ends the focused component's timers, capture and gesture contexts before it is delivered". The library's own `ColorPicker` handles Escape in its `keyboard_event` (`color_picker.rs:1124-1131`), the same workaround the tester found; `docs/contracts.md:695` says Escape cancels "according to the component's contract". | documentation error | guide/custom-widgets.md "Pointer input that converges"; `Component::cancel_input`; `Runtime::key_input` | slow | Correct the guide: Escape reaches `keyboard_event` after the runtime has released the widget's capture, timer and parameter edit sessions, so cancel the gesture there (as `ColorPicker` does) and in `cancel_input` for the other losses; say the same on `Component::cancel_input`. Because the silent capture release leaves a widget that follows the current guide with stale drag state, the maintainer may instead decide to call `cancel_input` on Escape (contract change, not a doc fix). | TODO.md |
| r1-3-claude#3 | **partly.** True that nothing states whether `cancel_input` may write: the guide's write list (`custom-widgets.md:19-23`) names input callbacks, actions, mailbox handlers, `deliver_notifications` and `EventContext::defer`, not `cancel_input`/`focus_lost`; `Component::cancel_input` (`runtime.rs:2417-2427`) only says "Do not activate application actions from cancellation". True that "edit gesture" in `Runtime::key_input` (`runtime.rs:4484`) is undefined for a custom component: it means `ParameterBinding::begin_edit` sessions (`parameter_binding.rs:340-351`, attached via `EventContext::attach`, `runtime.rs:3434-3439`) plus capture and timer; a signal-bound custom widget cannot opt in. Writes in `cancel_input` do work on every path probed (table above); every runtime call site runs outside the `Update` phase (`EvaluationGuard` resets to `Idle` before the `cancel_input` call at `runtime.rs:5767-5772`). The tester's Notifier/`deliver_notifications` restore is valid but has a gap the docs imply: "A component removed before delivery is not called" (`runtime.rs:1636`), so a picker removed mid-drag never restores. | documentation gap | `Component::cancel_input`; guide/custom-widgets.md "Who may write, and when"; `Runtime::key_input` ("edit gestures") | slow | State on `Component::cancel_input` that it runs outside evaluation phases, that restoring the widget's own bindings there is allowed (and is the only restore that also runs on removal), and that "application actions" means callbacks/commands; define "edit gesture" in `key_input` as parameter edit sessions plus capture and timer, as `command_key_input` stage 8 already does. | TODO.md |
| r1-3-codex#1 | **yes.** `fn cancel_input(&mut self, cx: &mut EventContext)` returns `()` (`runtime.rs:2431`); its callers are infallible cleanup paths (`Runtime::clear_focus` returns `()`, `runtime.rs:4948`; runtime drop; removal). The rustdoc says nothing about a failed write. The library's own `ColorPicker` discards the error of its restore notification (`let _ = self.model.notify();`, `color_picker.rs:1167`). The tester's pattern (keep the error, return it from the next `update`, `submission/src/picker.rs:85-87,152-157`) is a reasonable invention, not a documented one. | documentation gap | `Component::cancel_input` | annoyance | Keep `()` (cleanup cannot fail) but document the intended route for a failed restore write: record it and request a `Notifier`, so `deliver_notifications` returns it from the next flush (or return it from the next `update`). | TODO.md (merge with r1-3-claude#3) |
| r1-3-claude#2 | **yes.** `SemanticValue` has `Text`, `Bool`, `Number {value, minimum, maximum}`, `Choice` (`crates/pert-ui-runtime/src/diagnostics.rs:256-287`, `#[non_exhaustive]`). The convention exists only on `ColorPicker`'s rustdoc "Inspection" (`color_picker.rs:347-354`: area is a `Slider` whose value is text, plus numeric `hue`/`saturation`/`value`/`alpha` properties). Neither `SemanticValue`'s rustdoc nor guide/custom-widgets.md "Inspection" (`custom-widgets.md:160-165`) mentions values of more than one dimension. | documentation gap | `SemanticValue`; guide/custom-widgets.md "Inspection" | annoyance | Add one sentence on `SemanticValue` and in the guide's Inspection section: a value with several components is reported as `Text` plus one typed `InspectValue::Number` property per component, as `ColorPicker`'s area does. A `SemanticValue::Point`-like variant is optional and needs a demonstrated tool need. | TODO.md (doc); follow-ups (variant) |
| r1-3-claude#7 | **yes, both parts.** (a) `check_redescription` mounts the reference as a second root `fresh` (`crates/pert-ui-runtime/src/conformance.rs:217`) beside the `redescribe` root, both laid out in the same viewport; probe: with a viewport set, stderr prints `[pert-ui validation ERROR UnexpectedOverlap] Components "redescribe" (#1) and "fresh" (#3) overlap; ...`; without a viewport nothing is printed. The report is empty afterwards only because the reference is unmounted. (b) The function's `# Errors` says "returns the retained tree under [`Redescribed::root`]" (`conformance.rs:193-194`); the component is `Redescribed::component` (`conformance.rs:81-82`), which only the struct page states. | defect | `check_redescription` | annoyance | Mount the reference so it cannot overlap (e.g. both trees under one stacking/overlap-allowing helper root, or the reference laid out off to the side) and add a test with a viewport that asserts no validation output; say in the function's prose that `Redescribed::component` is the re-described component. | TODO.md |
| r1-3-codex acceptance note: `size_limits` unwraps | **yes, application issue.** `submission/src/picker.rs:95-98` calls `Size::new(260.0, 260.0).unwrap()` and `SizeLimits::new(..).unwrap()` inside the infallible `size_limits`. The library offers non-panicking routes: `Size::from([u16; 2])` (`crates/pert-ui-runtime/src/layout.rs:123-131`), `SizeLimits::at_least` (const, `layout.rs:184`) and the fallible `Component::measure_limits` (`runtime.rs:1716`). Library observation: there is no infallible exact limit; `SizeLimits::new(size, Constraints::bounded(size))` is fallible (`layout.rs:209`) although min == max cannot be inverted. | tester error | (application code); optionally `SizeLimits` | not logged by the tester | None required; optionally add `SizeLimits::exact(Size)` (const, infallible). | TODO.md (optional improvement) |
| r1-3-codex acceptance note: non-finite replaced by 0.5 | **yes, application issue.** `PickPoint::new` (`submission/src/picker.rs:9-23`) is the application's own type; the library's validated 0..=1 type `NormalizedValue` is exported by the facade (`crates/pert-ui/src/lib.rs:287`) and rejects non-finite input (`NormalizedValue::try_from`). The AGENTS.md data rules bind the library, not the tester. | not friction | (application code) | not logged | None. | — |
| Author observation: capture PNGs ~100× too large | **yes, measured.** The encoder is `FrameImage::encode_png` (`crates/pert-ui-runtime/src/capture.rs:435-477`) with the private `png::encode` (`capture.rs:542-...`): filter type 0 on every row and "one IDAT chunk of stored (uncompressed) deflate blocks, so the encoder needs no dependency and the output is deterministic ... An application that wants smaller files re-encodes with its own codec" (`capture.rs:435-438`); guide/diagnostics.md:125 says it "encodes without any dependency". Measured: r1-3-claude `capture-dark.png` 1040×600: 2,496,858 bytes; same pixels re-deflated with zlib level 1: 32,932, level 6: 18,375, level 9: 16,229 (≈136× at level 6). r1-3-codex `dark.png` 640×400: 1,024,543 → 12,243 at level 6. Output size is always ≈ 4·w·h. | API ergonomics (documented trade-off, not a defect) | `FrameImage::encode_png` | — (author) | Improvement: compress while staying deterministic and dependency-light, e.g. a small built-in fixed-Huffman deflate with run-length matches over Sub/Up-filtered rows (flat UI captures compress very well even so), or an optional pure-Rust deflate dependency such as `fdeflate`/`miniz_oxide` (already in Cargo.lock through `png`, but not in `pert-ui-runtime`'s graph today). Keep the doc's determinism promise. | TODO.md |

## Merged findings

- **Escape and `cancel_input` (r1-3-claude#5, r1-3-claude#3 second half, r1-3-claude
  acceptance note).** One problem: the guide promises `cancel_input` on Escape;
  the runtime instead half-cancels (releases capture, timer and parameter edit
  sessions via `Node::lose_context`) and delivers Escape as a key.
  `Runtime::key_input`'s "edit gestures" wording hides which half.
- **What `cancel_input` may do (r1-3-claude#3 first half, r1-3-codex#1).** Both
  testers had to invent a policy for a restore write in cancellation (deferred
  through `Notifier`, or stored error returned from `update`). One doc fix on
  `Component::cancel_input` covers both: writes allowed, not an application
  action, the error route.

## Library problems found while verifying (not reported by any tester)

1. **`guide/controls.md:11` is wrong for Escape.** "Escape/context loss cancels
   activation, including capture owned by nonfocusable custom components."
   Escape's `lose_context` only targets the focused node
   (`runtime.rs:4679-4683`, `commands.rs:1489-1493`). Probe: a nonfocusable
   component holding capture with nothing focused keeps the capture through
   `command_key_input(Escape)`, receives no key and no `cancel_input`, and keeps
   getting moves. Existing tests cover nonfocusable capture only for window loss
   and last release (`crates/pert-ui/tests/editing/input_controls.rs:311`,
   `crates/pert-ui/tests/presentation/zoom.rs:201`). Class: documentation error
   (or defect, if the sentence is the intended contract). Destination: TODO.md.
2. **Stale comment in `ColorPicker::cancel_input`** (`color_picker.rs:1164-1167`):
   "Where writes are refused (hiding, removal) the notification fails". In the
   probe, a signal write inside `cancel_input` succeeded for `set_visible`,
   `View::visible_signal` hiding in a flush, `unmount`, `Show` removal in a
   flush and runtime drop. Either the comment describes a path not probed or
   it is outdated; the `let _ =` discard then hides nothing on those paths.
   Verify and correct the comment. Destination: TODO.md.
3. **Escape silently drops a focused component's `EventContext::set_timer`
   deadline** (`lose_context` → `stop_timer`, `runtime.rs:995`) without any
   callback; only `command_key_input` stage 8 mentions "timers", and
   `EventContext::set_timer` / `Component::cancel_input` do not. A widget that
   ignores Escape loses its pending timed action unnoticed. Documentation gap;
   fold into the fix for r1-3-claude#5.
4. **No infallible exact `SizeLimits`** (see the size_limits note above): a
   constant-size control must either unwrap or move to `measure_limits`.
   Improvement, low priority.

## Correction after the Codex review

r1-3-claude#7's second part is not verified as a documentation error:
`check_redescription` says success returns "the retained tree under
`Redescribed::root`", which is accurate, since `root` owns the wrapping
column. The tester's expectation that `root` is the component was a tester
error; a pointer to `Redescribed::component` would help. The overlap with a
viewport set remains a defect (F25).
