# Plan: improvements for application authors

Status: proposed 2026-10-18 and revised after an independent Codex review.
Slices 0 to 5 and 7, and the refusal notification of slice 6, are
implemented in 0.3.2 to 0.3.8 and squash-merged into `main` as 116ab0c,
with the [mixer-probe plan](archive/mixer-probe-improvements.md); each slice's own
status line says so. The large-buffer editing of slice 6 is deferred to
TODO.md TEXT-01. Slices 8 and 9 are not started, and this plan stays open
for them: every type and method they name is a proposal until a slice
implements and documents it.

## Background

On 2026-10-18 the maintainer's session built a small notes application against
the public facade only: a menu bar, Ctrl+N/Ctrl+S commands, a `VirtualList`
and a `TextField`/`TextArea` editor in a `SplitPane`, a status bar with a
derived word count and saving on a worker thread through a `Mailbox`. It was
driven headlessly with simulated keys, captured with the CPU renderer and run
natively on Fedora 44 GNOME/Wayland, where it worked. Two read-only reviews
covered capability gaps and API ergonomics. The application lived in a
session scratchpad; the observations below that come from running it are
reported experiment results, not a reproducible evidence record.

What worked well: commands with shortcuts shown in menus, the Ctrl/Cmd
mapping, `App::on_close` admission, headless testing with
`find_descendant`/`text_input`, CPU captures and the default appearance.

What the author ran into, verified in code or by execution:

- No general conditional mounting or value-driven child description. Bound
  visibility exists (`View::visible_signal` keeps descendants and their
  state), and `ListSelection::active`/`selected` are bindings that derived
  presentation can follow. What is missing is (a) mounting, replacing or
  repeating children from a value without a hand-written `Component` that
  calls `request_children` (the gallery's `TagEditor` and `Location`), and
  (b) an event-phase notification when a collection's selection changes, so
  an application can load or switch an editable detail binding when the user
  clicks a row. `VirtualList`, `Tree` and `Table` have activation hooks but no
  selection-change callback.
- `Column` and `Row` expose only gap customization: no grow weights,
  alignment, stretch or spacer, and `Row` centers each line vertically. The
  editor filled its pane only after switching to `RegionLayout`.
- Write and tree noise: `Signal::set` returns `Result<bool, RuntimeError>`,
  so one-line callbacks end in `.map(|_| ())`; every node is
  `Element::new(key, ..)`; `TextField::new` takes an owned `Signal` while a
  `TextSource` accepts `&Signal`; `UpdateContext::read` takes only a
  `ReadSignal`.
- `TextEditError::MAX_TEXT_BYTES` (16 KiB) bounds editor buffers and prepared
  text alike. A `TextArea` given a larger binding keeps its previous valid
  presentation, which is empty when no valid text was adopted, draws the
  error outline and accepts no edits, while `flush` returns `Ok`. Applications
  can preflight with `TextArea::check`, and inspection reports the refusal,
  but there is no dedicated application notification.
- A `Mailbox` created in the `App::new` closure and dropped at its end is
  closed, as documented: later sends return `Closed`, and values already
  accepted are dropped. The `#[must_use]` lint does not fire once the owner is
  bound to a name, and the closure offers no obvious owner; the gallery moves
  its mailbox into a button closure.
- `LayoutError::InvalidChild` carries no payload: the notes application's
  `invalid child placement` came from a `ListItems::build` row whose key
  differed from `key(index)`, a documented but redundant invariant.
- No public integration for native file dialogs, files dropped from the
  operating system, opening a URL, a window icon, maximize, fullscreen,
  window position or the system dark/light preference.
- The guide has a complete first-window chapter and a packaged
  dynamic-documents example, but its chapter examples read as test
  transcripts and there is no cohesive document-application tutorial. The
  facade re-exports the runtime at its root, and the prelude carries every
  name a `Component` signature uses.
- The default `desktop` feature always compiles wgpu, including for
  `App::software_rendering`, and `AppError::Limit` carries the
  `pert_ui_wgpu` `LimitError` re-export (with `ResourceKind`). A clean release
  build of the notes application took about 46 s and produced a 21 MB
  stripped binary on the development machine (reported, not recorded).

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

## Slices in order

### 0. Immediate documentation repairs

Status: implemented; documentation only, no version change.

No new API; can land at once. Resolves TODO.md DOC-01 to DOC-04: the `App`
examples using the legacy `Runtime::mount`, the appearance guide's zero-size
`Accent`, the keyboard guide that never names `command_key_input`, and the
`struct Label` that shadows the built-in in `guide/state.md`. Gate: rustdoc
and example row.

### 1. Binding, element and collection-key conventions

Status: implemented in 0.3.2. Decisions taken while implementing:

- `Signal` dereferences to its `ReadSignal` instead of a separate sealed read
  trait, so every `&ReadSignal` parameter, `map` and `combine` accept a signal
  without new methods; by-value parameters take `impl Into<..>`.
- `FieldHandle::set` and `ZoomController::set` follow the `set`/`set_changed`
  split; other `set_*` methods that report a change keep their `bool`.
- Not done: containers taking `impl Into<Element>` (an array needs one item
  type, which `.key()` already gives) and implicit child keys for one-child
  wrappers (keys stay the explicit identity). The two meanings of `read_only`
  stay: one is a binding view, the other an editor mode, on different types.
  The `disabled(bool)` setters on `Semantics`, `SemanticItem` and `PickItem`
  describe data, not a capability restriction, and stay.

Settles the shapes that later slices' constructors use, so they are not
churned afterwards.

- `Signal::set` returns `Result<(), RuntimeError>`; `set_changed` keeps the
  changed flag for the callers that use it. Add `update(|value| ..)` and
  `Signal<bool>::toggle`. Acceptance: a failed write changes nothing and
  notifies nobody, as `set` documents today; `update` defines its no-op,
  phase-error, reentrant-borrow and closure-failure behavior.
- A key extension so a component becomes an element in place (proposed
  `component.key("name")`), containers taking `impl Into<Element>`, and a
  default child key for one-child wrappers.
- Binding constructors accept `&Signal<T>`; `UpdateContext::read` and
  `map`/`combine` accept a `Signal` as well as a `ReadSignal`, through one
  sealed trait. Read-only capability separation stays.
- `ListItems::build` returns row content that the list keys with `key(index)`,
  removing the duplicate key. Enumerate the affected implementations (lists,
  tree and table adapters, file browser, collection filter, multi-select,
  tests) and give migration examples.
- Remove second ways found in review: `MenuItem::enabled_signal` (keeping
  restrictive `Permission` composition), the differing `disabled(bool)` and
  `disabled()` forms, and the two meanings of `read_only`. Choice-source
  exclusivity is unchanged.

### 2. Value-driven children

Status: implemented in 0.3.3. Decisions taken while implementing:

- One retained core reads the binding in update and requests children only
  when the value changed or the component itself was described again.
- `Show` takes a factory and unmounts while false; retained hiding stays
  `View::visible_signal`.
- `Each` places its keyed items in a container it describes (`Column` by
  default, `within` for another), so container layout, including later flex
  work, applies unchanged.
- Description callbacks receive a `DescribeContext`, a state owner that can
  create state but not read bindings or request children, so constructors
  such as `Select::new` work inside them.
- A kept key cannot change its component type (the runtime's existing
  `ChildTypeChanged` rule); the docs say to use another key instead.

- Decide first between retained visibility and conditional construction.
  Bound visibility already exists, so the new component is conditional
  construction: a `Show`-style component takes a factory, not an owned
  `Element`, and unmounting on false releases the child's state by design.
- `Each`-style keyed repetition over a `ReadSignal<Vec<T>>` and a
  `Dynamic`-style description from any binding, built on `request_children`
  with the retained-generated-child policy.
- Specify and test: callback replacement on redescription (a new build
  callback with an unchanged value must not keep obsolete child properties),
  binding replacement, duplicate-key rejection, failed-request recovery,
  reorder, incompatible component types at one key, focus and input
  cancellation, and subscription release on unmount. Signals are never
  published from reconciliation; use `check_redescription`.
- Replace the gallery's hand-written equivalents where they are the plain
  pattern. `Drawer` stays: its visibility is revealable and component-owned.

### 3. Selection notifications

Status: implemented in 0.3.4. Decisions taken while implementing:

- The callback reports both the active item and the selected set, before and
  after, as `ListSelectionChange`.
- A collection observes its selection in update and reports the origin of the
  latest write, which the `ListSelection` records: `User` for a collection's
  own gestures (including tree cross keys and the expander), `Application`
  otherwise, including a filter moving the active item, which happens only
  while the collection, not the filter field, has focus. Writes between two updates of one collection
  coalesce into one change; a hidden collection reports when shown again.
- The callback follows the collections' existing rule that callbacks belong
  to the first description of a mounted list, tree or table.
- Not separately reported: `Reveal` and `Restore` origins, since reveal does
  not write the selection and the selection is not persisted in workspaces.
- A source change does not prune selected keys: the selection keeps keys
  that are no longer shown, as `ListSelection` documents, so a source
  replacement reports no selection change.

- A selection-change callback on `VirtualList`, `Tree` and `Table`, delivered
  after layout settles through the public notification contract with its
  `ChangeOrigin`.
- Decide what it reports: the active item, the selected set or both; they
  differ in multiple selection.
- Specify shared controllers, source pruning of selected keys, reveal,
  workspace restore and application writes; follow the FIFO, failure, removal
  and hidden-window rules of [component notifications](../docs/contracts.md#component-notifications).

### 4. Flex layout in rows and columns

Status: implemented in 0.3.5. Decisions taken while implementing:

- Growth is element metadata, `Element::with_grow(u16)`: an integer weight
  cannot be invalid, `0` keeps the natural size, and custom containers read it
  through `MeasureContext::grow`, so built-in containers have no privileged
  path. `Element::spacer` is an empty element with weight 1.
- `justify` and `align` are `Row`/`Column` builders; `Wrap` keeps its line
  placement. Row cross alignment is relative to the tallest child, as its
  existing centering is, and the defaults keep the previous layout.
  Superseded in 0.3.10 by
  [mixer plan slice 2](archive/mixer-probe-improvements.md#2-cross-axis-fill-in-rows):
  cross alignment is relative to the row's allocated height.
- The spare space is what the container's allocation leaves, so a column in a
  scroll area grows children into the visible height while its content is
  shorter.
- No overflow shrinking, and default insets are unchanged. Superseded in
  0.3.25 by
  [log-explorer plan slice 1](archive/log-explorer-improvements.md#1-collections-in-unbounded-layout):
  a bounded column shrinks children
  below their natural size. Default insets are in the
  [follow-up index](follow-ups.md).

- Grow weights per child, main-axis and cross-axis alignment, cross-axis
  stretch and a spacer, as element-level layout properties or as `Column`/
  `Row` builders; choose one before implementing.
- Acceptance: weights must be finite and positive (typed rejection
  otherwise); behavior under unbounded constraints; surplus redistributed
  after maximums; minimum overflow; hidden children take no share; the
  current `Row` line centering stays the default; same-key redescription
  that changes weights.
- Default insets (list rows and a bottom status region touch the window edge)
  are a separate decision and not required for this slice.

### 5. Application resources, background work and diagnostics

Status: implemented in 0.3.6, as three parts. Decisions
taken while implementing:

- `Runtime::keep_alive` is the owner; shutdown drops kept values last, in
  reverse order.
- `Runtime::spawn` starts one thread per call (explicit, not a pool), bounds
  waiting tasks with `MAX_BACKGROUND_TASKS`, reserves a mailbox place per
  task, does not cancel on handle drop, and reports a panic as
  `BackgroundPanicked` from the flush that receives it.
- `Runtime::start_interval` is runtime-owned; the native host services it
  every loop turn so a withheld redraw does not stop it; late ticks do not
  catch up.
- Error context is a `FailureSite` from `Runtime::last_failure` rather than
  new payloads on `LayoutError`, so existing error matching keeps working;
  the `App` default report prints it. The `ListItems::build` key duplication
  was removed in slice 1.

Three branches:

- **Resource retention:** a documented owner for application-lifetime values
  created in the `App::new` closure (mailboxes, hosts, controllers), with
  defined drop order at shutdown.
- **Background work and timers:** a helper that owns its mailbox (proposed
  `ui.spawn(work, on_result)`) with bounded concurrency and queue, startup and
  panic failure, cancellation, shutdown, stale results and delivery errors
  defined; workers never hold UI handles, and it is excluded from
  time-critical producer use. An application timer whose lifecycle is stated
  explicitly: runtime- or window-owned, not cancelled by focus loss or input
  cancellation the way component timers are, behavior while hidden, missed
  intervals, zero delay, callback failure and shutdown, each with a
  deterministic headless test.
- **Error context:** `LayoutError::InvalidChild` and related layout and
  reconciliation errors identify the element (key path or widget identity).

### 6. Text refusal and large documents

Status: the refusal notification is implemented in 0.3.7 as
`on_binding_refusal` on `TextField` and `TextArea`. Large-buffer editing is
not started: besides the byte limit, one prepared text may be at most
`TextLimit::MAX_RASTER_SIDE` physical pixels tall, so a longer editor text
needs paragraph-partitioned preparation, and the public `TextSelection` is
built on one `TextLayout`. Choosing between extending `TextArea` and a
separate document editor is the maintainer's decision.

Two branches:

- **Refusal notification:** an event-phase notification when an application
  binding is refused, distinct from refused user edits and from normalizer
  rejection. It is not a user-write callback, and `FieldStatus` is not
  published from update. Test recovery, repeated invalid values, callback
  replacement and removal, and callback failure without replay.
- **Large-buffer editing:** separate the editor capacity from the label and
  toast limits. Raising the buffer limit alone is not enough: text
  preparation must be partitioned within the raster budgets. Before choosing
  a capacity, define the corpus, build and hardware, operations and latency
  statistic, covering long unbroken lines, grapheme boundaries, IME, paste,
  caret scrolling, replacing a large external value, memory retention and
  unchanged-line work. Decide whether this extends `TextArea` or becomes a
  separate document editor. Local editor undo stays separate from
  application undo.

### 7. Desktop without wgpu

Status: implemented in 0.3.8 as the `desktop-cpu` feature, with `native` and
`gpu` as its building blocks. `LimitError` and `ResourceKind` moved to the
runtime. The measured program is about a quarter smaller; the clean build was
not faster on a 24-thread machine (see verification). Duplicate dependency
pruning remains an optional follow-up.

- A feature configuration with winit, softbuffer and the CPU renderer but no
  wgpu, for small tools, with default and non-default configurations added
  to CI and AGENTS.md.
- `LimitError` and `ResourceKind` become facade-owned, so `AppError` no longer
  depends on the renderer crate.
- Record build time and binary size per configuration in verification.

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
- A smaller application prelude and thematic modules for the docs.rs index,
  as a separate public-API branch: removing a prelude name can break
  consumers. The API audit's part 6e (0.3.67) grew the prelude by about
  seventy names under its decided membership rule and added the
  `component` module for component authors; whether the prelude shrinks
  stays this slice's decision, on its own breaking branch.

Inputs from later plans and the 2026-10-20 audit:

- The guide already has a two-part getting-started tutorial
  (`guide/getting-started.md`, `guide/getting-started-continued.md`) and a
  screenshot and capture section in `guide/diagnostics.md`. The packaged
  examples (`live_mixer`, `diagram_editor`, `log_explorer`, `inventory`)
  are applications with headless sessions. The tutorial starts from these
  rather than beside them, as the testing chapter does.
- The [mixer-probe plan](archive/mixer-probe-improvements.md) found that the
  prelude lacks the names an ordinary command or live-data application uses.
  `CommandScope`, `KeyChord`, `CommandSource`, `LevelMeter`, `Trace`,
  `MeterAxis` and `EditOutcome` joined it in the API audit's part 6e
  (0.3.67). `series_bridge`, `SeriesConfig`, `SeriesSource`, `Delivery` and
  `Timestamp` stay at the crate root by decision G5-11, which leaves a
  thematic live-data module to this slice. A smaller prelude still needs
  the names that joined, or thematic modules that make them easy to find.
- The same probe found inspection bounds only under
  `details.box_model.border_box`. The diagnostics guide should say where a
  component's bounds are.
- The [developer-usability plan](developer-usability.md#round-1-findings)'s
  blind tests (2026-10-21) add discovery evidence:
  - **Per-row state (F16):** how per-row controls bind to an application's
    own `Vec`; an optional controlled-control API stays in follow-ups.
  That finding may instead be fixed in that plan's slice 6 if its joint
  checkpoint chooses it. Background work from a callback (F13) left this
  list: the API audit's part 6f2 (0.3.69) added `EventContext::spawn`, and
  the guide's worker section uses it.

## Optional follow-ups

The optional follow-ups of this plan are in the
[follow-up index](follow-ups.md): a general signal observer outside
components (declined for now), find in an editor buffer, a longer editor undo
history, pruning duplicated dependencies and default insets.

## Not in this plan

Accessibility integration, X11, browser rendering, an undo framework owned by
the library, charts beyond the live widgets, and date/time pickers.

## Review record

2026-10-18, Codex (`codex exec -s read-only`), two runs: one on this plan,
one looking for further defects. The plan review corrected overstated
background claims, the slice order, the gate table and missing lifecycle
decisions; this revision applies them except dropping the per-slice version
bump, which is the maintainer's convention. The defect run reported three
findings: the appearance guide's `Accent` and the keyboard guide's missing
`command_key_input` were confirmed (TODO.md DOC-02 and DOC-03); a reported
stale ColorPicker area after revoking permission during a hue drag was not
reproduced by a headless capture probe, which showed the area return to the
committed hue.

Implementation, 2026-10-18: each of slices 1 to 7 passed the workspace gate
and an independent Codex implementation review whose findings were fixed
before the next slice, as the squash commit 116ab0c states. The per-slice
findings were not recorded here. The native suite and the `desktop-cpu`
window checks are in [verification](../docs/verification.md).

2026-10-20 audit ([plan-audit-repairs](plan-audit-repairs.md)): the
gallery, the manual and some tests lag behind slices 2 to 6. The
reconciliation failures of slice 5 have no failure site. That plan repairs
both.
