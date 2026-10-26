# Complexity inventory

Measured 2026-10-26 at commit 6219dda (0.3.69, branch
`developer-usability`), before part 6f3 and round 2 (slice 7 of
[the developer-usability plan](../developer-usability.md)). It is the
evidence base for a possible subtraction pass. It is a dated measurement
record, so it states counts; they describe this commit only.

**Method.** Every number comes from committed content (`git show
HEAD:…`), because `crates/` was being edited in the working tree at the
time. API counts parse the committed `cargo-public-api` listings under
`api/` with a throwaway script: an item is counted once by name although
the listing repeats it under the crate root, `prelude` and `component`;
methods are inherent methods only, deduplicated per type (methods from
trait impls are excluded). Before `api/` existed (359122e, 2026-10-21),
growth uses a source proxy: lines matching `pub struct|enum|trait` and
`pub fn` in the five library crates' `src/`, which also counts items in
private modules and is calibrated against the listings where both exist.
Structural searches used `ast-grep` 0.45.3 on an extracted copy of HEAD.
Part 6f3 ([design](api-audit/design-6f3.md), approved in direction) is
marked *planned* wherever it changes a number.

## Summary

The worry is partly justified, but the cause is mostly not what it may
look like. The public surface is large: 526 type names in
`pert-ui-runtime` and `pert-ui-core`, about 1,900 inherent methods, 77
built-in components, a `Runtime` with 137 methods, a `Component` trait
with 63 hooks and a 1,410-line contracts document. Yet the API itself has
grown slowly. Since 0.3.0 the type count rose about 6% (514 to 544 by the
source proxy), and the whole usability fix plan (0.3.53 to 0.3.69) added a
net five types and about 25 methods, most of them its testing driver. What
grew fast is the rules: contracts.md by 48% (953 to 1,410 lines), the guide
by 59%, the prelude by 32% (278 to 368 names), and every fix closed a
friction by adding an option, a rule or a sentence, never by removing one.

**Essential complexity**, which serves a distinct need named in AGENTS.md
and which testers used successfully: the retained runtime with read-only
evaluation phases and event-phase writes; the parameter edit-authority
protocol; the bounded live-data endpoints and their real-time boundary;
inspection and capture; workspace persistence; native multi-window
presentation. All eight round 1 runs finished their task (five met, three
partly met); the custom-widget runs both met their acceptance with first
builds that compiled.

**Accidental complexity**, by evidence:

1. **Application models versus signal-bound controls.** Controls bind a
   `Signal`, applications own a `Vec` or a document, and the gap is closed
   by retained helper components with a `Notifier` and
   `deliver_notifications`: 41 lines in the guide's editable-list recipe,
   about 60 in the inventory example, two components in a tester's task
   list. Part 6f3 removes it for three controls.
2. **Per-component exceptions and timing rules.** Editors differ in what a
   revocation, a close or a rebinding does to a draft; callbacks fall into
   at least eight timing families with no shared vocabulary. 14 of the 36
   round 1 findings were hidden rules or timing surprises, not missing
   features.
3. **Flat discovery.** The prelude holds 70% of the crate root's names,
   including 38 parameter-authority and 59 component-authoring names;
   `Runtime` puts about 47 host-integration methods beside the 65 an
   application calls; contracts.md serves applications, widget authors and
   maintainers in one text, and about a quarter of it is maintainer-only.

Subtraction should therefore mostly *layer* rather than remove: hide host
and authoring surfaces behind modules, split the contracts by audience,
and remove the workaround paths once their replacements exist. Few public
items are true duplicates; the clearest are the legacy `Runtime::mount`,
the eight `Inline*` controls that mirror standalone controls' builders,
and the three deferral flavors.

## 1. Public surface

### By crate

From `api/<crate>/default.txt` at 6219dda.

| Crate | Structs | Enums | Traits | Free fns | Inherent methods | Trait methods declared | Variants and pub fields |
|---|---|---|---|---|---|---|---|
| `pert-ui-runtime` | 300 | 178 | 8 | 3 | 1,771 (271 types) | 95 | 1,570 |
| `pert-ui-core` | 24 | 13 | 3 | 0 | 109 | 7 | 96 |
| `pert-ui` (own items) | 3 | 3 | 0 | 0 | 18 | 0 | 22 |
| `pert-ui-cpu` | 1 | 1 | 0 | 0 | 4 | 0 | 7 |
| `pert-ui-wgpu` | 1 | 3 | 0 | 0 | 7 | 0 | 12 |

The runtime re-exports the core's items, so the facade exposes 526 unique
runtime and core type names plus its own six (`App`, `AppError`,
`AppShutdownError`, `ErrorAction`, `NativeCloseAction`, `HeadlessApp`),
`Svg` with `svg` and `CpuRenderer` with `cpu`. The median type has five
inherent methods. 77 runtime types implement `Component` (the built-in
widgets and containers).

Names by role suffix (runtime and core): 39 `…Error`, 16 `…Status`, 13
`…Controller`, 12 `…Outcome`, 8 `…Source`, 7 `…Receipt`, 6 `…Context`.
Thematic groups by name: about 49 parameter and edit-authority types
(`Parameter*`, `Any*`, `Delivery*`, `Authority*`, `Remote*`, `Sink*`,
`Edit*`), about 41 inspection, capture, profiling and validation types,
about 36 window, region, split and tool-window types and 26 command and
key types.

### Facade modules

| Module | Names | Source |
|---|---|---|
| crate root | 533: the runtime's 487 own items (299 structs, 177 enums, 8 traits, 3 fns), 41 core re-exports and the facade's 5 | `crates/pert-ui/src/lib.rs:29,43` |
| `prelude` | 368 from the runtime plus up to 8 from the facade (7 under `native`, `Svg` under `svg`) | `crates/pert-ui-runtime/src/lib.rs:388-691`, `crates/pert-ui/src/lib.rs:86-98` |
| `component` | 126; 96 are also in the prelude, 30 only here (receipts, semantics, native-window options, drag preview, editor attachment) | `crates/pert-ui-runtime/src/lib.rs:693-746`, `crates/pert-ui/src/lib.rs:143` |
| `testing` | 10 (`Driver`, `TestError`, six inspection and capture names, `HeadlessApp` under `native`, `CpuRenderer` under `cpu`) | `crates/pert-ui/src/lib.rs:186-199` |

Inside the prelude, by the same name groups: 59 component-authoring names
(contexts, `Painter`, path and geometry types, input events, drag types),
38 parameter and authority names, 25 window, region and tool-window names,
21 command and key names, 9 file-browser names and 8 workspace names.

### Central types

| Type | Public methods | Notes |
|---|---|---|
| `Runtime` (`runtime.rs:3647`) | 137 | By first rustdoc line: about 65 application operations (state, commands, focus and reveal, workspace, search, structure), 25 inspection, diagnostics and test entry points, 47 host-integration operations (deadlines, presentation requests, captures served, window selection, wake delivery, `paint`, IME state, `mount`'s legacy roots). 16 of the application ones are command registry methods. |
| `EventContext` (`runtime.rs:2996`) | 49 | Geometry and coordinate queries (about 11), focus (4), native windows (6), deferral and work (`defer`, `defer_detached`, `spawn`), queued requests (reveal, text command), drag (2), timers (2), clipboard (2), commands (2), profiling (3). |
| `UpdateContext` (`runtime.rs:2638`) | 25 | Reads (`read`, `read_paint`, `read_series`), structure requests (`request_children`, `request_layout`, `request_reparent`, `request_unmount`), `notifier`, `wake_after`, `animate`. |
| `Component` (`runtime.rs:1048-2551`) | 63 hooks | Only `update` is required; 62 have defaults. The trait's declaration, mostly rustdoc, spans about 1,500 lines. |
| `Element` (`layout.rs:344`) | 19 | Ten are five getter and `with_` builder pairs (`label`, `grow`, `detachable`, `searchable`, `rich_label`). |
| `Painter` | 26 | Drawing only. |

Builder vocabulary across types: `enabled` on 29 types, `disabled` on 32,
`for_parameter` on 12, `bound` on 8, `editable` on 8, `session` on 5;
57 `on_*` callback builders under 21 names, led by `on_change` (13 types),
`on_save` (7) and `on_rejection` (7).

## 2. Growth

### Headline numbers over time

The proxy columns are source counts (see Method); the listing columns are
exact and exist from 359122e.

| Point | Commit | Types (proxy) | `pub fn` (proxy) | Runtime prelude | Library src lines | contracts.md | Guide lines | AGENTS.md |
|---|---|---|---|---|---|---|---|---|
| 0.3.0, first prerelease | 0c7cfb2 | 514 | 1,733 | 278 | 125,407 | 953 | 2,393 | 408 |
| 0.3.17, app-author slices and mixer probe | 116ab0c | 527 | 1,768 | 286 | 128,085 | 1,052 | 2,637 | 425 |
| 0.3.23, diagram probe | 0ba201b | 528 | 1,777 | 286 | 128,671 | 1,072 | 2,873 | 425 |
| 0.3.31, log probe | 9d23503 | 529 | 1,782 | 287 | 129,942 | 1,078 | 2,941 | 425 |
| 0.3.44, inventory probe | 5a6e451 | 536 | 1,811 | 288 | 132,090 | 1,129 | 2,970 | 425 |
| 0.3.52, round 1 kit | bebb411 | 536 | 1,811 | 288 | 132,637 | 1,163 | 3,026 | 434 |
| 0.3.56, end of slice 5 (docs) | 3c2e63a | 536 | 1,811 | 288 | — | 1,247 | 3,372 | 455 |
| 0.3.67, prelude part | 2032b93 | 540 | 1,819 | 367 | 135,900 | 1,354 | 3,407 | 470 |
| 0.3.69, HEAD | 6219dda | 544 | 1,842 | 368 | 138,462 | 1,410 | 3,810 | 470 |

Doc-comment lines (`///` and `//!`) in the library crates: 27,115 at
0.3.0, 29,530 at 0.3.52, 32,559 at HEAD (+20%). The CHANGELOG grew from
11 to 2,278 lines.

| Listing | 359122e (0.3.52) | 6219dda (0.3.69) |
|---|---|---|
| Runtime structs / enums / traits | 298 / 173 / 8 | 300 / 178 / 8 |
| Runtime inherent methods | 1,746 | 1,771 |
| Runtime variants and fields | 1,518 | 1,570 |
| Facade structs / methods | 2 / 12 | 3 / 18 |
| Core | unchanged | unchanged |

### What each fix-plan part added or removed

Set differences of type names and `Type::method` names between
consecutive listings. Slice 5 (0.3.53 to 0.3.56) changed only
documentation; its `api/generate.sh` runs showed no difference
([fix plan](api-audit/fix-plan.md) progress entries).

| Part | Version | Types | Methods | Character |
|---|---|---|---|---|
| 6a1 | 0.3.57 | +1 (`ExpansionChange`) | +6 / −4 | renames and shape changes (`bind_*`, `Color::from_linear`, `SizeLimits::exact`) |
| 6a2 | 0.3.58 | +7 / −4 | +28 / −28 | renames; three boolean flags became enums (`Expansion`, `FocusDirection`, `SelectionExtension`); variants `DropPosition` → `ListDropPosition` |
| 6a3, 6b1-6b3 | 0.3.59-0.3.62 | 0 | 0 | trait impls, `Debug`, lints, `#[must_use]` |
| 6c1 | 0.3.63 | 0 | +2 (`ListingError::path`, `with_path`) | error payloads; new error variants |
| 6c2, 6d1 | 0.3.64-0.3.65 | 0 | 0 | diagnostics; reconciliation behavior |
| 6d2 | 0.3.66 | 0 | +3 (`CaptureJob` accessors) | defects |
| 6e | 0.3.67 | 0 | +1 (`Semantics::warning`) | **prelude 288 → 367 names**; new `component` module (126 names) |
| 6f1 | 0.3.68 | +3 (`Driver`, `TestError`, `HeadlessApp`) | +19, then +1 (`Runtime::can_focus`) | testing module; `TestError` has 11 variants |
| 6f2 | 0.3.69 | +1 (`HiddenActive`, 3 variants) | +3 (`CollectionFilter::hidden_active`, `EventContext::spawn`, `InlineText::in_row`) | plus `TextCommand::BeginEdit` |
| 6f3 (planned) | — | 0 | +3 (`Checkbox::controlled`, `Toggle::controlled`, `InlineText::controlled`) | a fourth ownership path in contracts.md |

Net over the plan: no public capability was removed; four types and 28
methods were renamed away and replaced. Every round 1 friction the plan
closed in code was closed by an addition: a helper (`Driver`,
`HeadlessApp`), a convenience (`EventContext::spawn`), an option
(`in_row`, `hidden_active`), a command (`BeginEdit`), a path (planned
`controlled`) or prelude membership. That is the growth mechanism to
watch: none of these additions retired an older way, and only 6f3 is
planned to make code unnecessary (the retained row components).

## 3. Parallel paths

Judgements: **distinct** (a separate need, keep), **overlapping** (the
needs overlap; a reader must learn the boundary), **candidate** (hide,
merge or remove, see section 7).

### How a control gets its value

| Path | Meant for | Judgement |
|---|---|---|
| `new(&Signal<T>)` | UI-local state the UI owns (`contracts.md:535`) | distinct |
| `for_parameter(Parameter<T>)`, on 12 types | an authoritative value with a host, revisions and begin/preview/commit gestures (`contracts.md:536`) | distinct, but a separate audience (see section 7, N2) |
| `bound(ReadSignal<Vec<Choice>>)`, on 8 types | where the *options* come from, not the value (`contracts.md:39-49`) | distinct; the shared vocabulary with value ownership is a learning cost |
| Per-row `Signal::sibling` in `ListItems::build` or `TableItems::cell` | a row control when `build` has no state owner (`signal.rs:354-363`) | **candidate**: a workaround; every redescription rebinds every mounted row and ends open drafts |
| A retained row component with `Notifier`, `reconcile` and `deliver_notifications` | the same, keeping a draft alive (`guide/layout.md:575-618`, `examples/inventory/app.rs:108-170`) | **candidate**: exists only to satisfy the library |
| `controlled(value, …, callback)` (planned 6f3) | the application's model owns the value; the control proposes changes (`design-6f3.md:47-130`) | distinct; it makes the two workaround rows unnecessary for `Checkbox`, `Toggle` and `InlineText` |
| `session(EditorSession)` on 5 types; `DraftEditor` with its controller | a host running one editor at a time (a table cell); an editor embedded in a custom component (`contracts.md:1028-1066`) | distinct, widget-author audience |

After 6f3 a `Checkbox` has three value paths (`new`, `for_parameter`,
`controlled`) and the per-row `sibling` form still documented for other
controls. The follow-up narrowed by 6f3 keeps `RadioGroup`, `Select`,
`Slider`, number inputs and `TextField` on the workaround paths
(`design-6f3.md:334-338`).

### Children from data

| Path | Meant for | Judgement |
|---|---|---|
| Static `Column::new([...])` and other containers | fixed structure | distinct |
| `Dynamic` | one child described from a binding | distinct |
| `Show` | a child that exists while a flag is true (unmounts) | distinct from `View::visible_signal` and `Runtime::set_visible`, which hide and retain (`contracts.md:246-247`); overlapping for a reader |
| `Each` | a keyed child per list item, in a `Column` by default, under an implicit `items` segment | distinct; the implicit segment was F9 and recurred in task 4 |
| `VirtualList` + `ListItems`/`ListSource`, `Table` + `TableItems`, `Tree` + `TreeItems` | large, selectable, filterable, reorderable collections | distinct; the `Each` versus `VirtualList` boundary was F14 (a tester chose `Each` and lacked selection) |
| `UpdateContext::request_children` | a custom component's own children | distinct, widget audience |
| `Runtime::reconcile_children` | imperative reconciliation from the runtime | overlapping with the value-driven containers for application code |
| `Runtime::mount_tree` / `Runtime::mount` | a root; `mount` makes a "legacy root" that is never laid out (`runtime.rs:4354-4363`) | `mount`: **candidate**; used only by gallery self-checks, facade tests and probes |

### Background and cross-thread work

| Path | Meant for | Judgement |
|---|---|---|
| `Runtime::spawn`, `EventContext::spawn` | one unit of work on a new thread, result to a UI callback (`contracts.md:674-685`) | distinct; the second is the same call queued from a callback (F13) |
| `Runtime::mailbox` → `Mailbox`, `UiSender` | a long-lived worker sending owned values | distinct |
| `series_bridge` → `SeriesSource`, read as `Telemetry`; `LiveWaker`, `WakeFlag` | lossy, high-rate producer data with a callback-safe endpoint | distinct, mandated by AGENTS.md's real-time boundary |
| `remote_parameter` + `RemoteHost` | an external engine's acceptance | distinct, parameter audience |
| `RequestTracker` | data a component asks its host to load (`contracts.md:699-715`) | distinct, widget audience |
| `Runtime::start_interval`, `EventContext::set_timer`, `UpdateContext::wake_after`, `animate` | application ticks; a component's event-phase timer; a reread deadline; animation | distinct by phase, but four timer mechanisms |
| `Runtime::keep_alive` | a lifetime owner for mailboxes and similar | distinct |

### Deferral and requests from callbacks

| Path | Lifetime | Judgement |
|---|---|---|
| `EventContext::defer` | the widget: cancelled if it is hidden or unmounted first (`contracts.md:999-1000`) | overlapping |
| `EventContext::defer_detached` | the widget's window (`contracts.md:1000-1002`) | overlapping |
| `Runtime::defer` | the runtime | overlapping |
| `EventContext::spawn` | the window, like `defer_detached` (`contracts.md:681-685`) | distinct (work, not runtime access) |
| `request_reveal`, `request_reveal_and_focus`, `request_text_command`, `request_focus*` | typed queued requests with receipts | distinct |
| `Notifier::request` → `Component::deliver_notifications` | a component's own event phase | distinct, widget audience, but appears in application recipes (section 4) |

The three `defer` flavors differ only in what cancels the task. The
widget-bound default is a trap for application callbacks on surfaces that
close: `Select` closes before its callback, so work must use
`defer_detached` (`contracts.md:494-496`), and the custom-widget guide
repeats the rule for dialogs (`guide/custom-widgets.md:284-289`).
**Candidate** for one deferral call with an explicit lifetime.

### Text editors

`TextField`, `TextArea`, `SearchField`, `InlineText`, `DraftEditor` (with
`DraftEditorController`), `IntegerInput`, `DecimalInput`, `InlineInteger`,
`InlineDecimal` and the read-only `SelectableText`, plus `EditorSession`
for hosts. The presentations are distinct needs. The cost is the
per-editor policies:

- **Writing:** `TextField` writes immediately; number inputs keep a draft
  and save on Enter or focus loss (`guide/getting-started-continued.md:48-51`).
- **Revocation:** `TextField` and `TextArea` keep an unsaved draft, number
  inputs and `InlineText` end the edit, `SearchField` discards a pending
  query (`contracts.md:355-362`).
- **Closing a window:** five distinct outcomes by editor kind
  (`contracts.md:914-923`).
- **Sessions:** five types accept `session`; parameter-bound editors ignore
  it (`contracts.md:1049-1050`).

The inline numeric rows mirror their standalone editors builder for
builder: `InlineInteger` and `IntegerInput` have 16 methods each,
`InlineDecimal` and `DecimalInput` 18 each. Judgement: presentations
distinct; the policy differences are **overlapping** and produced
findings (F4, F6); the inline duplication is a **candidate** without
friction evidence.

### Availability

One model: `enabled(impl Into<Permission>)` on 29 types, fixed shorthands
`disabled()` (32) and `read_only()` (9), `editable` on 8 editors,
`*_permission` where an application callback creates the capability
(`VirtualList`, `Tree`, `Table`), adjectives taking a `Permission` where
the component provides it (`SplitPane::resizable` and others), element
detach permission, and plain `bool` part properties on `FloatingWindow`
and `App` (`contracts.md:307-379`). Parameter editability is a further
restriction. Judgement: **distinct and consistent**; composition by
`Permission::and` is one rule. The adjective versus `*_permission`
naming (`contracts.md:324-330`) and the bool-versus-`Permission` line
(`contracts.md:332-335`) are rules a reader must learn, but no tester
reported friction with availability.

### Notification and callback timing

| Family | When it runs | May write? | Source |
|---|---|---|---|
| User-write `on_change` | after the control's own write, inside the input; `Select` closes first, `MenuItem` after closure is requested | yes | `contracts.md:489-516` |
| Editor `on_save`, `on_submit`, `on_rejection` | at the save or refusal | yes | `contracts.md:1015-1019` |
| Editor `on_modified`, `on_binding_refusal` | event phase after the flush settles | yes | `contracts.md:1021-1026` |
| Component notifications (`on_select`, `on_selection_change`, `Accordion::on_change`) | event phase after update and layout settle; selection coalesced per update | yes | `contracts.md:554-599` |
| Geometry (`on_scroll`, `on_resize`, `on_frame`) | event phase, at most once per flush, last value, committed only | yes | `contracts.md:381-393` |
| Parameter sinks (`on_outcome`, `on_preview`) | at delivery | **no**: post to a mailbox | `contracts.md:517-525` |
| Mailbox handlers, `spawn` completions | start of the next flush | yes | `contracts.md:661-685` |
| Deferred tasks, command actions | with `&mut Runtime` | yes, and may flush | `contracts.md:974-1003` |
| `timer_elapsed`, `start_interval` ticks | event phase; UI thread | yes | `guide/state.md:52-58`, `contracts.md:687-691` |
| `App::on_close`, `on_error` | after close admission; on failure | yes | `contracts.md:904-947` |

Each timing is justified in contracts.md, but there is no shared name for
the families, so a reader learns per callback whether it may write,
whether layout has settled and whether changes are coalesced. The
parameter-sink row produces the "refresh mailbox" dance that applications
write (`guide/state.md:184-242`, `examples/live_mixer.rs:250-253,386,399`).
Judgement: **overlapping**; a documentation candidate (named timing
classes), with no evidence that fewer families are possible.

## 4. The minimal path

Non-blank, non-comment lines; "types" are distinct library type names the
code writes.

| Task | Source | Lines | Types | Code that exists for the library |
|---|---|---|---|---|
| First window (profile form) | `examples/getting_started.rs` | 33 | 10 | one `clone` pair to move signals into the button (`:10`); a `.key` on every element (9) |
| Custom widget (indicator) | `guide/getting-started-continued.md:118-152` | 27 | 14 | none; 3 of 63 hooks implemented |
| Editable, selectable, filterable list | `guide/layout.md:530-681` (application part) | 134 | 27 | the retained `Title` component, 41 lines and 9 types (`:575-618`); a deferred path lookup to begin editing (`:658-667`); a combined binding guarding Delete against a hidden active row |
| Same, tests | `guide/layout.md:682-742` | 47 | 7 | — |
| Headless test | `guide/testing.md:26-50` | about 20 | 11 | — |
| Settings editor (no guide recipe) | round 1 task 1 submissions | 304 (Claude, app only), 577 (Codex, app and tests) | — | `defer` then `Runtime::spawn` for the save (F13, since fixed) |

**Planned 6f3 effect on the list recipe:** the `Title` component and its
four types (`Component`, `Invalidation`, `Notifier`, `UpdateContext`)
disappear, the checkbox becomes `Checkbox::controlled(…)`, and the title
lookup shortens from `[key, "title", "editor"]` to `[key, "title"]`
(`design-6f3.md:319-330`): about 41 of 134 lines.

### Packaged examples

| Example | Lines | `impl Component` | Library-shaped code |
|---|---|---|---|
| `getting_started` | 33 | 0 | — |
| `dynamic_documents` | 99 | 0 | `let _operation = cx.defer(…)` |
| `live_mixer` | 672 | 0 | the capacity-one refresh mailbox and its `let _ = sender.send(())` (`:252,386,399`), required because parameter sinks cannot write |
| `diagram_editor` | 2,812 | 2 (`Canvas`, `NodeView`: real custom widgets) | three `let _receipt = cx.defer_detached(…)` (`app.rs:600,821,852`) |
| `inventory` | 1,632 | 1 (`ActiveCell`) | `ActiveCell`, about 60 lines, whose rustdoc says it exists because "a checkbox binds a signal, and `TableItems::cell` has no state owner" (`app.rs:108-170`); planned for removal in 6f3 |
| `log_explorer` | 1,456 | 0 | — |

The guide shows `impl Component for` 8 times outside the custom-widget
chapter: in state (2), appearance (3), controls (1), layout (1) and
getting started (1). The state chapter's first example teaches local state
through a custom component (`guide/state.md:9-31`), so an application
author meets `Component` before any application-level state example.

Close and error policies receive the `Runtime` but not what the
initializer built; the guide teaches an `Rc<RefCell<Option<…>>>` slot
filled by the initializer (`guide/custom-widgets.md:290-320`), recorded as
a follow-up ("Typed state handoff", `plans/follow-ups.md:33-35`).

### In tester code

| Run | Task | `impl Component` | `Notifier` uses | `sibling` | `retain_initial_children` |
|---|---|---|---|---|---|
| r1-2-codex | task list | 2 | 2 | 1 | 1 |
| r1-2-claude | task list | 0 | 0 | 2 | 0 |
| r1-3-claude | custom control | 1 | 3 | 0 | 0 |
| others | — | 0 or 1 (task 3) | 0 | 0 | 0 |

A task-list application, not a widget task, wrote two components and a
`DraftEditorController` lifecycle to keep per-row edits alive.

## 5. Who must know which contract

Classes: **A** an ordinary application author, **W** a custom-widget or
host author, **M** maintainers only (internal ordering, failure
provenance, policy). Mixed sections are listed by their primary class.
`docs/` is not packaged, so consumers see these rules only where rustdoc
or the guide repeats them; shipped docs link contracts.md in three places
(`mailbox.rs:3`, `guide/diagnostics.md:91`, `guide/surfaces.md:119`).

### docs/contracts.md (1,318 lines in sections)

| Class | Lines | Sections |
|---|---|---|
| A | 465 (22 sections) | choice sources, element identity, captions, text limits, placeholders, adornments, value-driven children, controller ownership, permissions, geometry notifications, container validation, user-write callbacks, ownership paths, parameters, producer data and mailboxes, commands and modal admission, native and headless key entry, command palette, calling the runtime from callbacks, text and draft editing, theme palettes, workspace persistence |
| A with W detail | 209 (5) | component redescription, layout and focus, component notifications, reveal and focus, inspection |
| A with M detail | 93 (3) | titles and inspection labels, closing a window, rendered capture |
| W | 214 (11) | retained generated children, host-loaded data, physical keys, stacking and hit testing, activation keys, drag and drop, editor sessions, search field, capture and restore with children, content search |
| M | 337 (8) | menus, pickers and dialogs; delivery order and hidden windows; failure provenance and input cleanup; runtime shutdown; restore and active gestures; compatibility policy; error policy; naming rules |

An application author needs between a third and a little over half
(465 to 767 lines), and much of that is per-component detail consulted
on demand. About a quarter is
maintainer-only and could leave the document an author is pointed to.

### Guide chapters (3,810 lines)

| Chapter | Lines | Class |
|---|---|---|
| getting started, both parts | 199 | A (the custom-widget section, 53 lines, W) |
| state | 391 | A for local state, workers and recovery (143); A with parameters for gestures, families, sinks, overlays and external authority (248) |
| layout | 794 | A (the editable-list recipe alone is 277) |
| controls, surfaces | 980 | A |
| appearance | 518 | A (experimental text coverage, 62, is advanced) |
| custom widgets | 333 | W, except "Building the application" (45) |
| testing, diagnostics | 597 | A as test author |

About 3,400 guide lines address application authors, of which about 250
assume the parameter protocol; about 350 address widget authors.

## 6. Tester evidence

Round 1: two AI testers (Claude Opus 5.5 and Codex `gpt-6.1-sol`), one
run per task each, at 0.3.52 ([runs](runs/README.md)). The evidence is
exploratory: AI agents read documentation exhaustively, guess API names
well and do not tire, so they likely understate the cost of reading
contracts and overstate discoverability for people. The friction entries
were 17 *understand*, 8 *discover*, 6 *compile*, 6 *docs-wrong*, 6
*runtime* and 1 *missing*; no tester read library source.

Classification of the 36 findings
([round 1 table](../developer-usability.md), lines 412-447) by their
primary cause:

| Cause | Findings | Count |
|---|---|---|
| Complexity: hidden rules, timing, several ways, address rules | F3 (prelude/root split), F4 (draft not settled before `on_close`, also a defect), F7c (filters trim the query), F9 (`Each`'s `items` segment), F12 (what `cancel_input` may write), F16 (signals versus the app's `Vec`), F17 (`retain_initial_children` and cancellation order), F18 (Escape routing), F21 (`Field` description precedence), F22 (editor key precedence), F26 (slot versus structural path), F27 (`InlineText` takes the row's press), F31 (shared address, `AmbiguousKey`), F34 (two recipes for modifiers) | 14 |
| Missing piece | F2 (testing helpers), F5 (headless close), F6 (number rejections), F7b (hidden-selection policy), F13 (`EventContext::spawn`), F15 (begin edit from code), F24 (2-D value), F30 (boxed errors) | 8 |
| Documentation gap or error | F1, F7a, F8, F10, F14, F20, F28, F32 | 8 |
| Small ergonomics | F11 (error type named wrongly), F19 (`#[must_use]` on `CommandId`), F23, F29, F33 | 5 |
| Defect | F25 | 1 |

The testers' own "concepts I had to learn" lists point the same way. The
task-list runs list `Signal::sibling` for per-row state, the
`DraftEditorController` owner lifecycle, event-phase notifications and
reconciliation of controller-owned children
(`runs/r1-2-claude/FRICTION.md`, `runs/r1-2-codex/FRICTION.md`). Every
task 4 run lists the split between `command_key_input` with
`native_modifiers` and `key_input`, and capture hosting
(`set_capture_support`, `request_capture`, `serve_captures`); part 6f1's
`Driver` now hides both. Positive evidence: four runs report a first
build that compiled or nearly did, testers praised the precision of the
reference pages, and no run read library source.

Of the 14 complexity findings, the fix plan resolved F16 (planned 6f3),
F15 and F27 by additions, F3 by growing the prelude, and the rest by
documentation, a defect fix or an error message. None was resolved by
removing a way of doing something.

## 7. Subtraction candidates

Prioritized by evidence and saving. "Breaking" refers to the unpublished
API, where AGENTS.md permits documented breaking migrations.

### Safe now

| # | Item | Evidence | Action | Breaking | Saves an application author |
|---|---|---|---|---|---|
| S1 | Retained row components and per-row `Signal::sibling` as the taught way | `guide/layout.md:575-618`, `examples/inventory/app.rs:108-170`, F16, r1-2-codex | Complete 6f3 as designed; then teach `controlled` first in `ListItems::build`, `TableItems::cell` and `Signal::sibling`, keeping the retained form as one sentence for controls without a controlled form | no (planned additions; docs shrink) | 41 lines and four types in the list recipe; about 60 lines in a table app; the `Notifier`/`deliver_notifications` concept for ordinary apps |
| S2 | contracts.md mixes three audiences | section 5: 337 maintainer-only lines | Move the M sections (delivery order, failure provenance, shutdown, menus and dialogs internals, restore and gestures, compatibility, error and naming policy) to a maintainer document; keep A and W rules in contracts.md, W ones under their own heading | no (docs) | about a quarter of the document an author is pointed to |
| S3 | Callback timing without a vocabulary | section 3 timing table; F4, F12, F18 | Name three or four timing classes in contracts.md (in-input, after-settle event phase, next flush, no-write sink) and state the class in each callback's rustdoc | no (docs) | learning each callback's timing separately |
| S4 | `Runtime` page mixes host, test and application methods | 47 of 137 methods are host integration | Group the rustdoc into "Applications", "Tests and inspection", "Hosts" sections now; decide N3 separately | no (docs) | scanning 137 methods for the 65 an application calls |
| S5 | No "which way" table per concept | sections 3 and 4; F14, F34 | One table each at the top of the state and layout chapters: value source, children from data, background work, deferral | no (docs) | choosing between `Each` and `VirtualList`, `Show` and `visible_signal`, the `defer` flavors |
| S6 | The state chapter opens with a custom `Component` | `guide/state.md:9-31` | Open with a signal and a control; move the component example to the custom-widget chapter | no (docs) | meeting `Component` before it is needed |

### Needs a decision

| # | Item | Evidence | Action | Breaking | Saves |
|---|---|---|---|---|---|
| N1 | Prelude of 368 names, 70% of the root | section 1; slice 9's "smaller application prelude" (`plans/app-author-improvements.md:360-365`); 6e grew it by 79 | A prelude for the first program (controls, containers, `Signal`, `Runtime`, `App`, `Element`, errors, `Permission`, list and form types, about 120-150 names), with thematic modules beside `component` and `testing`: `parameter` (38 names), `commands` (21), `windows` (25), `inspection` | imports only | a docs.rs prelude page an author can read; reverses most of 6e, so it needs the maintainer's rule first |
| N2 | The parameter and authority protocol in the ordinary path | 49 types, 38 in the prelude; 248 guide lines; 12 `for_parameter` constructors | Keep the protocol (AGENTS.md names it), but move its names to a `parameter` module and its guide sections under one "Authoritative values" chapter | imports only | about 10% of the type surface for apps without a host |
| N3 | Host integration on `Runtime` | 47 host methods (`take_presentation_requests`, `set_wake_delivery`, `complete_capture`, `select_window`, …) | Move them to a host extension trait or `host` module that `App` and custom hosts import | yes, for custom hosts | a third of `Runtime`; also decides whether custom hosts are a supported 0.x audience |
| N4 | Three `defer` flavors with different cancellation | `contracts.md:494-496,999-1003`; `examples/diagram_editor/app.rs:600,821,852` | One `EventContext::defer` with an explicit lifetime argument, or window-bound by default for application callbacks | yes | a trap in closing surfaces |
| N5 | Per-editor draft policies | `contracts.md:355-362,914-923`; F4, F6 | One draft policy for revocation and close across text and number editors, with documented exceptions only where the presentation requires one | behavior | several per-editor rules |
| N6 | `Runtime::mount` (legacy root, never laid out) | `runtime.rs:4354-4363`; used by gallery self-checks and facade tests | Remove, or move to `testing`; `mount_tree` covers laid-out roots | yes, small | one root API |
| N7 | Lookup and address rules | F9, F26, F31; `find` (110 uses), `find_descendant` (33), `find_path` (2) | Drop `find_path` from the public surface; make `Each` not insert an `items` segment, or document it at `Each::new`'s first line | yes, small | the address rules that recurred in task 4 |
| N8 | `Inline*` controls mirroring standalone controls | 8 types; `InlineInteger`/`IntegerInput` 16/16 methods, `InlineDecimal`/`DecimalInput` 18/18 | Consider an inline presentation option on the standalone controls | yes, large | about 6 types and 70 builder methods; no tester friction, so lowest priority |

### Keep

- `Signal` versus `Parameter`: distinct ownership, distinct audiences.
- Live-data endpoints separate from mailboxes: required by AGENTS.md's
  real-time rules.
- `Dynamic`, `Show`, `Each`: small, distinct, and cheaper than a custom
  component.
- The `Permission` model: one composition rule across 29 types and no
  tester friction.
- The `Component` trait's 63 hooks: 62 have defaults, the indicator
  implements three, and both custom-widget runs met their acceptance.
  Grouping the trait's rustdoc by concern is a documentation task.

## Open questions for the maintainer

1. Is the parameter and authority protocol a first-release audience, or an
   advanced module (N2)? The answer decides how much of the prelude and
   the state chapter it keeps.
2. Which rule should the prelude follow: "every name an application may
   write" (6e, 368 names) or "the names of a first program" (slice 9)?
   The two are in tension; doing 6e and then slice 9 migrates consumers
   twice.
3. Are custom hosts a supported audience before 1.0 (N3)? If not, about 47
   `Runtime` methods can leave the application's view.
4. Should a fix for a friction finding state what it makes unnecessary?
   The fix plan closed every finding by addition; a rule for round 2's
   fixes could require that each addition names the path it retires, or
   says why none can be.
5. After 6f3, should `Signal::sibling`'s per-row role leave the
   documentation entirely, leaving one taught way per control, or wait
   until the controlled follow-up covers the remaining controls?
6. Should round 2 record lines and distinct types per task, beside the
   friction log, so the next inventory measures the same thing for people
   and AI testers?
7. Is the unified editor draft policy (N5) acceptable as a behavior change,
   or are the per-editor policies deliberate product decisions?

## Re-measurement at 46a219b (fix plan closed, before the layering pass)

Measured 2026-10-26 at commit 46a219b (0.3.70, branch
`developer-usability`): the API audit fix plan closed with parts 6f2 and
6f3, before layering part L1. It records the post-6f3 baseline apart from
the layering pass, as [the layering plan](layering-plan.md) ("Round 2 and
attribution") requires.

**Method.** The header's method and this document's definitions, applied
to `git archive` extractions of 46a219b and 6219dda, because the working
tree was being edited for L1. The 6219dda column was measured again with
the same throwaway scripts; where that differs from the value recorded
above, the table gives the recorded value with the re-measured one in
brackets, and the difference is taken against the re-measured value. The
contracts.md classes are this document's section 5 classes, with section
lengths counted below each heading as there; a new section is classed by
its primary audience. Names whose source did not change were checked by
diff rather than recounted: `crates/pert-ui/src/lib.rs` and
`crates/pert-ui-runtime/src/lib.rs` are identical at both commits, and
the `api/` listings differ only by three added method lines (each listed
under the crate root and `prelude`).

| Measure | 6219dda (0.3.69) | 46a219b (0.3.70) | Difference |
|---|---|---|---|
| `pert-ui-runtime` structs / enums / traits / free fns | 300 / 178 / 8 / 3 | 300 / 178 / 8 / 3 | 0 |
| `pert-ui-runtime` inherent methods (types with one) | 1,771 (271) | 1,774 (271) | +3 |
| `pert-ui-runtime` trait methods declared | 95 | 95 | 0 |
| `pert-ui-runtime` variants and pub fields | 1,570 [1,571] | 1,571 | 0 |
| `pert-ui-core` structs / enums / traits; methods; trait methods; variants and fields | 24 / 13 / 3; 109; 7; 96 | same | 0 |
| `pert-ui` own items; `pert-ui-cpu`; `pert-ui-wgpu` | unchanged rows of section 1 | same | 0 |
| Unique runtime and core type names | 526 | 526 | 0 |
| Types implementing `Component` | 77 | 77 | 0 |
| Crate root / `prelude` / `component` / `testing` names | 533 / 368 (+8 facade) / 126 / 10 | same | 0 |
| Prelude groups: authoring / parameter / windows / commands / file browser / workspace | 59 / 38 / 25 / 21 / 9 / 8 | same | 0 |
| `Runtime` / `EventContext` / `UpdateContext` / `Element` / `Painter` methods | 137 / 49 / 25 / 19 / 26 | same | 0 |
| `Component` hooks | 63 | 63 | 0 |
| Types (proxy) / `pub fn` (proxy) | 544 / 1,842 | 544 / 1,844 | 0 / +2 |
| Library src lines | 138,462 | 138,857 | +395 |
| Doc-comment lines | 32,559 [32,553] | 32,753 | +200 |
| AGENTS.md lines | 470 | 477 | +7 |
| contracts.md lines | 1,410 | 1,498 | +88 |
| contracts.md lines in classified sections | 1,318 | 1,404 | +86 |
| Class A | 465 (22 sections) | 534 (23) | +69 |
| Class A with W detail | 209 (5) | 226 (5) | +17 |
| Class A with M detail | 93 (3) | 93 (3) | 0 |
| Class W | 214 (11) | 214 (11) | 0 |
| Class M | 337 (8) | 337 (8) | 0 |
| Lines an application author needs (A to A + A/W + A/M) | 465 to 767 | 534 to 853 | +69 to +86 |
| Guide lines | 3,810 | 3,763 | −47 |
| Guide: layout chapter / editable-list recipe section | 794 / 277 | 742 / 225 | −52 / −52 |
| Guide: controls and surfaces | 980 [979] | 984 | +5 |
| Guide: getting started, both parts | 199 [198] | 198 | 0 |
| Guide: state / appearance / custom widgets / testing and diagnostics | 391 / 518 / 333 / 597 | same | 0 |
| List recipe, application part: lines / types | 134 / 27 | 93 / 22 | −41 / −5 |
| List recipe, retained `Title` component: lines / types | 41 / 9 | removed | −41 / −9 |
| List recipe, tests: lines / types | 47 / 7 | 47 / 7 | 0 |
| `inventory` example lines / `impl Component` | 1,632 / 1 | 1,570 / 0 | −62 / −1 |
| `inventory` retained `ActiveCell` component | about 60 [57] | removed | −57 |
| Other packaged examples' lines and `impl Component` | section 4's rows | same | 0 |

- The API grew only by part 6f3's three controlled constructors
  (`Checkbox::controlled`, `Toggle::controlled`, `InlineText::controlled`);
  6f2's review fixes after 6219dda changed behavior and documentation, no
  public item, export or prelude name. The `pub fn` proxy counts two
  because `Checkbox` and `Toggle` share one generated constructor.
- contracts.md grew by 88 lines, 66 of them 6f3's new "Controlled
  controls" section (class A) and 13 more 6f3 text in component
  redescription, ownership paths and user-write callbacks; 6f2's focus
  handover rule added 8 lines to "Layout, visibility and focus", and the
  contents list one entry. All the growth is in application-facing classes, so the maintainer-only share
  falls from 26% to 24% by dilution alone, and an application author's
  share rises.
- The guide shrank by 47 lines, the first decrease in the growth table:
  6f3 replaced the editable-list recipe's retained `Title` component and
  its two-way explanation with controlled controls (−52), and the
  controls chapter gained five lines introducing `controlled`.
- The list recipe's application part lost 41 lines and five types
  (`Component`, `EventContext`, `Invalidation`, `Notifier`,
  `UpdateContext`), matching the 41 lines section 4 planned; the fifth
  type is `EventContext`, which only the removed `deliver_notifications`
  signature named. The title lookup is now `[key, "title"]`.
- The inventory example lost 62 lines because 6f3 replaced `ActiveCell`
  with `Checkbox::controlled`; no packaged example now implements
  `Component` except the diagram editor's two real custom widgets.
- AGENTS.md grew by seven lines from 146133e (unpublished breaking changes
  are welcome where they improve the library) and 6f3's two approved
  sentences on controlled controls; source and doc-comment growth is 6f3's implementation and
  rustdoc and 6f2's fixes.

Not reproduced exactly at 6219dda: the runtime's variants and fields
(1,571 by this parse, 1,570 recorded; no variant or field changed between
the commits), doc-comment lines (32,553, 32,559 recorded), the guide's
getting-started pair (198, 199 recorded) and controls with surfaces (979,
980 recorded; the guide total of 3,810 matches), and `ActiveCell` (57
non-blank, non-comment lines, "about 60" recorded). The `component`
module's 126 and the crate root's 533 were not recounted; neither source
nor listing changed. The CHANGELOG's 2,278 lines recorded above are its
length at 6311e48, where this inventory was written; at 6219dda it has
2,263 lines, and at 46a219b 2,358.
