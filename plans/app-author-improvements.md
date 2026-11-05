# Plan: improvements for application authors

Status: open for slices 8 and 9; not started. Slices 0 to 5 and 7, and the
refusal notification of slice 6, were implemented in 0.3.2 to 0.3.8 and
squash-merged into `main` as 116ab0c; their text was removed from this plan
on 2026-11-05 and is in Git history (`git show f5fde76:plans/app-author-improvements.md`).
The large-buffer editing of slice 6 is deferred to TODO.md TEXT-01 and its
design plan [document-editor.md](document-editor.md). Every type and method
slices 8 and 9 name is a proposal until a slice implements and documents it.

## Background

On 2026-10-18 the maintainer's session built a small notes application
against the public facade only and ran it headlessly and natively on Fedora
44 GNOME/Wayland. The gaps the completed slices did not close are:

- No public integration for native file dialogs, files dropped from the
  operating system, opening a URL, a window icon, maximize, fullscreen,
  window position or the system dark/light preference.
- No cohesive document-application tutorial.
- An editor's text is bounded by `TextEditError::MAX_TEXT_BYTES` and one
  prepared `TextLayout` (TODO.md TEXT-01).

Accessibility integration and X11 remain outside the initial scope by
decision ([AGENTS.md](../AGENTS.md)); this plan does not change that.

## Working rules

- One slice per branch. Each slice updates rustdoc, examples, the gallery,
  the manual and the CHANGELOG with its own API, and records the replacement
  usage for every breaking change, migrating all repository consumers.
- The maintainer's convention is a patch version bump per slice; it is not a
  compatibility requirement while the library is unpublished.
- An independent Codex review (`codex exec -s read-only`) before a slice is
  called merge-ready.
- Read [contracts](../docs/contracts.md) before each slice, in particular
  component redescription and retained generated children, component
  notifications, user-write callbacks, calling the runtime from callbacks and
  workspace persistence.
- Gates, per the [AGENTS.md](../AGENTS.md) table:

  | Work | Required gate |
  |---|---|
  | Prose only | Author review, links and whitespace |
  | Rustdoc and tutorial examples | Targeted doctests, example compilation, warning-free rustdoc |
  | Public API changes, including prelude and module layout | Workspace gate, focused contract tests, semantic review |
  | Dependencies, features or platform integration | The full dependency/platform row, CI updated for new configurations, and native evidence recorded in [verification](../docs/verification.md) |

  A slice escalates to the last row as soon as it adds a dependency, a
  feature or platform integration. Loom models run only if the core `live`
  protocols change.

## Open slices

### 6. Large documents (deferred)

Status: the refusal notification is implemented in 0.3.7 as
`on_binding_refusal` on `TextField` and `TextArea`. Large-buffer editing is
not started: besides the byte limit, one prepared text may be at most
`TextLimit::MAX_RASTER_SIDE` physical pixels tall, so a longer editor text
needs paragraph-partitioned preparation, and the public `TextSelection` is
built on one `TextLayout`. Choosing between extending `TextArea` and a
separate document editor is the maintainer's decision.

- **Large-buffer editing:** separate the editor capacity from the label and
  toast limits. Raising the buffer limit alone is not enough: text
  preparation must be partitioned within the raster budgets. Before choosing
  a capacity, define the corpus, build and hardware, operations and latency
  statistic, covering long unbroken lines, grapheme boundaries, IME, paste,
  caret scrolling, replacing a large external value, memory retention and
  unchanged-line work. Decide whether this extends `TextArea` or becomes a
  separate document editor. Local editor undo stays separate from
  application undo.

### 8. Operating system services

Separate branches, each with native evidence on the platforms it claims;
NATIVE-01..04 track existing gaps and do not cover these services. OS
dependencies stay behind the facade and platform boundary, and the runtime
stays window-free.

- **File dialogs:** asynchronous completion, cancellation distinct from
  failure, owner-window identity, modality, completion after the owner
  closed; no blocking file work in components.
- **External file drop** and **opening a URL or file** in the default
  application.
- **Window controls:** icon, maximize, fullscreen and position, with
  unsupported operations reported per platform.
- **Window geometry persistence:** stable window addressing, units, requested
  versus observed geometry, restore fallback and how documents of the current
  workspace schema version are handled, with encode/decode, serde round-trip
  and rejection tests for removed windows and invalid geometry.
- **System dark/light preference** as an opt-in theme mode.

### 9. Application tutorial and API layout

- A notes or todo tutorial (state, value-driven list, editing, saving through
  background work, error feedback, shortcuts, close admission), written after
  the APIs it shows have settled; each slice still documents its own API.
- Guide examples shaped as applications with harness lines hidden. The
  chapter on headless testing and CPU captures exists: the API audit's part
  6f1 (0.3.68) added "Testing an application" with the `testing` module's
  driver and `HeadlessApp`.
- Met by layering part L2 (0.3.73): the smaller application prelude and the
  thematic modules (`parameter`, `commands`, `windowing`, `inspection` and
  the live-data module `live`) exist, so they have left this slice's inputs
  (`usability/layering-design-L2.md`). The tutorial and application-shaped
  examples stay; L2 reshaped the guide's representative examples as an
  application part with the test in `fn main`.

Inputs from later plans and the 2026-10-20 audit:

- The guide already has a two-part getting-started tutorial
  (`guide/getting-started.md`, `guide/getting-started-continued.md`) and a
  screenshot and capture section in `guide/diagnostics.md`. The packaged
  examples (`live_mixer`, `diagram_editor`, `log_explorer`, `inventory`)
  are applications with headless sessions. The tutorial starts from these
  rather than beside them, as the testing chapter does.
- The mixer-probe plan (`plans/archive/mixer-probe-improvements.md`) found that the
  prelude lacks the names an ordinary command or live-data application uses.
  `CommandScope`, `KeyChord`, `CommandSource`, `LevelMeter`, `Trace`,
  `MeterAxis` and `EditOutcome` joined it in the API audit's part 6e
  (0.3.67). `series_bridge`, `SeriesConfig`, `SeriesSource`, `Delivery` and
  `Timestamp` stayed at the crate root by decision G5-11, which left a
  thematic live-data module to this slice. Layering part L2 (0.3.73) met
  this input: `pert_ui::live` holds the bridge, `SeriesSource`, the
  mailboxes and the live widgets, `commands` the command names beyond a
  shortcut's chord and scope, and `parameter` `EditOutcome`.
- The same probe found inspection bounds only under
  `details.box_model.border_box`. The diagnostics guide should say where a
  component's bounds are.
- The developer-usability plan (`plans/archive/developer-usability.md`)'s
  blind tests (2026-10-21) added discovery evidence, which has left this
  list: background work from a callback (F13) with the API audit's part 6f2
  (0.3.69), which added `EventContext::spawn`, used by the guide's worker
  section, and per-row state (F16) with part 6f3 (0.3.70), which added
  controlled `Checkbox`, `Toggle` and `InlineText`, used by the guide's
  editable list recipe and the inventory example.

## Optional follow-ups

The optional follow-ups of this plan are in the
[follow-up index](follow-ups.md): a general signal observer outside
components (declined for now), find in an editor buffer, a longer editor undo
history, pruning duplicated dependencies and default insets.

## Not in this plan

Accessibility integration, X11, browser rendering, an undo framework owned by
the library, charts beyond the live widgets, and date/time pickers.

## Review record

2026-10-18, Codex (`codex exec -s read-only`) reviewed the plan; slices 1 to
7 each passed the workspace gate and an independent Codex implementation
review, as the squash commit 116ab0c states. The full record is in Git
history.
