# Changelog

## 0.3.91 (2026-10-29)

Round 3 historical reveal diagnostics, hidden-window documentation and public
focus-continuity recipes.

- RevealRefusal Debug now prints immutable runtime-local WidgetId serials
  without live mounted state. A retained refusal's text stays identical after
  opening its container, unmount and Runtime drop. Standalone WidgetId Debug
  keeps reporting current mount state. No field, variant, signature or persisted
  format changes; the serial is the inspection id available while mounted.
- This also changes nested Debug formatting of RevealOutcome, RevealStatus,
  RevealReceipt and TestError, including Cleanup and boxed errors; TestError's
  Display uses the same historical refusal. ModalBlocked formatting is unchanged.
- Runtime::reveal/reveal_and_focus and content-search jumps document the
  operational RuntimeError::Hidden from a host-hidden native window, distinct
  from an explicit set_visible refusal. Driver::click/click_item, queued
  request_reveal/request_reveal_and_focus, RevealStatus/RevealOutcome and
  RuntimeError::Hidden document the forwarded error and retry after known
  native visibility resumes. No reveal or host visibility behavior changes.
- Public facade tests retain genuine Dialog, bound View and explicit-hiding
  refusals across lifecycle transitions, covering Display, normal/pretty Debug,
  terminal receipts and nested errors. A negative control restoring derived
  live Debug fails. Host-hidden public reveal/click/item/jump/receipt tests
  confirm the existing operational error and successful visibility retry.
- The layout guide now demonstrates Button rows that set active selection and
  transfer focus to the list, so arrows and Enter continue from the clicked row;
  VirtualList::on_activate links the recipe and keeps its Enter-only behavior.
  A separate retained-tab recipe observes Redescription, defers and settles
  current selection before refocusing a replacement editor only while the strip
  still has focus. Missing editors and stale selection leave focus alone.
  Both recipes use existing public APIs; no runtime focus or selection policy
  changes. Public input tests and negative controls cover ownership, remount,
  other-region focus and deferred selection/focus changes.
- Runtime::register_command and Runtime::invoke_command now explicitly document
  that actions may call operations that flush, including reveal/focus, and that
  recursively invoking the same command returns BorrowConflict.

## 0.3.90 (2026-10-28)

Round 3 disabled-action presentation, from the independently reviewed Button
design checkpoint. Frozen blind results remain on 0.3.88/0.3.75.

- Disabled framed Button now draws a neutral Surface fill, Muted content and
  inward Outline at BorderWidth, instead of keeping the enabled Button fill.
  This makes the state differ in its surface and boundary as well as its text.
  Size, padding, corner radius, permissions and gesture cancellation are
  unchanged. Disabled frameless buttons keep content only.
- The roles deliberately share the inactive editable-field frame; there is no
  independent disabled-button palette. Button overrides affect enabled
  buttons, while Surface/Muted/Outline/BorderWidth also affect their other
  users. Zero border width omits the outline; a width reaching half the smaller
  extent follows the existing filled-stroke behavior. Custom themes own contrast.
- Empty/all-fixed-disabled Menu's framed trigger inherits the new frame.
  Icon-menu, history and panel controls that are frameless keep their existing
  disabled treatment. Frameless buttons subscribe only to frame roles they
  actually draw; changes to an undrawn frame schedule no presentation work.
  AlertDialog buttons share the paint code but have no
  disabled state, so no dialog visual change is claimed.
- Updated Button and Button::frameless, ColorRole::Button/OnButton and
  MetricRole::BorderWidth documentation plus controls/appearance/rendering
  guidance. No public signature or persisted format change.
- Focused tests cover held-input revocation/re-enable, paint-only role/metric
  changes, zero border width, frameless and contextual-command behavior, Menu
  trigger composition, and actual CPU pixels beside editable fields in both
  built-in palettes. Existing availability and palette contrast tests supply
  their unchanged contracts.

## 0.3.89 (2026-10-28)

Round 3 usability repairs: discoverable composite controls and real pointer
geometry for tab titles and the option list shared by menus and Select.
Blind submissions remain on the frozen 0.3.88 and 0.3.75 kits; these repairs
have regression evidence, not a new blind-test result.

- Added `Element::expose_part`, `Runtime::find_exposed_part`,
  `testing::Driver::{exposed_part, exposed_parts}` and
  `testing::ExposedPart` with public `name` and `id` fields. The UI-local
  record is non-exhaustive and implements Clone, Debug, Eq and PartialEq.
  Names are exact scoped selectors, independent of keys, semantic labels and
  workspace paths. Queries do not flush, reveal, focus, select a window or
  schedule work. Internal wrappers forward declarations; ordinary application
  children and exposed parts bound nested scopes. Repeated builders replace;
  same-key redescription replaces/removes metadata without remounting or
  invalidating layout solely for the name. Retires ad hoc traversal of private
  structural keys for the supported built-in parts; custom composites use the
  same public declaration.
- Added `RuntimeError::EmptyExposedPartName`,
  `RuntimeError::AmbiguousExposedPart { owner, name }`, and
  `testing::TestError::ExposedPartNotFound { owner, key, name, available }`.
  Blank declarations fail before adoption; duplicate names remain enumerable
  but singular lookup refuses ambiguity. Invalid owners are checked before
  query names. Names select current parts; returned WidgetIds target events.
- Breaking inspection format: added `ComponentInspection::exposed_part`;
  `InspectionSnapshot::SCHEMA_VERSION` advances to 17. Serialized metadata is
  required and nullable, and present blank names are refused. Snapshots and
  query results reject other revisions. Workspace schema 2, addressing markers
  and naming propagation are unchanged.
- Select/InlineSelect expose `control` and `options`; Menu exposes its actual
  `trigger` Button and `items`; MenuBar exposes `title/<menu-key>`; menu levels
  expose `submenu/<row-key>` panels with nested `items`. Slashes are literal.
  Editors' owned menus are nested under `clipboard`. Structurally embedded
  search fields in MultiSelect and QuickPick bound their clipboard scopes with
  `search`; QuickPick/CommandPalette retain the existing `surface` address
  boundary. These are part names, not new persisted path segments.
- Tabs title items and OptionList rows now report local pointer rectangles
  from their painting/hit geometry. Titles use the live scroll offset and the
  current key/incarnation, clipped outside overflow arrows; rows use the live
  row offset and viewport, excluding separators. Offscreen/hidden items have
  no window bounds. Driver::click_item uses actual pointer input, does not
  scroll to an item or open a closed menu/Select list, and disabled command
  rows invoke nothing. Retires application-computed coordinates and keyboard
  navigation as workarounds for pointer testing of these items.
- Re-exported the existing `ApplicationError` and its unchanged members under
  `component`, meeting that module's component-author contract. Its existing
  constructors, conversions and traits appear at the additional path in API
  listings; no error behavior changes. The component-only callback example and
  membership tests cover reporting an application failure without a root import.
- Documentation: complete public examples for parts and tab/menu/Select pointer
  input, a custom composite using scoped parts, actual Dialog/AlertDialog
  inspection/address contracts, list Button rows versus Enter activation,
  pre-lookup settling in reveal_child, App::new's named RuntimeError-returning
  builder wrapper, windowing imports for splits, empty editor semantic values,
  drawing-test imports, owned normalizer arguments, choice Eq requirements,
  ApplicationError message conversions, register_command routing precedence,
  and a retained mailbox worker recipe
  with explicit full/closed handling and bound table-cell guidance.
- Contract tests cover scoped/custom lookup, ambiguity and validation, identity
  errors and redescription idleness, another presentation window, naming and
  workspace path collisions/round trips, closed/disabled/submenu pointer input,
  live tab scrolling and recreated keys, clipping/view scale/dynamic title
  widths, detached-title activation, and schema-17 round trips/rejection.
  Existing item-reporters TODO narrowed to the remaining control families.

## 0.3.88 (2026-10-28)

Gallery: a Revert for the sign-in form, the native evidence the round 2 fix
plan's part P12 still lacks. No library change.

- Gallery: Text › Input states has a Revert button (key `revert`, in
  `sign-in-actions`). Its action clears the statuses the application
  publishes on the user-name and password fields with `FieldHandle::clear`,
  then calls `FormFields::discard_drafts`, and reports how many shown fields
  were asked to discard their drafts; the requests are delivered after the
  action, so a refused draft disappears, its error ends and the unsaved
  count falls once the flush settles, while saved values stay. The section's
  help text says that a refused entry keeps its draft (focus leaving saves
  only a valid one) and what Revert does; the gallery README follows.
- Tests: the gallery test
  `the_sign_in_examples_revert_discards_a_refused_draft_and_clears_its_marks`
  types a too-short user name, saves it with Enter, activates Revert through
  keyboard input and checks the bound value, status, summary and outcome;
  it fails with the `discard_drafts` call removed.
- Verification: `docs/verification.md`'s 0.3.87 "Form drafts" row names the
  gallery Revert as the next hand check (Text › Input states: `ab` in User
  name, Enter, then Revert); it has not been run.

## 0.3.87 (2026-10-28)

Form drafts and the settings recipe (round 2 fix plan, part P12, R2-2 by
decision D2, from its approved design checkpoint
`plans/usability/round-2-design-P12.md`, revision 2, with decisions P12-D1,
P12-D2 and P12-D3).

- Added (R2-2): `FormFields::discard_drafts(&self, &mut EventContext) ->
  Result<usize, RuntimeError>`: queues `TextCommand::DiscardDraft` through
  `EventContext::request_text_command` for every field control a form
  records (a `Field` or `Property` bound with `form`) that is visible at the
  call, in registration order, and returns the number queued, whatever the
  fields' marks or statuses say; `ForeignRuntime` for a caller of another
  runtime than the form, queueing nothing, whether or not a field is shown
  (P12-D1). Delivery, dropped and modal-blocked targets, a target's error
  and partial progress are `request_text_command`'s; an editor that still
  holds its draft at delivery reports `on_rejection(None)` within it where a
  refusal was reported and `on_modified(false)` after settling, while a draft
  a changed write in the same callback already ended reports as that write
  does. The call writes no signal, status or mark and requests no focus.
  Retires collecting editor ids for a per-editor `request_text_command` loop
  as the way to end drafts from a Revert; `DiscardDraft` stays the
  single-editor route.
- Documentation (R2-2): `docs/contracts.md`'s draft policy rule 3 says the
  draft stays after an equal write and how to end it, the `DiscardDraft`
  paragraph adds `on_rejection(None)`, `FormFields::discard_drafts` and the
  route of a draft a changed write ended, and the matrix row "Application
  write" reads "Application write of a changed value" (no cell changes; the
  policy-table test passes unchanged). `TextField`'s normalization,
  `IntegerInput`'s page (which `DecimalInput` and the inline rows follow) and
  `InlineText::normalize` say that writing the value the field shows keeps
  the draft and what ends it; `TextField::on_rejection`, the number editors'
  and `InlineText`'s `on_rejection` add `DiscardDraft` to where a reported
  refusal ends. `FormFields` (it can discard every field's draft, and
  schedules nothing of its own except the text commands this queues),
  `Field::form`, `Property::form`, `FieldHandle::remove` and
  `FieldHandle::set_modified` (a removed editor reports nothing, so an
  application that removes it while keeping the handle clears the mark).
  The state chapter gains "A settings form": fields registered in a
  `FormFields`, dirty state through the tuple `ReadSignal::combine`, Apply
  through `EventContext::spawn`, a Revert that writes the applied values,
  clears the statuses it published and discards the drafts, and a close
  policy over `App`'s state, tested with `App::headless` for an equal and a
  changed bound value; the guide's and the chapter's summaries name it. The
  testing chapter's close-policy paragraph points to it; the controls and
  getting-started chapters and `docs/manual.md` name the form discard.
- Changed: the diagram editor's inspector counts the drafts its label and
  size fields keep in a `FormFields` they report to through `on_modified`,
  and its close policy asks with the existing discard dialog while the
  document is dirty or a draft is kept (P12-D2; its message now reads "The
  diagram or the inspector has unsaved changes."), so a refused inspector
  draft is no longer lost with the window without a prompt (TODO.md entry
  resolved). Its `Shown::One` carries an inspector incarnation that `sync`
  advances, forgetting the marks, only when what is shown changes, and the
  panel is keyed by node and incarnation, so leaving a node remounts its
  panel, also when the node is selected again before the next update, and a
  cleared count never hides a surviving draft. The size editors have keys of
  their own (`width-input`, `height-input`). `main.rs` builds the `App` in
  `application(dir)`, which the window runs, and the session's new
  `close_admission` step starts it with `App::headless` and checks close
  admission through `HeadlessApp::admit_close` with the `native` feature,
  replacing the stand-in that released focus and called the policy itself;
  without `native` the step and `close_requested` are compiled out. The
  session also checks that the inspector's form counts a refused label
  draft.
- Tests (R2-2): the facade's `editing` binary gains `form_drafts`: an equal
  write keeps a refused and a kept draft and Revert's `discard_drafts`
  discards both with `Ok(2)` and the summary unchanged inside the callback,
  while after a changed write the refusal's status stays until Escape; the
  call counts every visible recorded control (an editor without a draft and
  a draft without `on_modified` included) and skips a field never shown, one
  described with `status_signal` and a hidden control, and a control hidden
  before the flush is dropped without error; a failing `on_rejection(None)`
  returns from the flush with the later request still queued and its `None`
  pending; a discarded focused `InlineText` session returns focus as Escape
  does, standalone and in a list row; a caller in another runtime gets
  `ForeignRuntime` for a form with and without shown fields; and a
  modal-blocked target spends its request while the later one is delivered
  by the next flush. Each new-behavior test, and the diagram editor's
  `close_admission` step, was shown to fail under its named mutation.
- Follow-up recorded in `plans/follow-ups.md`: a form discard from a
  command action, which receives `&mut Runtime` (P12-D3).
- Fixed (R2-2, same kind, the Revert path above): a changed application
  write that ends a refused draft in an ordinary `IntegerInput` or
  `DecimalInput` no longer leaves the draft's error behind, so inspection
  no longer reports `validation_error` beside `valid: true` until the next
  edit; the reported refusal's `None` still stays pending until the next
  save or Escape. Tests: the `editing` binary's
  `a_changed_write_ending_a_refused_number_draft_leaves_no_error` for both
  editors, and the form-draft test's changed-value case, both shown to fail
  with the fix removed.
- TODO.md: the diagram editor's `close_requested` entry is removed.
- API listings: `api/pert-ui-runtime/default.txt` and `serde.txt` list
  `pub fn pert_ui_runtime::FormFields::discard_drafts(&self, &mut
  pert_ui_runtime::EventContext) -> core::result::Result<usize,
  pert_ui_runtime::RuntimeError>` at the root and in `prelude`. No other
  listing changes.
- Native evidence: none for this change (`docs/verification.md`); the hand
  check of the diagram editor's close with a refused draft is the author's,
  and no native window has a form Revert.

## 0.3.86 (2026-10-28)

Application state from the initializer (round 2 fix plan, part P11, R2-1 by
decision D9, from its approved design checkpoint
`plans/usability/round-2-design-P11.md`, revision 2, with decisions P11-D1,
P11-D2 and P11-D3).

- Breaking (R2-1): `App` takes the application state as a type parameter,
  `App<S = ()>`: `App::new`'s initializer returns `Result<S, AppError>`,
  and `on_close`, `on_error` and `on_window_error` receive `&mut S` after
  the runtime. Replacement usage: `.on_close(|ui, state, window| ..)`,
  `.on_error(|ui, state, error| ..)`, `.on_window_error(|ui, state, error|
  ..)`; without state, `|ui, _, window|`. An application that shared what
  its initializer built through a slot returns it instead:
  `App::new("Notes", |ui| Ok(build(ui)?)).on_close(|_, notes, _| ..)`. An
  initializer that never returns `Ok` (one that always fails or panics)
  names the type: `App::<()>::new(..)`, since a defaulted type parameter
  does not take part in inferring a call. `App` written alone, as in
  `fn app() -> App`, still names `App<()>`, and an `Ok(())` initializer
  without handlers compiles unchanged. A callback runs only after the
  initializer returned its state, one at a time, so a change it makes to
  the state persists to its next call; the library keeps the state and
  never reads, copies, compares or persists it.
- Breaking (R2-1): `HeadlessApp<S = ()>`, returned by `App::headless`;
  `HeadlessApp` written alone still names `HeadlessApp<()>`.
  `HeadlessApp::admit_close` passes the same state to `on_close` that
  `state` reads.
- Added (R2-1): `HeadlessApp::state`, `state_mut` and
  `runtime_and_state_mut` (P11-D2: the runtime and the state at once, as a
  callback receives them, so a test drives the runtime with the handles the
  state holds without cloning them out first).
- Changed (R2-1): the state, and the values the callbacks capture, drop
  after the runtime has shut down on every path: the end of `App::run`,
  `HeadlessApp::shutdown`, a dropped `HeadlessApp` and an `App::headless`
  whose first flush failed, also while a panic unwinds out of the
  initializer or the first flush; the order captured callback values already
  had. The native runner stays non-generic: `App::run` runs the
  initializer where it did, after the event loop exists, and erases the
  state into a private owner (P11-D1), so a call that cannot run still
  never builds the host's content. The packaged examples (`diagram_editor`,
  `inventory`, `log_explorer`, `live_mixer`) return their application
  object as the state instead of keeping it with `Runtime::keep_alive`
  (P11-D3), and the diagram editor's slot and its empty-slot branches are
  gone. The gallery's checks write `|_, _, _|`.
- Documentation (R2-1): `App`'s page gains "Application state", with an
  example whose close policy keeps the window open while the state holds
  unsaved text; `App::new` states the inference rule with a runnable
  example (`let app: App = ..` and `App::<()>::new`); `on_error`'s example
  shows a failure on a status line the state holds; `on_window_error`
  gains an example counting failures in the state and states its default
  report (`[pert-ui window error]` on standard error) and that a failed
  report ends the application with `AppError::WindowErrorReporting` (same
  kind); `on_close`'s fifth close step names the state. `HeadlessApp`'s
  example, the testing chapter's "The close policy" (with a builder
  function returning `App<Notes>`) and the custom-widgets chapter's
  "Building the application" no longer build a slot. `Runtime::keep_alive`
  is the owner for code that has only a runtime; an `App` initializer
  returns such values as its state. `docs/manual.md`, `docs/contracts.md`
  ("Callbacks outside the classes", "Calling the runtime from callbacks",
  application services), the `testing` module page and the crate root's
  table. `plans/follow-ups.md`'s "Typed state handoff from `App::new`" is
  removed.
- Tests: the facade's `surfaces` binary checks that a close policy reads
  and changes the state the initializer returned, read and reset through
  `state` and `state_mut`, and that the state and a callback's captured
  value are dropped after the runtime has shut down (shutdown, drop, a
  failed first flush, never started, and while a panic unwinds out of the
  first flush or the initializer); the runner's unit tests drive
  `on_error` with state through component recovery, repair and exit, and
  drop the erased owner with its runtime open. Each was shown to fail
  under its named mutation. The `close` tests read `state` and
  `runtime_and_state_mut` instead of a slot helper, and a failing
  initializer with panicking handlers still returns its error. The native
  smokes check `on_error`, `on_window_error` and `on_close` with state, and
  the probes that overlap on purpose report `OverlapPolicy::Allow` and
  assert a validation report without `UnexpectedOverlap` (TODO.md entry
  resolved; the proposal to summarize overlap reports is re-recorded as
  its own entry). The native checks have not been run for this change
  (`docs/verification.md`).
- Fixture: `plans/usability/task4-app` requires `pert-ui` 0.3.86; its
  `Ok(())` initializer compiles unchanged.
- API listings: every `api/pert-ui/*.txt` with `native` (`default`, `cpu`,
  `images`, `image-formats`, `serde`, `svg`, `no-default-desktop-cpu`)
  lists `pert_ui::App<S>` (at the root and in `prelude`) and
  `pert_ui::testing::HeadlessApp<S>` with `impl<S: 'static>` blocks;
  `App::new`'s initializer returning `Result<S, AppError>`; `on_close`,
  `on_error` and `on_window_error` with `&mut S`; `App::headless`
  returning `HeadlessApp<S>`; the added `HeadlessApp::state`, `state_mut`
  and `runtime_and_state_mut`; the unchanged methods listed under the
  generic types (`App<S>::size`, `min_size`, `max_size`, `resizable`, `run`
  and, with `cpu`, `software_rendering`; `HeadlessApp<S>::runtime`,
  `runtime_mut`, `main_window`, `admit_close` and `shutdown`); and `Debug`
  and the auto-trait lines in
  their generic form (`impl<S> Unpin for HeadlessApp<S> where S: Unpin`,
  `UnsafeUnpin` likewise; `App<S>`'s are unconditional), which follow
  from the type parameter. No runtime, core, CPU or wgpu listing changes.

## 0.3.85 (2026-10-28)

Derivations over several bindings (round 2 fix plan, part P10, R2-3 by
decision D10, from its approved design checkpoint
`plans/usability/round-2-design-P10.md`, revision 3, with decision P10-D1).

- Breaking (R2-3): `ReadSignal::combine` is an associated function over a
  tuple of 2 to 12 signals or read bindings, whose function receives a
  reference to the tuple of their values:
  `ReadSignal::combine<I: CombineInputs>(I, impl Fn(&I::Values) -> T +
  'static) -> Result<Self, RuntimeError>`, in an impl over the output
  `T: Clone + PartialEq + 'static`. It replaces the method
  `ReadSignal<T>::combine<U, V>(&self, &ReadSignal<U>, impl Fn(&T, &U) -> V
  + 'static)`. Replacement usage: `a.combine(&b, |a, b| ...)` becomes
  `ReadSignal::combine((&a, &b), |(a, b)| ...)` with the body unchanged; a
  chain building a tuple becomes one call, `ReadSignal::combine((&a, &b,
  &c), |(a, b, c)| ...)`; `.read_only()` on an input is no longer needed;
  an annotated closure names the tuple, `|values: &(A, B)|`. rustc's
  suggestion for the old call, `ReadSignal::<T>::combine(&b, f)`, is not the
  replacement; the error that follows it names the tuple form. Its rules
  are today's: `ForeignRuntime` for an input of another runtime, checked
  before anything is read; at most one evaluation per batch of input writes
  over one derived node; an equal result advances no revision and updates
  no component that reads it, while a binding derived from it still
  evaluates again (a clarification of D10's "equal results invalidate
  nothing", with the algorithm unchanged); every input and the function
  retained for the result's lifetime and released with its last clone.
- Added (R2-3): `CombineInput` (`Value`), implemented for `Signal<T>`,
  `ReadSignal<T>` and `&B`, and `CombineInputs` (`Values`), implemented for
  tuples of 2 to 12 inputs; sealed (by `signal::sealed::CombineInputSeal`
  and `CombineInputsSeal`, which the listing shows as supertrait paths), at
  the crate root and not in the prelude (an application writes a tuple, not
  the trait), with compiler messages for a tuple outside the limit and an
  element that is not a binding.
- Changed: the file browser, the tool-window menus and the `inventory`
  example derive with one `combine` where they chained two (no behavior
  change); every other caller in the runtime, the examples, the gallery,
  the guide and the facade's tests is migrated. `map` and `combine` share
  one private constructor of the derived node; `map`'s behavior is
  unchanged.
- Documentation (R2-3): `combine`'s rustdoc states the input forms, the
  limit, the function's argument, batching, equal results, retention,
  repeated inputs and its errors in order, with a settings form's dirty
  flag as its example (it had none); `CombineInput` and `CombineInputs`
  have runnable examples, a 12-tuple and a generic caller. `map`'s
  `# Errors` names `Closed` and the phase errors it already returned, and a
  write refused inside the function and a later evaluation's errors move to
  a paragraph of their own (same kind). `Signal`'s page; the prelude pages
  (the runtime's and the facade's); `docs/contracts.md`'s evaluation class;
  `guide/state.md` (a paragraph on derivations from several bindings);
  `plans/follow-ups.md`'s "Writable model projection" records round 2's
  evidence.
- Tests: the facade's `foundation` binary adds `combine` (five inputs
  evaluated once per batch, an equal result updating no component; inputs and function
  retained and released with the last result; `ForeignRuntime` before
  anything runs, also for a dirty derived input; a write inside the
  function refused at every evaluation; an update may combine and a paint
  may not; `ForeignRuntime` before `Closed`; what an equal result stops;
  every input form and a repeated input; the prelude alone). Each new
  behavior test was shown to fail under its named mutation, at an assertion
  of its outcome. `CombineInputs` has `compile_fail` doctests for a 13-tuple
  and a 1-tuple, each shown to compile, and so fail, with that arity added.
  The diagnostics cases `d01_combine_single_input` and
  `d02_combine_not_a_binding` snapshot the new messages; no other snapshot
  changed.
- Fixture: `plans/usability/task4-app` uses the tuple form and requires
  `pert-ui` 0.3.85.
- API listings: `api/pert-ui-runtime/default.txt` and `serde.txt` remove the
  method `ReadSignal<T>::combine<U, V>(&self, …)` and add the associated
  function `ReadSignal<T>::combine<I: CombineInputs>` with its impl block,
  at each of `ReadSignal`'s listing paths (the root, `component` and
  `prelude`), where `ReadSignal` and `Signal` also list their
  `CombineInput` impls with `Value`; the traits `CombineInput` with `Value` and its impls for
  `Signal<T>`, `ReadSignal<T>` and `&B`, and `CombineInputs` with `Values`
  and its impls for the tuples `(A, B)` to `(A, …, L)`. The facade's
  listings re-export the runtime root by one glob line and do not change.

## 0.3.84 (2026-10-28)

Notifications in inspection (round 2 fix plan, part P9, R2-20 by decision
D6 and R2-25 by decision D12; no design checkpoint, since P8 fixed the item
shape and inspection schema 16, and D6 was approved without separate items
for a toast's action and Dismiss controls).

- Added (R2-20): a `ToastHost`'s stack reports one `SemanticItem` per
  visible toast, in display order: named by its message's plain text as
  shown, with `SemanticAction::Activate` when the toast has an action and
  `SemanticAction::Dismiss`, `selected` on the toast the stack's keys act
  on, and `SemanticItem::bounds` of the whole toast's visible part, from the
  rectangle the stack's last arrangement gave it. The stack's new
  `action_labels` property lists the action labels in the items' order, with
  empty text for a toast without an action. A bound message's change is in
  the next inspection after the stack's update; an expired, dismissed or
  activated toast leaves the items. The stack keeps its role, value,
  `Dismiss` action and other properties. Not breaking: the stack's semantics
  gain items, inspection schema 16 is unchanged.
- Documentation (R2-20): `ToastHost`'s new section "Inspection and tests"
  describes the items, with a `Driver` example that reads them and
  activates a toast's action with Tab, Down and Enter, and states two
  limitations: content search does not find toast messages, and an item's
  rectangle is the whole toast, not its controls, so a press at its center
  (`Driver::click_item`) is not a way to activate or dismiss it.
  `SemanticItem` and `Driver::click_item` name the toasts among the built-in
  components that give item bounds; `docs/contracts.md` ("Content search")
  says content search does not read semantic items; `docs/manual.md`'s
  `ToastHost` row.
- Documentation (R2-25): `Toast`, `ToastController::push` and `ToastHost`
  point to `Banner` for a notification that stays, since every toast
  expires.
- Tests: the facade's `surfaces` binary adds `toast_items` (two toasts, one
  with an action: names, actions, selection, order, bounds and
  `action_labels`; a bound message change, also in the toast's height; an
  expired toast leaving the items; Tab, Down, Enter and Delete through the
  stack's focus). Each was shown to fail under its named mutation, at an
  assertion of its outcome.
- TODO.md: content search does not read semantic items (D6).
- API listings: no change; the items and the property are behavior of
  existing items.

## 0.3.83 (2026-10-28)

Tables in tests (round 2 fix plan, part P8, R2-22, R2-23 and R2-27 by
decision D8, from its approved design checkpoint
`plans/usability/round-2-design-P8.md`, revision 5, with decisions P8-D1 to
P8-D4).

- Added (R2-27): `SemanticItem::local_bounds` and `SemanticItem::bounds`: a
  component gives an item a rectangle in its own coordinates and an
  inspection reports its visible part in window coordinates, following the
  view scale, clips, promoted surfaces and detached windows; the table's
  header titles report theirs. `Driver::click_item`, with
  `TestError::NoItem` (`key`, `index`, `count`) and
  `TestError::ItemWithoutBounds` (`key`, `index`). Item bounds are absent
  before a component's first layout, while it is not presented, when nothing
  of the item is visible, after a failed layout that arranged the component
  and while its committed geometry is from another window; a value a
  component assigns to `bounds` directly is replaced.
- Added (R2-23): `ValidationHints::fixed_height` and
  `ValidationCode::ExceedsFixedHeight`: development validation reports a
  table cell's content that measured or is laid out taller than its row,
  such as a checkbox, a text field or an editor in a row lower than
  ControlHeight, once per mounted cell, naming the content and the table
  component that gives it the row's height (the cell, or the wrapper keyed
  `content` or `cell editor` around content keyed like the editor's host and
  around an open edit's editor). The measurement is the one the parent
  obtained during layout, kept in f64 at the component's scale; a
  measurement that changes while the child keeps its rectangle is recorded
  after the next successful layout of its window. An external grid sets the
  same hint.
- Added (R2-23): `ArrangeContext::refresh_forwarded`, which measures a child
  again under the space the arranging component's own `measure` was last
  given: the constraints it was last measured under, within its resolved
  size limits. A fixed-height component that forwards its child's measurement calls
  it in `arrange`, so a measurement the runtime answers from its retained
  results, such as a column returning to an earlier width, leaves no stale
  child height for `ExceedsFixedHeight`; it measures nothing in release
  builds. The table's wrappers around content keyed like the editor's host
  and around an open edit's editor use it, and an external wrapper can do
  the same.
- Breaking (R2-22): `TestError::NotFound` gains `resolved`, the longest
  prefix of the path that resolves (a prefix `Runtime::find` refuses as
  ambiguous counts), and its message says how far the path resolves.
  Replacement usage: match `TestError::NotFound { path, .. }`; a test that
  builds the value adds `resolved`.
- Breaking (R2-27, persistence): inspection schema 16
  (`InspectionSnapshot::SCHEMA_VERSION`); deserializing an
  `InspectionSnapshot` or `QueryResult` of another revision fails with
  "inspection schema version N is not supported (this build writes 16)",
  `schema_version` is a snapshot's first field, and
  `ComponentInspection::view_scale` is the scale of the committed layout the
  record's geometry is from (before the first layout, and while the
  committed geometry is from another window, the scale the component will be
  laid out at). Replacement usage: capture snapshots again with this
  runtime; a tool that reads several revisions compares `schema_version`
  before decoding. `Workspace::SCHEMA_VERSION` stays 2.
- Breaking: `ValidationHints` has a new public field. Replacement usage:
  build it from `..ValidationHints::default()`, as its page says.
- Changed (same kind): after a failed layout, the next layout arranges again
  every component the failed one arranged, so their painting and hit testing
  no longer keep the failed pass's retained geometry when a later pass would
  skip them; a restored workspace resolves the table's columns once it
  succeeds, so nothing pairs the restored order with the previous widths
  (a rejected restore keeps the resolved columns and the scroll offset);
  after a detach or redock, inspection reports no `border_box` or clip for a
  moved component until its new window's layout commits it, and development
  validation's record after a component's update does the same; a changed
  view scale is staged at the next layout even when the window bounds do
  not change, as for a component of no size in a continuing zoom.
- Changed (R2-23): `inventory`'s table takes its row height from
  ControlHeight, which its checkboxes and editors measure, and the gallery's
  from the largest ControlHeight of the densities it offers (40 px with the
  default metrics), since a mounted table keeps its row height across a
  density change. The `Table` editing example takes its row height from
  ControlHeight.
- Documentation (R2-22, R2-23, R2-27): the cell address on `Table` (a new
  section "Addresses and tests" with the header items, `click_item` and the
  keyboard route), `TableItems` and `TableItems::cell`, and on
  `Runtime::find_descendant`; the row height on `Table` and `Table::new`
  (with the metric lookup); `Driver::find`'s resolved prefix; the `testing`
  module page; `Component::inspect`, `BoxModelInspection::border_box`,
  `ValidationHints::clip` and `Detail::Semantics`; the testing chapter's new
  "Tables" section; the diagnostics chapter's decoder check;
  `docs/contracts.md` ("Inspection and diagnostics", "Inspection in custom
  components"), `docs/manual.md` and the native evidence record.
- Tests: the runtime's `testing` binary covers the resolved prefix and
  `click_item`'s refusals; its `validation` binary covers a custom
  fixed-height parent, the hint alone, the measurement alone and a
  measurement queued across a detach and a redock, and the largest finite
  measured height at a view scale of 2; its unit tests cover the
  queue's move and unmount cleanup, idle flushes and a rejected restore. The
  facade's `collections` binary adds `table_items` (header bounds and
  sorting, horizontal clipping, a view scale, a title out of view, refusals
  before and after a scrolling reveal, before the first, pending and failed
  layouts, a hidden table, a promoted surface and a detached window, a
  restore, a reparent and a failed zoom, moves before a layout, a zero-size
  component in a continuing zoom, the cell address, the row-height reports
  and width cycles through wide and narrow columns for wrapped content and an
  editor); `foundation` covers an external forwarding wrapper through both
  width cycles and with a maximum and a minimum width, custom item rectangles and the schema-16
  envelopes, including unknown fields in both; the gallery tests the Note
  editor after a density switch. Each new-behavior test was shown to fail
  under its named mutation, at an assertion of its outcome.
- API listings: `api/pert-ui-runtime/default.txt` and `serde.txt` add
  `SemanticItem::bounds`, `SemanticItem::local_bounds`,
  `ValidationHints::fixed_height`, `ValidationCode::ExceedsFixedHeight`,
  `ArrangeContext::refresh_forwarded`, `Driver::click_item`,
  `TestError::NoItem` and `TestError::ItemWithoutBounds` with their fields,
  and
  `TestError::NotFound::resolved`, each at every path the listing shows; the
  `api/pert-ui/*.txt` listings show the runtime's items as re-exports and do
  not change, and no listing shows `SCHEMA_VERSION`'s value or the
  snapshot's field order.

## 0.3.82 (2026-10-28)

Rows that shrink (round 2 fix plan, part P7, R2-21 by decision D7, from its
approved design checkpoint `plans/usability/round-2-design-P7.md`,
revision 5, with decision P7-D1).

- Breaking (R2-21, behavior): a grown child of a row whose width is bounded
  now gives up width when the children's natural widths do not fit, by
  weight and down to its minimum, as a grown child of a bounded column gives
  up height since 0.3.25. The row measures each child it shrank again at its
  final width, so a wrapping label gets its new height and the row grows to
  it, and an ellipsis label truncates; a grown horizontal collection in a
  bounded row is now bounded. The row overflows only past its ungrown
  children's natural widths plus its grown children's minimums, where any
  fill child the row did not shrink, grown or not, counts with its width at
  its fill height instead; development validation still reports that
  overflow as `EscapesParent`. Whether the row shrinks, and by how much, is
  decided once from the natural widths; its reported width, overflow and
  justified spare space are computed from the total of the final
  allocations in f64, so f32 cancellation on huge widths cannot move them,
  and a child counts as shrunk, and is measured again at its final width,
  only when that width as allocated is smaller than its natural one (a
  share that rounds away leaves it unshrunk). An
  unshrunk fill child is allocated the width it reports at its fill height;
  a shrunk one keeps its final width. Growth when the natural widths fit
  works as before; after a shrink no child grows back. Rows that fit, rows
  without a growing child and rows offered an unbounded width are
  unchanged. Replacement for a layout that relied on a grown child keeping
  its natural width: drop its growth weight, or keep a minimum with
  `Container::new(child).size_limits(SizeLimits::at_least(..))` or the
  component's own `size_limits`. Rows and columns share one crate-private
  shortfall computation; a column's results are unchanged.
- Changed (P7-D1): each component keeps its four most recently used
  measurements instead of one, so a row that measures a shrunk child at its
  natural and its final width, measured and arranged at one height or two,
  reuses them; more than four constraints are measured again, with the same
  result. `Component::measure`, `Component::fills_row_height`,
  `MeasureContext::measure` and `ArrangeContext::measure` say so.
- Fixed (same kind, P7-D1): a component's theme and font dependencies from
  its measurements stay recorded until its layout is invalidated. Before,
  they were reset before each new measurement, so an ancestor's cached
  measurement computed from an earlier result of the component could miss a
  metric or font change that only that result had read. The theme and font
  notifiers now judge every subscriber's reads before invalidating any, since
  a layout invalidation clears the measurement reads of a component and its
  ancestors; a runtime test covers both visiting orders for metrics and
  fonts.
- Documentation (R2-21): `Element::with_grow` (with a row example), `Row`,
  `ProgressBar` (a grown bar beside its percentage), `VirtualList`'s
  bounded extent and the `UnboundedCollection` message, `docs/contracts.md`,
  `docs/manual.md`, and guide/layout.md: "Growing, shrinking and stretching
  rows and columns", the collections paragraph and the new "A progress
  column".
- Gallery: the Collections table's Progress column, a growing bar beside its
  percentage; the Layout page's growth stage is taller and says what a
  narrow window shows. The gallery README says both.
- Tests (R2-21, P7-D1): the facade's `foundation` binary replaces
  `rows_keep_their_lengths`, which encoded the old rule, with the row's
  weights and minimums (the column test transposed) and adds a grown
  horizontal list bounded by a row, the bar beside its percentage in a
  167 px cell, a shrunk child measured again (and the row's uncapped
  tallest-child height), a fill child following the wrapped height and
  counting with its fill height, an ellipsis label, residual overflow
  reported for ungrown children and for a grown minimum, fill children wider
  and narrower at their fill height with and without a shortfall, the
  measurement counts of nested shrinking rows, a fitting row, a shrunk fill
  child, a row in a column redescribed with a new `Justify` and more than
  four constraints, metric and font dependencies of an evicted
  measurement, the f64 total of a shrinking row's final widths (its
  justified spare space and reported width at 100,000,000 px) and a share
  that rounds away for an ordinary and a fill child; the runtime adds unit
  tests of the store and of the sticky dependency bits; the gallery tests
  the Progress column. Each
  new-behavior test was shown to fail under its named mutation.
- No public item is added, removed or changed: the `api/` listings do not
  change.

## 0.3.81 (2026-10-28)

Display controls take values (round 2 fix plan, part P6, F16 by decision D1
and R2-26 by decision D13, from its approved design checkpoint
`plans/usability/round-2-design-P6.md`, revision 3, with decision P6-D1).

- Breaking (F16): `ProgressBar::new(ProgressStatus)` shows a described
  status, which applies when the description is adopted (at mount or a
  same-key reconcile) and changes only with a new description; a described
  indeterminate status sweeps as a bound one does. `ProgressBar::bound`
  (`impl Into<ReadSignal<ProgressStatus>>`) follows a binding, as `new` did.
  Replacement usage: `ProgressBar::new(status.read_only())` becomes
  `ProgressBar::bound(status.read_only())`, or
  `ProgressBar::new(record.status)` in a collection row instead of a signal
  per row.
- Breaking (F16): `Spinner::new(bool)` and `Spinner::bound(impl
  Into<ReadSignal<bool>>)`, with the same rules; a described busy spinner
  steps every 100 ms on the runtime clock. Replacement usage:
  `Spinner::bound(busy.read_only())`, or `Spinner::new(record.running)`.
- Breaking (F16): `Field::status(FieldStatus)` shows a described status and
  `Field::status_signal(impl Into<ReadSignal<FieldStatus>>)` follows a
  binding; `Property::status(FieldStatus)` and
  `Property::status_signal(..)` delegate to the row's field. `status`,
  `status_signal` and `form` each replace the other two, and `status` and
  `status_signal` clear the form target, so `FormFields::focus_first_error`
  no longer reaches that field. Reconciliation compares sources, not shown
  statuses: a binding replaced by an equal fixed status no longer drives the
  field. Replacement usage: `.status(signal.read_only())` becomes
  `.status_signal(signal.read_only())`, or `.status(FieldStatus::error(..))`
  for a fixed status.
- Breaking (F16): `InlineReadout::new` takes `impl Into<TextSource>`: fixed
  or bound text, plain or rich (shown as its plain text), or a
  `StringRole`. Fixed text applies when the description is adopted, checked
  together with the description's literal unit (`unit`); a text or unit
  that cannot be shown refuses both there, keeping the last valid text and
  unit, until a valid description replaces them (the next update returns
  the `TextEditError`); a binding or role is read in the update, and a
  same-key description with the same binding or role keeps the text read.
  Every argument the library converted before still converts; a
  caller-defined conversion into `ReadSignal<String>` is replaced by
  passing the `ReadSignal<String>`.
- Breaking (R2-26): `ProgressStatus::Failed(Option<NormalizedValue>)` and
  `ProgressStatus::Cancelled(Option<NormalizedValue>)`. `None` is the full
  track in the Error or Muted role as before; `Some(reached)` fills the
  track to `reached` (with the gap beside a partial fill; zero leaves the
  track empty) and reports the fraction as the semantic value and the
  `fraction` property. `invalid` stays set for every failure and never for
  a cancellation. Replacement usage: `ProgressStatus::Failed(None)` for the
  old full track, `ProgressStatus::Failed(last.fraction())` to keep the
  fraction reached; patterns `Failed(_)` and `Cancelled(_)`.
- Added (R2-26): `ProgressStatus::fraction`, the fraction a status carries
  (`Determinate`, and `Failed` or `Cancelled` with `Some`), so a
  prelude-only program writes `Failed(last.fraction())` without naming
  `NormalizedValue` (decision P6-D1).
- Fixed: an `InlineReadout` whose bound or formatted text is refused (over
  `TextEditError::MAX_TEXT_BYTES` or holding a control character) keeps its
  last valid text in inspection and its semantic value, keeps its unit and
  stays in text mode; it no longer stores the refused text and ends text
  mode before validating it (the same kind as P6's fixed-text rule, in
  touched code). A same-key description with the same binding no longer
  repaints the readout.
- Documentation (F16): `docs/contracts.md` states the display-control rule
  under "Ownership paths"; `ListItems::build` gains a "Described" way and
  narrows the per-row and retained ways to editing controls, as do
  `TableItems::cell` and `Signal::sibling`; guide/state.md's table gains a
  row for a value at construction; the editable-list recipe in
  guide/layout.md gains a progress column described from each record, with
  a test of a written record; guide/controls.md, `docs/manual.md`, the
  types' pages (`ProgressStatus`, `ProgressBar`, `Spinner`,
  `InlineReadout`, `Field`, `Property`, `TextSource`'s list of takers) and
  the prelude pages, whose `NormalizedValue` bullet now names
  `ProgressStatus::fraction`; `plans/follow-ups.md` records round 2's
  evidence for controlled forms of more controls.
- Migrated: the gallery's progress page describes its fixed statuses and
  gains "Failed at three of eight" and "Cancelled at three of eight", and
  its Fail and Cancel end the live task at the fraction it reached; the
  text page's fixed field statuses are described, retiring its `fixed`
  helper; `log_explorer` binds its bar with `bound` and keeps the fraction
  a failed load reached; the runtime's `quiet_for` example and driver tests,
  and the facade's tests, use `bound`, `status_signal` or the fixed forms
  as the design's manifest says.
- Tests (F16, R2-26): the facade's `collections` binary gains
  `display_rows` (a table republished through `TableController::set_source`
  whose cells describe a bar and a spinner per record, and a `VirtualList`
  row remounted by a scroll); `controls` covers every change of source for
  the bar and the spinner through `check_redescription`, with the dropped
  binding after bound to fixed, fixed values before any flush, fixed,
  rich, rebound and role readout text, refused fixed, bound and formatted
  text, a fixed readout's unit that cannot be shown and its recovery (also
  from the first mount and by a corrected unit alone), the old readout
  conversions, a described field status, the builder
  replacement rules and a form replaced by a fixed status; `presentation`
  covers both terminal statuses at zero, three eighths and full, a
  described indeterminate bar and a described busy spinner, and the
  background-contrast test adds the partial terminal fills; `collections`
  covers a property row's fixed status; the prelude fixture writes each
  new form without a `parameter` import. Each new-behavior test was shown
  to fail under its named mutation.
- API listings: `pert-ui-runtime` (`default.txt` and `serde.txt`) changes
  `ProgressBar::new`, `Spinner::new`, `InlineReadout::new`, `Field::status`
  and `Property::status`, and `ProgressStatus::Failed` and
  `ProgressStatus::Cancelled` (now tuple variants, listed at the root and in
  the prelude), and adds `ProgressBar::bound`, `Spinner::bound`,
  `Field::status_signal`, `Property::status_signal` and
  `ProgressStatus::fraction`. The facade's listings do not change: they
  re-export the runtime by glob.

## 0.3.80 (2026-10-28)

Rows in display order (round 2 fix plan, part P5, F9 and R2-6 by decision
D4 and its sub-decision on `Each::new`'s placeholder key, from its approved
design checkpoint `plans/usability/round-2-design-P5.md`, revision 3).

- Added (F9): `Driver::address_children` and `AddressChild` (`segment`,
  `id`; `#[non_exhaustive]`, `Clone`, `Debug`, `Eq`, `PartialEq`), in
  `pert_ui_runtime::testing` and `pert_ui::testing`: the components one
  address segment below a parent, in child order, each as `Runtime::find`
  resolves it (decision P5-D1). A component is listed with segment `s`
  exactly when `find(parent, &[s])` matches it, so an `Each`'s rows come
  without its default container, an `Each` with `within` lists its
  container, and a `VirtualList`'s and a `Table`'s mounted rows come in
  index order, offscreen retained rows included and unmounted rows left to
  the collection's controller. A part is listed under its name and its
  descendants are listed too; two components with one segment are both
  listed, and `find` refuses that address. The walk shares `find`'s
  per-component classification, so the two cannot drift.
- Breaking (TODO.md, `Each::new`'s placeholder key): `Each::new`'s `build`
  returns any `EachItem` (`pub fn new<R: EachItem>(.., build: impl
  Fn(&DescribeContext<'_>, &T) -> R + 'static)`): a component, which `Each`
  describes under the item's key, exactly as `component.key(item_key)`
  would, or an `Element`, whose key it replaces as before. Every closure that
  returned an `Element` compiles unchanged; one whose return type was
  inferred only from the expected `Element`, such as `|_, item|
  make(item).into()`, needs its type named (`-> Element`). Replacement
  usage: return the component, `|_, task| Label::new(task.as_str())`,
  without `.key(..)`; return `Element`s for items of different component
  types or for an item that needs an element builder.
- Added: `EachItem`, at the crate roots, sealed with an uncallable
  conversion (decision P5-D2) and implemented for every `Component` and for
  `Element`. A `build` returning anything else reports "`T` cannot be an
  `Each` item" with a note naming the two kinds.
- Documentation (R2-6): `Runtime::children` says that it lists structural
  wrappers, such as an `Each`'s default container and a `Table`'s body, and
  a virtualized collection's mounted rows in index order, and points to the
  driver for the address children; `Runtime::key` says that a structural
  wrapper's key is no address segment and that a slot's or part's segment
  is its name; `VirtualList` and `Table` say what their children are, and
  `Selector::Children` and `Selector::children` that their results are in
  incarnation order and match absolute paths, so a part's descendants are
  not the part's children and two components with one path pool theirs.
  The testing chapter's "List rows" reads an `Each`, an `Each` with
  `within`, a `VirtualList` with a revealed row and a `Table` in order, and
  its introduction says the driver lists rows in order. `Each`, `Each::new`,
  the layout chapter, `docs/contracts.md` and `docs/manual.md` describe an
  item as a component; the prelude pages name `EachItem` beside
  `DescribeContext` as inferred; the `testing` pages list `AddressChild`.
- Migrated: the placeholder keys in `Each`'s rustdoc, the guide, the
  gallery's tag list, the facade's tests and the task 4 fixture
  (`plans/usability/task4-app`, which now requires 0.3.80) are gone; the
  tests whose subject is the `Element` path keep their elements.
- Tests (F9, R2-6): the runtime's driver tests cover an `Each` after a
  reorder, an `Each` with `within`, a reordered and a scrolled
  `VirtualList` (focus, overscan, `keep_mounted`, a revealed row), a
  `Table`, a structural parent, a part, a shared segment, a slot, the two
  cases where `Selector::children` differs, a leaf and refused parents,
  hidden and detached children, a structural row and a drag preview. The
  facade's `foundation` binary covers a bare component as an item, mixed
  `Element` items and a duplicate key with bare components, written with
  the prelude alone; `compile_fail` doctests on `EachItem` cover an
  external implementation, a `String` item and a generic call of the
  conversion without and with a token; the diagnostics snapshots gain
  `e01_each_item_not_component`. TODO.md gains the same placeholder key in
  `ListItems::build` and `TreeItems::build`, which are trait-object methods.
- API listings: `pert-ui-runtime` adds `testing::AddressChild` with its
  fields and impls, `testing::Driver::address_children`, `EachItem` with
  its two implementations (the `Element` one also listed on `Element`), and
  changes `Each::new`'s signature; every `pert-ui` listing adds
  `pub use pert_ui::testing::AddressChild`, and `EachItem` reaches the
  facade's root through its glob re-export.

## 0.3.79 (2026-10-28)

Focus and idle in the driver (round 2 fix plan, part P4, R2-11 and R2-24 by
decision D5, from its approved design checkpoint
`plans/usability/round-2-design-P4.md`, revision 3). Nothing breaks:
`TestError` is `#[non_exhaustive]`, so its new variant changes no match.

- Added (R2-11): `Driver::clear_focus`, the selected window's deactivation
  (`HostIntegration::clear_focus`) followed by a settle, for "nothing
  focused" in a test. A focused editor applies its focus-loss rules, so a
  valid draft is saved; held keys and reported modifiers are released, so
  it needs no neutral input and leaves input neutral. Its rustdoc states the
  two reaches: focus, surfaces and pointer state are the selected window's,
  while pressed keys, modifiers and every admitted held-input registration
  are cleared runtime-wide.
- Added (R2-24): `Driver::quiet_for` and `QuietReport` (`components_updated`,
  `next_deadline`, `needs_paint`, `baseline_painted`, `is_idle`), in
  `pert_ui_runtime::testing` and `pert_ui::testing`. It flushes at the
  current clock and paints a baseline, then lets the interval pass in
  `Driver::FRAME` steps without painting, a final partial frame landing
  exactly on its end and a zero interval making no step, so the clock moves
  by exactly the interval. The report covers the selected window and the
  work windows share; another window's own timers do not make it busy, and
  a background worker still computing is not observed until its result
  arrives, so a test of background work calls `Driver::wait_until` first. A
  baseline the runtime refuses because the baseline flush left work queued
  is activity (`baseline_painted: false`, never idle), told apart from a
  component's own `DrawError::PendingUpdates` by `Runtime::last_failure`.
  `QuietReport` is `#[must_use]`, so `driver.quiet_for(d)?;` warns.
- Added (R2-24): `TestError::Draw(DrawError)`, which `quiet_for`'s baseline
  paint returns, and `impl From<DrawError> for TestError`, which keeps a
  runtime failure in its one form: `DrawError::Runtime` becomes
  `TestError::Runtime`, every other variant `TestError::Draw`, so a test
  helper returning `TestError` applies `?` to `Runtime::paint`. `DrawError`
  joins both `testing` modules (decision P4-D1) and stays at the roots, in
  `component` and in `host`.
- Documentation (R2-11, R2-24): the testing chapter's "Nothing focused" and
  "Nothing scheduled while idle", and its introduction;
  `HostIntegration::clear_focus` and `HostIntegration::needs_paint` point
  to the driver, and `needs_paint` says that a capture waiting for its
  window keeps it true and that a queued command does not;
  `Runtime::next_deadline` states its scope (the selected window's deadlines
  and timers, the work windows share, the current time while work waits,
  including an arrived background result but not a worker still computing,
  and `HostIntegration::application_deadline` for every window); the
  `testing` and `host` pages and the trait's page no longer teach clearing
  focus through the trait, and the facade's `testing` page drops its
  `focus`/`clear_focus` lines; `Driver`'s "Settling" points to
  `quiet_for`; `TestError::ClockOverflow` names the driver's other callers;
  `DecimalInput`'s example clears focus with the driver. The `testing` pages
  list `VectorId`, `MaskId` and `ImageId`, which `DrawError`'s variants
  hold, among the names in `component`.
- Tests (R2-11, R2-24): the runtime's driver tests cover a draft saved by
  `clear_focus`, its returned focus-loss error, held input and its two
  reaches; a spinner, a timer the test installed, a toast waiting to expire,
  a spinner in another window, baseline work, the exact clock, paint
  failures in both forms, a refused baseline at a zero and a nonzero
  interval, a component's own `PendingUpdates`, the overflow check, a
  step's flush error, the uncounted nested flushes of a queued command's
  admission and of a queued reveal into another window, and a worker
  blocked on a channel, which leaves the report idle until the test waits
  for its result. A `compile_fail`
  doctest under `#![deny(unused_must_use)]` discards a `QuietReport` after
  `?`, beside one that reads it. The facade's module tests name
  `QuietReport` and `DrawError` in `testing`, and the closure check lists
  `VectorId`, `MaskId` and `ImageId` as reviewed exclusions (in
  `component`).

The `api/` diff: in `api/pert-ui-runtime/default.txt` and `serde.txt`,
`Driver::clear_focus`, `Driver::quiet_for`, the item block of
`pert_ui_runtime::testing::QuietReport` (its four fields, `is_idle` and its
derived and auto trait impls), `TestError::Draw`,
`impl From<DrawError> for TestError` (listed in the blocks of `TestError`,
`DrawError` and each module that exports `DrawError`) and the item block of
`pert_ui_runtime::testing::DrawError`; in every `api/pert-ui/*.txt`,
`pub use pert_ui::testing::DrawError` and
`pub use pert_ui::testing::QuietReport`.

## 0.3.78 (2026-10-28)

Small ergonomics (round 2 fix plan, part P3, R2-4 by decision D11 and
R2-17; no design checkpoint, D11 settles the shape).

- Breaking (R2-4): `EventContext::defer_while_shown` returns
  `Result<ShownTaskReceipt, RuntimeError>` instead of
  `Result<TaskReceipt, RuntimeError>`. Replacement usage: name the new type
  where the receipt is stored, such as
  `Rc<RefCell<Option<ShownTaskReceipt>>>`; it has the same `status` and
  `status_binding` over `TaskStatus`, `Clone`, `Eq` and `Debug`. Code that
  only calls `status()` on the result needs no change.
- Added (R2-4): `ShownTaskReceipt`, at the crate root and in `component`,
  the `#[must_use]` receipt of `defer_while_shown`, the one deferral that
  hiding or removing its component also cancels, which nothing else
  reports. The attribute is on the type, so `cx.defer_while_shown(...)?;`
  with the receipt discarded still warns. Not in the prelude: L2's closure
  treats `EventContext` as a hub whose methods' types belong to their
  families, and `TaskReceipt` is not there either.
- Changed (R2-4): `TaskReceipt`, returned by `EventContext::defer`,
  `EventContext::spawn` and `Runtime::defer`, is no longer `#[must_use]`:
  only shutdown cancels those before delivery, and a panic cancels the
  operation whose callback unwinds (operations queued behind it stay
  queued); failures propagate from the flush. `cx.defer(...)?;` and
  `cx.spawn(...)?;` compile without a warning, and `let _receipt` left the
  rustdoc examples, the guide (`state.md`, `layout.md`), the packaged
  examples (`dynamic_documents`, `diagram_editor`), the gallery and the
  facade's tests. `docs/contracts.md`'s receipt rule and deferral section,
  the rustdoc of both receipts and the three deferral methods, and the API
  audit's G2-8 record (an addendum) say which receipt must be used and why.
- Added (R2-17): `impl From<u16> for Spacing`, as `Spacing::Fixed`, matching
  `Length`, so `.padding(24)`, `.gap(8)` and `Divider::spacing(4)` compile.
  An integer literal stays unambiguous, since `u16` is the only integer type
  `Spacing` converts from. `Spacing`, `Container::padding` and
  `Group::padding` say so, with a `.padding(24)` example.
- Changed: `CaptureError::Unsupported`'s message names
  `Runtime::set_capture_support(true)` and, for a test, the CPU renderer's
  `CpuRenderer::serve_captures` (the `cpu` feature); a unit test pins it.
  Moved from P1 (a round 2 tester error, 5-r1-claude-2#7).
- Tests: the facade's `foundation` binary has a module under
  `#![deny(unused_must_use)]` that queues with `cx.defer(...)?;`,
  `cx.spawn(...)?;` and `ui.defer(...)?;` unbound; a `compile_fail` doctest
  on `ShownTaskReceipt` denies `cx.defer_while_shown(...)?;` under the same
  lint; the receipt contract test covers `ShownTaskReceipt`; the facade
  tests share a `Receipt` helper for code that queues with every call.

The `api/` diff: in `api/pert-ui-runtime/default.txt` and `serde.txt`,
`EventContext::defer_while_shown`'s return type, the item blocks of
`pert_ui_runtime::ShownTaskReceipt` and
`pert_ui_runtime::component::ShownTaskReceipt`, and
`impl From<u16> for Spacing`. The facade's listings do not change: they
list its root and `component` globs, not the runtime's items. The listings
record no `#[must_use]` attributes, so the tests above check their
placement.

## 0.3.77 (2026-10-28)

Glob membership (round 2 fix plan, part P2, F3 by decision D3 and the
TODO.md entry on the facade's `svg` module, from its approved design
checkpoint `plans/usability/round-2-design-P2.md`, revision 3).

- Breaking (TODO.md svg collision): the facade module `pert_ui::svg` is
  `pert_ui::svg_images`, so a crate that globs the crate root and depends on
  the `svg` crate no longer finds `svg` ambiguous in a `use` path or
  resolves `svg::…` to the module elsewhere. Replacement usage:
  `use pert_ui::svg_images::{SvgError, intrinsic_size, rasterize,
  rasterize_mask};` and `svg_images::MAX_NESTING`; the `svg` feature and
  `pert_ui::Svg` are unchanged, and `Svg::new` and `Svg::tinted` return
  `pert_ui::svg_images::SvgError`.
- Added (F3): to `pert_ui::testing` and `pert_ui_runtime::testing`,
  `FlushReport` and `RevealRefusal` (what `Driver::settle`, `Driver::advance`
  and `TestError::NotRevealed` hand a test); `Semantics`, `SemanticRole`,
  `SemanticAction` and `SemanticItem`; `QueryResult`, `ComponentInspection`,
  `ComponentDetails`, `InspectValue` and `Depth`; `ValidationReport`,
  `ValidationIssue`, `ValidationCode` and `ValidationMarker` (development
  validation, decision P2-D2); `Capture`, `CaptureError`, `Pending`,
  `InspectionReply`, `WaitError`, `NativeWindowId` and `Rect`; and
  `Magnification`. With `cpu`, `CpuRenderError` joins `pert_ui::testing`
  beside `CpuRenderer`. A test that names what the driver, a query, the
  validation report or a capture hands it needs only `prelude` and
  `testing`. Each name stays where it was, so every existing path keeps
  working.
- Added (F3): `ThemeMode` to `pert_ui::prelude` and
  `pert_ui_runtime::prelude`, the one exception to the layering plan's
  decision D2 (L2): a theme switch is first-program code. `ThemeManager`,
  `Palette` and the other theme services stay at the crate root.
- Documentation (F3): the `testing` pages and the testing chapter state the
  module's closure rule (every type its names take, return or hold is in
  `testing` or the prelude) and list where the excluded records are; the
  first example of each `testing` page and the chapter's "Reading what the
  runtime shows" write helpers that return `Semantics` and read an
  `InspectValue` from a `QueryResult`. The facade's `testing` page now names
  `CpuRenderer`, `CpuRenderError` (in `cpu`) and `HeadlessApp` (only in
  `testing`) beside `HostIntegration` as the names not at the crate root.
  The prelude pages list `ThemeMode`, their testing row adds semantics,
  query results and validation, and the root row's example is
  `use pert_ui::{Palette, Workspace};`. The `svg_images` page says why it is
  not named `svg`; the guide, the crate page, the README, the status table
  and `docs/contracts.md` use the new name. `docs/verification.md`
  describes the closure check.
- Tests (F3): `pert-ui-diagnostics` checks from the committed API listings
  that every type `testing`'s names hand a test is in `testing` or the
  prelude, apart from a reviewed exclusion list, in the runtime and the
  facade's default, `cpu`, no-default and `desktop-cpu` configurations;
  that every exclusion is reached and is no member; that the facade's
  `testing` holds every name of the runtime's; and that the `Runtime`
  methods it starts from exist. It runs in the workspace tests. The facade's
  module tests cover the `host` module (its names, and its glob combined
  with the others, which now resolves every name the modules export) and
  `HostIntegration` in `testing`, and a prelude-only program writes
  `ThemeMode`.

The `api/` diff: in `api/pert-ui-runtime/default.txt` and `serde.txt`, the
item blocks of the 23 names above under `pert_ui_runtime::testing::` and
`ThemeMode`'s under `pert_ui_runtime::prelude::`; in every
`api/pert-ui/*.txt`, the 23 `pub use pert_ui::testing::…` lines, and in
`cpu.txt` and `no-default-desktop-cpu.txt` also
`pub use pert_ui::testing::CpuRenderError`; in `svg.txt` and
`no-default-svg.txt`, `pub mod pert_ui::svg` and its items moved to
`pert_ui::svg_images`, and `Svg::new` and `Svg::tinted` returning
`pert_ui::svg_images::SvgError`.

## 0.3.76 (2026-10-28)

Documentation from the round 2 blind test (round 2 fix plan, part P1):
rustdoc and guide only, with no behavior or public API change. Each item
answers a finding the testers met (`plans/archive/developer-usability.md`,
"Round 2 results").

- R2-5 (decision D14): `Semantics::value` says which content an editor
  reports while an edit is pending. `IntegerInput` has an "Inspection"
  section: the role `NumberInput`, the saved number as
  `SemanticValue::Number` with the range also while a draft is pending, the
  draft in the `text` property, the text value when no saved number is
  available (before a valid value is read, and after a rebinding or a resting
  range change until the field accepts a value), and the inspection
  properties (`text`, `value`, `display_value`, `shows_display`, `unit`,
  `valid`, `dirty`, `validation_error`, `binding_error`, `inline`,
  `text_editing`, `stepper`), with an example; `DecimalInput` points to it.
- R2-7: `InlineText` has a section on its editing child `"editor"`: its
  address, that it stays mounted and is shown while editing, that focus and
  typing move into it, and where the draft is reported (the child's value and
  `text`, the label's `draft`). The testing chapter has an "In-place editors"
  section that edits through the child.
- R2-8: `VirtualList::on_activate` and `VirtualList::reorderable` say that
  the list consumes Enter and Alt with an arrow in ordinary bubbling, before
  `View`, `Window` and `Application` command bindings, and link
  `Runtime::command_key_input`'s routing order; `CommandScope` and each
  variant say where its bindings sit in that order.
- R2-9: `Checkbox` and `Toggle` point to `Field::hide_label` for a control
  named by text shown elsewhere.
- R2-10: `KeyChord::new` has an example of explicit Control+Alt and Alt
  chords written with `Modifiers`' fields, run through `Driver::chord`, and
  says that ASCII letters match in either case and other characters exactly.
- R2-12: the testing chapter's capture ends by writing
  `FrameImage::encode_png` to a file under `CARGO_TARGET_TMPDIR`, with the
  system temporary directory where Cargo does not set it (a doctest).
- R2-13: `Label`, `Driver::semantics` and the testing chapter say that an
  empty label has no name (`None`).
- R2-14: the custom-widgets chapter's imports say that a widget module globs
  `component`, adding `prelude` when it also uses built-in components or runs
  a runtime, and its first example globs only `component`, as the facade's
  prelude test now checks.
- R2-15: the custom-widgets chapter has "A control that edits a value": a
  complete control with live writes, Escape's restore in `keyboard_event`,
  the restore in `cancel_input`, what `on_change` reports, a failed restore
  recorded and reported through a `Notifier` while the control stays mounted
  (and the stated limit: a restore that fails during removal is not
  reported), the readings it supports (a value and a limit in 0..=1; any
  other reading, NaN included, fails the update and keeps the previous
  presentation) and the focus ring by `ColorRole::FocusRing`'s recipe. Its
  doctest runs a failed restore while mounted and one during removal,
  refuses a NaN limit, and counts the focus ring's four sides in a frame.
- R2-16: every type whose `Serialize` and `Deserialize` are behind the
  `serde` feature says so on its page, and the feature table of the crate
  page and the README links the documents it serializes.
- R2-18: the appearance chapter's examples, and the surfaces, controls and
  custom-widgets chapters' examples that did the same, import the prelude,
  the modules and named items instead of globbing the crate root. Same kind,
  fixed here: every rustdoc example of `pert-ui-runtime` and `pert-ui-core`
  that globbed the crate root, the runtime README's included (only
  `pert-ui`'s `compile_fail` example that shows the root glob lacks
  `HostIntegration` keeps it), the packaged
  `dynamic_documents` example and the gallery's modules and tests. The
  examples touched lose imports they did not use (`EventContext::to_local`,
  `VirtualList::on_selection_change`, the diagnostics chapter's query and
  four blocks of the runtime README), and
  `KeyChord::named`'s example compares it with the `KeyChord::new` form its
  text names. `IntegerInput`'s local-signal example and its wheel and Shift
  paragraph move before the page's headings, out of "Parameter binding".
- R2-19: `SizeLimits::exact`'s example builds its size with the infallible
  `Size::from([260, 260])` and says so.
- R2-28: `Pending::wait` says that on the UI thread it blocks forever unless
  the reply has already arrived, and to take the reply with `try_reply`
  there; `Runtime::request_capture` and the diagnostics chapter say the
  same, and the UI-thread examples of the diagnostics and testing chapters,
  the `cpu` module and `CpuRenderer::serve_captures` use `try_reply` after the
  flush or the serve that answers the request, and assert before it that the
  reply has not arrived (`WaitError::Timeout`).

The `api/` listings are unchanged.

## 0.3.75 (2026-10-27)

One draft policy (layering plan, part L4b, N5 in the complexity inventory,
from its approved design checkpoint `plans/usability/layering-design-L4b.md`,
revision 4). What saving, refusal, application writes, revocation, hiding,
removal, close admission and text commands do to a draft is now one set of
rules for every built-in editor, in contracts.md's new "Draft policy"
section, which replaces the per-editor lists in the permissions and close
sections. The editors that broke it follow it, and every built-in editor that
can keep a draft without focus reports it.

- Breaking (N5): revoking `enabled` on an `IntegerInput` or `DecimalInput`
  bound to a signal keeps a typed or stepped draft unsaved, focused or not, as
  `TextField` does, instead of discarding it, by a bound permission or a fixed
  `disabled()` description; with an `EditorSession` the edit stays open with
  it, and nothing is reported. Input cancellation other than hiding and
  removal (a pointer gesture's loss, a failed clipboard operation, a geometry
  change) no longer discards such a field's draft, focused or not; hiding
  still discards it. Replacement usage: `TextCommand::DiscardDraft` through
  `Runtime::text_command` or `EventContext::request_text_command` discards the
  draft without touching the binding. This makes `IntegerInput::session`'s,
  contracts.md's and `ColorPicker::enabled`'s documentation true: a picker's
  channel fields now keep their drafts as its hex field does.
- Breaking (N5): a held step button on such a field no longer discards the
  draft when editing is revoked or focus leaves: the press ends and keeps the
  steps it took; focus loss then saves as without a press, and a session field
  reports `FocusLeft` with the outcome (it reported nothing and lost the
  stepped draft). Replacement usage: Escape, or `TextCommand::DiscardDraft`,
  restores the saved value.
- Breaking (N5): an open text mode of `InlineInteger` and `InlineDecimal`
  (signal or parameter) becomes read-only when its editing permission is
  revoked, keeping its draft, selection and undo history, and editable again
  when it is allowed; it stayed editable with saves refused. Replacement
  usage: `TextCommand::DiscardDraft` ends it.
- Breaking (N5): `SearchField` writes an edit it receives without focus (a
  text command, direct or queued, including its own clipboard menu's) at
  once, as without a debounce, and discards a pending debounced query when
  focus leaves while editing is denied; it armed the debounce without focus
  and kept a refused query pending after focus left. Whether focus leaving
  writes the query follows the editing permission's current value, so a
  field allowed again just before its departure writes it. Replacement usage:
  none needed.
- Breaking (N5): a session field (`TextField`, `IntegerInput`, `DecimalInput`)
  that a focus loss reaches while disabled, between a fixed `disabled()`
  description and the next flush, reports nothing; it reported `Refused`.
  Replacement usage: none needed; the edit stays open, as after a bound
  revocation.
- Changed: a session field's `on_modified` report is requested before each
  completion it reports to its `EditorSession`, Escape included (Escape
  reported directly before), so a host that closes the edit and removes the
  field does so after the field's `false`, and an application count of unsaved
  fields returns to zero.
- Changed: `DraftEditorController::end` also releases the pointer capture the
  editor holds for a gesture the session's end discards, so a selection drag
  cut short by `end` no longer keeps routing the held button's moves to the
  editor.
- Changed: a `Table` has a second, invisible child after its body, keyed
  `edit end reporter` and structural (no workspace state), which delivers
  `on_edit_end`; it has no extent, paints nothing, takes no focus or hits and
  has no semantics. `Runtime::children` and inspection list it. The body stays
  the first child.
- Added (F6): `on_modified` on `IntegerInput`, `DecimalInput`,
  `InlineInteger`, `InlineDecimal` and `InlineText`, reporting when a draft
  starts or stops differing from the value the editor shows at rest (an
  inline row's text mode only; a value drag or held step at rest is a gesture
  and reports nothing), after settling, so a close policy and `FormFields`
  see the drafts these fields keep.
- Added: `Table::on_edit_end`, with `CellEditEnd` (`row`, `column`, `reason`)
  and `CellEditEndReason` (`Completed(EditorCompletion)`, `Cancelled`,
  `Replaced`, `RowLeft`, `Failed`, `Hidden`), reporting every cell edit's end,
  including those the table ends itself, one notification per end in the
  global request order; removing the table reports nothing, and removing the
  callback discards ends not yet delivered.
- Added: to the prelude, `CellEditEnd`, `CellEditEndReason`,
  `EditorCompletion`, `EditorOutcome`, `EditorTrigger` and `FocusDirection`,
  the names an edit-end callback matches; `CellEditEnd` and
  `CellEditEndReason` are also at the crate root. No import an L2 program
  writes changes.
- Added: `DraftEditorController::set_read_only`, which makes the active
  session read-only or editable again without ending it (the draft, payload,
  selection and undo history stay), releases a drag's pointer capture, and,
  called while the editor serves a clipboard request, applies when that
  operation completes, before `on_edit`; `TextCommand::DiscardDraft`,
  answered by every built-in editor also while disabled or read-only:
  `Handled` with Escape's effect and no write when there was a draft (an
  in-place session, including `InlineSlider`'s text mode, ends; a parameter
  draft is cancelled; a search query is dropped; a composition and a
  selection drag end, releasing the drag's pointer capture), `Unavailable`
  otherwise. A `DraftEditor` answers it `Unavailable`; the command is aimed
  at the control that owns the draft. An in-place control whose session held
  focus requests focus for itself, as Escape does (a row-mode `InlineText`,
  not focusable at rest, leaves focus to its nearest focusable ancestor, such
  as its list), so `Runtime::text_command`
  and `EventContext::request_text_command` now document two commands that
  request focus.
- Documentation: contracts.md's "Draft policy" (scope, save attempts, rules
  1 to 11, the event, parameter-outcome and table-completion tables),
  replacing the per-editor lists in "Permissions and availability" and
  "Closing a window"; editor sessions and table cell edits state the
  ordering and `on_edit_end`. `App::on_close` step 3 links the policy; the
  rustdoc of the number fields' `enabled`, `editable`, `stepper` and
  `session`, `InlineInteger`, `SearchField::editable` and `debounce`,
  `TextField::session` and `on_modified`, `DraftEditor`'s owner contract and
  `Table`'s cell editing are rewritten to it. The guide's controls, testing
  and custom-widgets chapters and the manual name the policy, `on_modified`
  on the added types and `DiscardDraft`.
- API listings: `api/pert-ui-runtime/default.txt` and `serde.txt` add the
  items above (the six prelude names and the two new types also under
  `prelude`). The facade's listings are unchanged: they record its root and
  prelude as glob re-exports of the runtime's.

## 0.3.74 (2026-10-27)

One way where history left two (layering plan, part L3, from its approved design
checkpoint `plans/usability/layering-design-L3.md`, revision 4). The methods
only a host calls leave `Runtime`'s own page for a sealed trait in a new `host`
module; the legacy root that is never laid out and the full-key lookup
`find_path` are removed; `Painter::size` is no longer optional; and an `Each`'s
default container stops adding an `items` segment to its items' addresses, which
moves the workspace document to revision 2.

- Breaking: the host-integration methods move from inherent `Runtime` methods to
  the sealed trait `pert_ui::host::HostIntegration`
  (`pert_ui_runtime::host::HostIntegration`), which `Runtime` implements, with
  unchanged signatures, behavior and errors: `flush_windows`, `flush_commands`,
  `has_command_requests`, `application_deadline`, `interval_deadline`,
  `service_intervals`, `service_inspection`, `set_wake_delivery`,
  `select_window`, `current_window`, `set_window_visible`,
  `set_presentation_rate`, `presentation_rate`, `needs_paint`, `invalidate`,
  `has_presentation_requests`, `take_presentation_requests`, `prepare_detach`,
  `commit_detach`, `redock`, `close_detached_window`, `native_window_options`,
  `text_preedit`, `text_input_state`, `cursor_icon`, `clear_focus`,
  `cancel_input`, `captures_supported`, `pending_captures`, `capture_scene`,
  `complete_capture`, `set_clipboard`, `take_shutdown_errors`. Replacement: a
  custom host adds `use pert_ui::host::HostIntegration;` (`use
  pert_ui_runtime::host::HostIntegration;` with the runtime crate alone); a test
  that globs `pert_ui::testing::*` needs nothing; applications using `App` are
  unaffected. Neither the prelude nor a crate-root glob brings the trait into
  scope, so a call from application code fails with rustc's help naming the
  trait. `set_viewport`, `viewport`, `paint`, `command_key_input`,
  `native_modifiers`, `keyboard` and `set_capture_support` stay inherent.
- Breaking: `Runtime::mount` is removed; use `ui.mount_tree(Element::new(key,
  component))`, with the same key rules and errors. A root mounted so is
  measured and arranged into the whole window, so it has bounds, takes part in
  hit testing, focus navigation and development validation's overlap check, and
  is presented (painted, animated, polled for live data) only while its window's
  viewport has area; its commands carry its
  allocation as their clip. An overlay probe above other content returns false
  from `Component::hit_test` and reports `OverlapPolicy::Allow`.
- Breaking: `Painter::size` returns `Size`, the component's allocation in local
  logical pixels, which may be empty, instead of `Option<Size>`: drop the `None`
  branch, `let size = painter.size();`. A paint of a presented entry without an
  allocation, which layout makes unreachable, is refused with
  `DrawError::PendingUpdates`.
- Breaking: `Runtime::find_path` is removed; use `Runtime::find` with the
  addressable path, or walk `Runtime::children` and `Runtime::key` level by
  level.
- Breaking: an `Each`'s default container is internal structure
  (`Element::structural`) and no longer adds `items` to its items' addresses:
  `find(&each, &["items", key])` becomes `find(&each, &[key])`, and the same for
  `Selector` paths, `CaptureTarget::Path`, workspace paths and content-search
  hit paths. The container keeps its key for `find_descendant`, `Selector::Key`
  and `Runtime::children`. A container given with `within` keeps its key in the
  address, `items` included. A container's addressing is decided when it first
  mounts; to change it, give the new container another key, which mounts the
  items again. An item the application marks `Element::structural` now takes its
  name from the `Each`'s label or a naming parent above the `Each`, and under an
  `Each` the application marks structural, content search names an ordinary item
  from a `Field` above it.
- Breaking: `Workspace::SCHEMA_VERSION` is 2; a revision-1 document is rejected:
  `Workspace::decode` returns `WorkspaceError::UnsupportedVersion { found: 1,
  supported: 2 }`, and serde deserialization fails with that error's message.
  Capture the workspace again. The format is otherwise unchanged.
- Changed: `ComponentInspection::path` of an `Each` item omits the default
  container, and the container has no path, as for every structural component.
  `InspectionSnapshot::SCHEMA_VERSION` stays 15, since no field is added,
  removed or reinterpreted. `TestError::NotLaidOut` stays as the driver's report
  of a broken layout invariant; the reveal before a click flushes, so a revealed
  target has bounds.
- Added: the module `pert_ui_runtime::host` with the trait `HostIntegration` (the
  methods above and its implementation for `Runtime`) and `#[doc(no_inline)]`
  re-exports of the names a host writes: `CaptureError`, `CaptureJob`,
  `Clipboard`, `ClipboardError`, `CommandKey`, `CommandKeyInput`, `CursorIcon`,
  `DisplayList`, `DrawError`, `FlushReport`, `FrameImage`, `Invalidation`,
  `KeyAction`, `KeyCode`, `KeyDispatch`, `KeyboardState`, `LimitError`,
  `Magnification`, `Modifiers`, `NativeWindowId`, `NativeWindowOptions`,
  `PhysicalExtent`, `Point`, `PointerButton`, `PointerEvent`, `PointerKind`,
  `PreparedDetach`, `PresentationAction`, `PresentationRate`,
  `PresentationRequest`, `PresentationStatus`, `Runtime`, `RuntimeError`,
  `TextInputState`, `TextPreedit`, `Viewport`, `WidgetId`, `WindowError`; the
  facade's module `pert_ui::host`, with the same trait and re-exports and a
  complete custom-host example; `HostIntegration` in `pert_ui_runtime::testing`
  and `pert_ui::testing`; the diagnostics fixture `h01_hostintegration_impl` for
  the sealed trait. `HostIntegration` is reached through `host` and `testing`
  only, not at the crate root.
- API listings: `api/pert-ui-runtime/{default,serde}.txt` add
  `pert_ui_runtime::host` with its trait, its methods under
  `host::HostIntegration` and `testing::HostIntegration`, and the listing of
  every re-exported item under `host::`; they remove the 33 inherent `Runtime`
  methods, `Runtime::mount` and `Runtime::find_path` and change `Painter::size`.
  Every facade listing adds `pert_ui::host` with its re-exports and
  `pert_ui::testing::HostIntegration`. `api/pert-ui-cpu` and `api/pert-ui-wgpu`
  are unchanged.
- Documentation: `Runtime`'s audience index points hosts to the trait;
  contracts.md's custom hosts, workspace persistence and children from data; the
  guide's list rows, layout, number-input and custom-component examples; the
  facade's root page and module tables; and every migrated rustdoc example.
- Tests: the tests that mounted legacy roots mount laid-out roots. Tests without a
  viewport that relied on a legacy root being presented set one; tests whose
  fixtures reported their own geometry place them with a test-local container;
  scene assertions of an unclipped command expect the root's allocation as its
  clip; and the deferred-notification delivery test settles the root's own
  first update and layout before the edit and clears the unrelated focus
  restoration that removing a focused laid-out root leaves due
  (`HostIntegration::clear_focus`, no flush), keeping its deadline assertions
  around the delivering parameter write, with the outcome monitor's count as
  further evidence.

## 0.3.73 (2026-10-27)

A layered prelude (layering plan, part L2, from its approved design checkpoint
`plans/usability/layering-design-L2.md`). The prelude holds what an
application's first program writes, and thematic modules gather the rest by
need. Every name stays at the crate root; no signature, behavior, persisted
format or real-time endpoint changes.

- Breaking: the prelude (`pert_ui::prelude`, `pert_ui_runtime::prelude`) holds
  the names an application's first program writes: the names the guide's
  representative programs write in their application part, every built-in
  component outside the thematic modules with the types its constructors and
  builders take, the callback values an application matches and the values it
  keeps, and the application-wide names (`Runtime`, `Signal`, `ReadSignal`,
  `StateOwner`, `Element`, `IntoElement`, `WidgetId`, `EventContext`,
  `Permission`, `RuntimeError`, `ApplicationError`). The names below leave it,
  by replacement import; each stays at the crate root.
- Breaking: a custom component adds `use pert_ui::component::*;` for
  `ArrangeContext`, `ChildInfo`, `ChildReveal`, `Component`, `ContentRows`,
  `DisplayList`, `DraftAlign`, `DraftEditor`, `DraftEditorController`,
  `DraftSelection`, `DragEvent`, `DragOutcome`, `DragPayload`, `DragPhase`,
  `DragRole`, `DrawCommand`, `DrawError`, `DropResponse`, `EditorSession`,
  `EditorStart`, `FillRule`, `IconSource`, `ImageId`, `KeyTransition`,
  `KeyboardState`, `LineCap`, `LineJoin`, `Magnification`, `MaskId`,
  `MaskImage`, `MeasureContext`, `MotionPreference`, `Notifier`, `Painter`,
  `Path`, `PathBuilder`, `RestoreRejection`, `Shape`, `StrokeStyle`,
  `SurfaceKind`, `TextCommandOutcome`, `TextInputState`, `TextKeyOutcome`,
  `TextLayout`, `TextPreedit`, `UpdateContext`, `VectorGeometry`, `VectorId`,
  `ViewState`, `WorkspaceError`, `WorkspaceValue`.
- Breaking: a test adds `use pert_ui::testing::*;` for `CommandKeyInput`,
  `FocusDirection`, `InputOutcome`, `KeyAction`, `PointerButton`,
  `PointerEvent`, `PointerKind`, `Viewport`, which `component` also holds.
- Breaking: the authoring names that inspection records carry,
  `ComponentDetails`, `Invalidation`, `KeyEvent`, `Rect`, are in `use
  pert_ui::component::*;` and `use pert_ui::inspection::*;` (and `Rect` in
  `windowing`).
- Breaking: command names other than `CommandScope`, `KeyChord`, `CommandKey`,
  `KeyCode` and `Modifiers`, which stay in the prelude, need `use
  pert_ui::commands::*;`: `CommandContext`, `CommandEntry`, `CommandId`,
  `CommandMatcher`, `CommandOutcome`, `CommandPalette`,
  `CommandPaletteController`, `CommandPaletteStatus`, `CommandReceipt`,
  `CommandSource`, `CommandStatus`, `HeldCancellation`, `HeldInputId`
  (`CommandContext`, `CommandId`, `CommandOutcome`, `CommandReceipt`,
  `CommandSource` and `CommandStatus` also in `component`).
- Breaking: parameter and authority names, `Slider`, `Knob` and `InlineSlider`
  need `use pert_ui::parameter::*;`: `AnyDeliveryMonitor`, `AnyParameter`,
  `AnyParameterHost`, `AnyParameterSnapshot`, `AnyPreviewEvent`, `AnySpec`,
  `AnyTerminalEvent`, `AnyValue`, `ArcOrigin`, `AuthorityReceipt`,
  `AuthorityResponse`, `AuthorityStatus`, `ChoiceId`, `ChoiceOption`,
  `DeliveryMonitor`, `DeliveryResolution`, `DeliverySnapshot`,
  `DeliveryStatus`, `EditOutcome`, `EditSession`, `EditToken`, `InlineSlider`,
  `Knob`, `NormalizedValue`, `Parameter`, `ParameterFamily`, `ParameterHost`,
  `ParameterId`, `ParameterMapping`, `ParameterSnapshot`, `ParameterSpec`,
  `ParameterValue`, `PowerExponent`, `PreviewEvent`, `PreviewOutcome`,
  `RemoteCommit`, `RemoteHost`, `ResponseDisposition`, `Revision`,
  `SinkError`, `SinkErrorKind`, `Slider`, `SubmissionDelivery`,
  `TerminalEvent` (`ParameterSpec` also in `live`, `NormalizedValue` also in
  `windowing`).
- Breaking: windows, regions, splits and tool windows need `use
  pert_ui::windowing::*;`: `EdgeMode`, `Expansion`, `FloatingWindow`,
  `RegionEdge`, `RegionLayout`, `SplitAxis`, `SplitPane`, `SplitRule`,
  `SplitSizing`, `StripeGroup`, `ToolEdge`, `ToolWindow`, `ToolWindowChange`,
  `ToolWindowChangeKind`, `ToolWindowController`, `ToolWindowLayout`,
  `ToolWindowMode`, `WindowCloseAction`, `WindowController`, `WindowLayer`,
  `WindowPlacement`, `WindowStatus`.
- Breaking: the live widgets need `use pert_ui::live::*;`: `LevelMeter`,
  `MeterAxis`, `Trace`.
- Breaking: theme, settings, fonts, strings, the workspace document,
  `DescribeContext`, `HexColorError`, `PhysicalExtent` and `VirtualAxis` are
  imported by name from the root, for example `use pert_ui::{ThemeMode,
  Workspace};`: `Density`, `DescribeContext`, `FontData`, `FontId`,
  `FontManager`, `FontRole`, `HexColorError`, `IconGlyph`, `IconManager`,
  `IgnoredEntry`, `NumberLocale`, `OpaqueColor`, `Palette`, `PhysicalExtent`,
  `StringRole`, `TextRenderingMode`, `TextSource`, `ThemeManager`,
  `ThemeMetrics`, `ThemeMode`, `UiMessage`, `UiSettings`, `UiStrings`,
  `UiZoom`, `VirtualAxis`, `Workspace`, `WorkspacePath`, `WorkspaceReport`,
  `WorkspaceSettings`.
- Added: the modules `parameter` (authoritative values: parameters, edit
  gestures, hosts, external authority and the parameter-only controls),
  `commands` (commands, shortcuts and the command palette), `windowing`
  (windows inside a view and native windows, regions, splits and tool windows;
  not `windows`, which would collide with the `windows` crate under a
  crate-root glob), `inspection` (inspection, capture, profiling and
  validation) and `live` (live series and mailboxes, with the live widgets) in
  `pert-ui-runtime` and `pert-ui`, each closed one level over its members'
  constructors, builders, payloads and fields. `NumberInputError` joins the
  prelude. `testing` gains `Viewport`, `KeyAction`, `FocusDirection`,
  `PointerEvent`, `PointerKind`, `PointerButton`, `CommandKeyInput`,
  `InputOutcome`, `SemanticValue` and `ClickCounter`; `component` gains
  `ClickCounter`, `DraftAlign`, `DraftEditor`, `DraftEditorController` and
  `DraftSelection`. The runtime's private modules `inspection` and `live` are
  renamed `inspection_service` and `live_source`; no public path changes.
- Added: the members of `pert_ui_runtime::parameter` and `pert_ui::parameter`:
  `parameter::AnyDeliveryMonitor`, `parameter::AnyParameter`,
  `parameter::AnyParameterHost`, `parameter::AnyParameterSnapshot`,
  `parameter::AnyPreviewEvent`, `parameter::AnySpec`,
  `parameter::AnyTerminalEvent`, `parameter::AnyValue`,
  `parameter::ArcOrigin`, `parameter::AuthorityReceipt`,
  `parameter::AuthorityResponse`, `parameter::AuthorityStatus`,
  `parameter::BindingError`, `parameter::ChoiceId`, `parameter::ChoiceOption`,
  `parameter::DeliveryError`, `parameter::DeliveryMonitor`,
  `parameter::DeliveryResolution`, `parameter::DeliverySnapshot`,
  `parameter::DeliveryStatus`, `parameter::EditError`,
  `parameter::EditOutcome`, `parameter::EditSequence`,
  `parameter::EditSession`, `parameter::EditSource`, `parameter::EditToken`,
  `parameter::HostValue`, `parameter::InlineSlider`, `parameter::Knob`,
  `parameter::NormalizedValue`, `parameter::NormalizedValueError`,
  `parameter::Parameter`, `parameter::ParameterError`,
  `parameter::ParameterFamily`, `parameter::ParameterHost`,
  `parameter::ParameterId`, `parameter::ParameterIdError`,
  `parameter::ParameterMapping`, `parameter::ParameterSnapshot`,
  `parameter::ParameterSpec`, `parameter::ParameterState`,
  `parameter::ParameterValue`, `parameter::PowerExponent`,
  `parameter::PreviewEvent`, `parameter::PreviewOutcome`,
  `parameter::RemoteCommit`, `parameter::RemoteHost`,
  `parameter::ResponseDisposition`, `parameter::Revision`,
  `parameter::SinkError`, `parameter::SinkErrorKind`, `parameter::Slider`,
  `parameter::SubmissionDelivery`, `parameter::TerminalEvent`.
- Added: the members of `pert_ui_runtime::commands` and `pert_ui::commands`:
  `commands::CommandContext`, `commands::CommandEntry`, `commands::CommandId`,
  `commands::CommandInfo`, `commands::CommandKey`, `commands::CommandMatcher`,
  `commands::CommandOutcome`, `commands::CommandPalette`,
  `commands::CommandPaletteController`, `commands::CommandPaletteStatus`,
  `commands::CommandReceipt`, `commands::CommandScope`,
  `commands::CommandSource`, `commands::CommandStatus`,
  `commands::HeldCancellation`, `commands::HeldInputId`, `commands::KeyChord`,
  `commands::KeyCode`, `commands::Modifiers`, `commands::NativeWindowId`.
- Added: the members of `pert_ui_runtime::windowing` and `pert_ui::windowing`:
  `windowing::DetachedClosePolicy`, `windowing::EdgeMode`,
  `windowing::Expansion`, `windowing::FloatingWindow`,
  `windowing::NativeWindowId`, `windowing::NativeWindowOptions`,
  `windowing::NormalizedValue`, `windowing::PresentationReceipt`,
  `windowing::PresentationStatus`, `windowing::Rect`, `windowing::RegionEdge`,
  `windowing::RegionLayout`, `windowing::SplitAxis`, `windowing::SplitPane`,
  `windowing::SplitRule`, `windowing::SplitSizing`, `windowing::StripeGroup`,
  `windowing::ToolEdge`, `windowing::ToolWindow`,
  `windowing::ToolWindowChange`, `windowing::ToolWindowChangeKind`,
  `windowing::ToolWindowController`, `windowing::ToolWindowLayout`,
  `windowing::ToolWindowMode`, `windowing::WindowCloseAction`,
  `windowing::WindowController`, `windowing::WindowError`,
  `windowing::WindowLayer`, `windowing::WindowPlacement`,
  `windowing::WindowStatus`.
- Added: the members of `pert_ui_runtime::inspection` and
  `pert_ui::inspection`: `inspection::BorderInspection`,
  `inspection::BoxModelInspection`, `inspection::Capture`,
  `inspection::CaptureError`, `inspection::CaptureRequest`,
  `inspection::CaptureTarget`, `inspection::ComponentDetails`,
  `inspection::ComponentInspection`, `inspection::DebugMode`,
  `inspection::DebugOverlay`, `inspection::Depth`, `inspection::Detail`,
  `inspection::DiagnosticCommand`, `inspection::DragInspection`,
  `inspection::EventHistory`, `inspection::EventKind`,
  `inspection::EventRecord`, `inspection::EventStatus`,
  `inspection::FailurePhase`, `inspection::FailureSite`,
  `inspection::FlushReport`, `inspection::FrameError`,
  `inspection::FrameImage`, `inspection::InspectValue`,
  `inspection::InspectionClient`, `inspection::InspectionQuery`,
  `inspection::InspectionReply`, `inspection::InspectionService`,
  `inspection::InspectionSnapshot`, `inspection::Invalidation`,
  `inspection::KeyEvent`, `inspection::LimitError`,
  `inspection::NativeWindowId`, `inspection::OverlapPolicy`,
  `inspection::ParameterInspection`, `inspection::Pending`,
  `inspection::ProfileOperation`, `inspection::ProfileSample`,
  `inspection::ProfileSnapshot`, `inspection::ProfileStats`,
  `inspection::QueryResult`, `inspection::Rect`, `inspection::RequestError`,
  `inspection::ResourceKind`, `inspection::Selector`,
  `inspection::SemanticAction`, `inspection::SemanticItem`,
  `inspection::SemanticRole`, `inspection::SemanticValue`,
  `inspection::Semantics`, `inspection::ShortcutBinding`,
  `inspection::ShortcutScope`, `inspection::SourceInspection`,
  `inspection::ValidationCode`, `inspection::ValidationHints`,
  `inspection::ValidationIssue`, `inspection::ValidationMarker`,
  `inspection::ValidationReport`, `inspection::Viewport`,
  `inspection::WaitError`, `inspection::WindowInspection`.
- Added: the members of `pert_ui_runtime::live` and `pert_ui::live`:
  `live::Capacity`, `live::Delivery`, `live::DrainReport`, `live::Freshness`,
  `live::Incarnation`, `live::LevelMeter`, `live::LiveError`,
  `live::LiveItem`, `live::LiveWaker`, `live::Mailbox`, `live::MeterAxis`,
  `live::ParameterSpec`, `live::PublishOutcome`, `live::Sample`,
  `live::SampleWindow`, `live::SampleWindowEntry`, `live::SendError`,
  `live::Sequence`, `live::SeriesConfig`, `live::SeriesConsumer`,
  `live::SeriesEndpoint`, `live::SeriesSource`, `live::Telemetry`,
  `live::Timestamp`, `live::Trace`, `live::UiSender`, `live::WakeError`,
  `live::WakeFlag`, `live::series_bridge`.
- Documentation: the guide's new "Authoritative values" chapter
  (`guide::authoritative_values`, after the state chapter, now titled "State")
  holds the parameter sections moved from the state chapter unchanged in
  substance; links to `guide::state#parameters` and its sibling anchors move
  to `guide::authoritative_values`. The prelude pages state the membership
  rule, the guarantee by audience, the paths that need another import and
  where every other name is, replacing "Root names to add"; the `component`,
  `testing` and crate pages, the runtime README and the custom-widgets
  chapter's imports describe the globs by audience. The guide's representative
  examples (the first window, the editable list of records, the dialogs, the
  menu bar with a command and the background work) are an application part
  importing only the prelude and a test in `fn main` adding `testing`.
- Migrated: the packaged examples, the gallery, the facade and runtime tests,
  the doc examples and the diagnostics fixtures import the modules they need.
  The `diagram_editor` canvas imports only `component`, and the `ContentRows`
  fixture `component`; the `i03` snapshot's candidate list names `ContentRows`
  unqualified now that the prelude does not hold it.
- Tests: the facade's foundation area rewrites `prelude` (a component from
  `prelude` and `component`, driven from `prelude` and `testing`, and a check
  that each representative program imports only its audience's globs) and adds
  `vocabulary` (each vocabulary group configured from the prelude alone) and
  `modules` (every promised name of every module, each thematic module's
  sufficiency, and every glob combined with the crate root resolving every
  exported name).

## 0.3.72 (2026-10-27)

One deferral call for application callbacks (layering plan, part L4a, from
its approved design checkpoint `plans/usability/layering-design-L4a.md`).
Work a callback queues runs whatever its own surface or window does next.

- Breaking: `EventContext::defer_detached` is removed; use
  `EventContext::defer`, which now has its "run after the surface closes"
  property without its window cancellation:
  `cx.defer_detached(move |ui| files.save_to(ui, path))` becomes
  `cx.defer(move |ui| files.save_to(ui, path))`.
- Breaking: `EventContext::defer` is no longer cancelled when its widget is
  hidden or removed, or when its window is hidden or closes; only shutdown
  cancels it before delivery. It runs in its widget's current window while
  the widget is mounted, else in the window it was queued in while that
  window is open, else with the window the delivering flush has selected.
  An operation whose targets are gone reports their error, such as
  `Unmounted`, from the flush. For the old behavior, a component's work on
  itself, use `EventContext::defer_while_shown`.
- Breaking: `EventContext::spawn`'s start is no longer cancelled when its
  window closes, superseding part 6f2's window owner (its decision D2); only
  shutdown cancels it before it starts. To tie a start to a component, use
  `cx.defer_while_shown(|ui| ui.spawn(work, deliver).map(|_| ()))`.
- Added: `EventContext::defer_while_shown`, the former `defer` rule under a
  name that states it: cancelled when its component is unmounted or not
  effectively visible at delivery (its own or an ancestor's visibility, a
  closed surface, a hidden native window), routed to the component's window
  at delivery. It is for a custom component's work on itself.
- Fixed: a task queued during `Runtime::shutdown`, such as from a
  `Component::cancel_input` that removal calls, is refused with `Closed`; it
  was accepted and its receipt stayed `Pending`. After `Runtime::shutdown`
  returns no queued task's receipt reads `Pending`.
- Documentation: `TaskStatus::Cancelled` names its three causes (shutdown
  before delivery, a `defer_while_shown` component hidden or removed, a
  panicking callback unwinding), and the rustdoc of the deferral calls
  states that cancellation reaches only queued tasks, not an executing one.
  The closing-surface warnings on `Select::on_change`, `InlineSelect`,
  `MenuItem::on_change`, `QuickPick` and `HistoryNavigator`, contracts.md
  (user-write callbacks, the "in the input" and "at the next flush" timing
  classes, the cancellation property, `EventContext::spawn`, and a table of
  each call's cancellation and window in "Calling the runtime from
  callbacks"), the manual and the guide's controls, state (the "Work
  deferred from a callback" table, with `defer_while_shown` as advanced) and
  custom-widgets chapters now say that work queued with
  `EventContext::defer` runs after the surface closes.
- Migrated: the `diagram_editor` example's open and save dialogs and the
  gallery's history restore, search jump and navigation history use
  `defer`; the gallery's worker decode uses `EventContext::spawn`; its
  workspace and interaction check components, which step themselves, use
  `defer_while_shown`.
- Tests: new `deferral` modules in the facade's foundation and surfaces
  areas cover a closing `Select`, `QuickPick`, menu row and dialog with the
  closure settled before delivery, hidden widgets and windows, detached
  windows closed with `Redock` and `RemoveView`, the origin-window fallback,
  a widget detached and then hidden, and queuing during shutdown, each
  checked with a mutation restoring the old rule; and the kept contracts
  (`defer_while_shown`, routing after detachment, ordering, shared capacity,
  nested flushes, failure, shutdown, shutdown inside a task, a panicking
  task). The window clause of the callback spawn test now expects the start
  to run.
- API listing: `pert_ui_runtime::EventContext::defer_detached` removed and
  `pert_ui_runtime::EventContext::defer_while_shown` added in
  `api/pert-ui-runtime/default.txt` and `serde.txt`.

## 0.3.71 (2026-10-26)

Documentation layering (layering plan, part L1): each audience meets its own
layer of the documentation first. No public item, signature or behavior
changes, and no code moves; the `api/` listings are unchanged. The review
record is `plans/usability/layering-L1-classification.md`.

- Documentation: contracts.md is classified per obligation as application,
  extension or maintainer material. Extension material (redescription and
  conformance helpers, retained generated children, layout and focus hooks,
  `Notifier` reporting, stacking, hit testing and pointer refresh, content
  search hooks, host-loaded data, editor sessions, workspace hooks, custom
  semantics, failure cleanup in components and what a custom host does)
  moves under a new heading, "For component and host authors", after the
  application sections; each
  application section keeps its guarantees, with a pointer where it named a
  moved hook. A table's cell editing gets its own application section, "Table
  cell edits".
- Documentation: a new `docs/internals.md` ("Implementation notes") holds
  only mechanisms moved from contracts.md: how command queries walk surface
  declarations, how notification delivery is prepared and how ToolWindows
  slots and coalesces its housekeeping, and the weak provenance and request
  lineage behind failure cleanup. Every guarantee an application or extension
  author observes stays in contracts.md, which links the notes.
- Documentation: contracts.md defines callback timing classes, derived from a
  classification of every public callback: in the input, after settling, at
  the next flush, at parameter delivery and evaluation (in the input also
  covers focus changes, where an editor saves on focus loss; a numeric
  editor's normalizer and its number format's parser are evaluation
  callbacks, since the editor evaluates its draft in update, paint and
  inspection), with the callbacks
  that keep a moment of their own and the excluded host-integration
  callbacks. Access, coalescing, cancellation and failure and replay are
  defined as separate properties. Each classified callback's rustdoc names
  its class with a link: the `on_*` builders, constructors that take a
  callback (`Button::new`, `controlled`, `Breadcrumb::new`, `QuickPick::new`
  and others), normalizers, deferral, `spawn`, `mailbox`, `start_interval`,
  parameter sinks, binding derivations, description factories, matchers and
  number formats. The table of callbacks that receive the runtime adds
  interval ticks and names `App`'s callbacks, and geometry notifications
  state that a `RegionLayout` reports once per divider.
- Documentation: the guide's state and layout chapters open with "Choosing a
  way" tables: where a control's value comes from (`new`, `controlled`,
  `for_parameter`, with the per-row and retained row forms as advanced),
  background work, work deferred from a callback with the closing-surface
  rule, and children from data. The state chapter opens with a signal and a
  text field; its custom component example moves to the custom-widgets
  chapter as "A component that follows a binding".
- Documentation: `Signal::sibling` and the guide's editable list recipe name
  the controlled form first and the per-row binding as the advanced form for
  a control without a controlled form.
- Documentation: `Runtime`'s page indexes its methods by audience
  (applications; tests and inspection, including the host methods a headless
  test uses to stand in for a host; hosts), and the `Component` trait's
  page groups its hooks by concern with the few a typical widget implements;
  the declaration order is unchanged.
- AGENTS.md: the design-reading list includes `docs/internals.md`, and the
  component API conventions gain a sentence: an addition that offers a new
  way to do something already possible names, in its change rationale, the
  way it makes advanced or retires, or says why none can be.

## 0.3.70 (2026-10-26)

Controlled controls (API audit fix plan, part 6f3, after its design
checkpoint `plans/usability/api-audit/design-6f3.md`). A checkbox, toggle or
editable label can show a value the application owns and propose each user
change to its callback, so a list row or table cell can edit a record
without a signal of its own, and a list may describe its rows again without
ending an open edit (F16). Not breaking.

- Added (F16): `Checkbox::controlled(checked, label, on_change)` and
  `Toggle::controlled(checked, label, on_change)`, a third ownership path
  beside `new` (a signal) and `for_parameter`. The control writes nothing
  and shows the described value only: a toggle proposes the negation of the
  value shown to `on_change`, at the moments `new` writes (a matched primary
  release inside, a matched unmodified Enter or Space release), and a new
  description with another value changes what it shows. Two activations
  before the next description propose the same value twice. A description
  with the same value keeps a pending activation; a changed value, or
  another path under the same key, cancels it. A later `on_change` replaces
  the constructor's callback, and each description replaces it.
- Added (F16): `InlineText::controlled(text, on_save)`. The label writes
  nothing: Enter or focus leaving it with a changed draft proposes the
  normalized draft to `on_save` and ends the session, and the label shows
  the described text until a new description changes it. A description with
  the same text keeps an open draft; changed text ends it unsaved, as an
  application write does with `new`. Described text the editor does not
  accept (`TextField::check` fails) is refused at adoption as a refused
  binding value is (previous text kept, `binding_error`, invalid semantics,
  no editing), without failing the runtime. Every other builder, `in_row`
  and `TextCommand::BeginEdit` work as with `new`.
- Added: the ownership path "described value with a change callback" in
  contracts.md's ownership paths, and a "Controlled controls" subsection
  under "Data and edit ownership" on presentation, refusal, callback timing
  and redescription, which "User-write callbacks" and "Component
  redescription" point to.
- Changed: `on_change` of `Checkbox` and `Toggle`, and `InlineText::on_save`,
  report a controlled control's proposal, where the other paths write,
  besides successful writes and proposal submissions; contracts.md's
  "User-write callbacks", the guide's controls chapter and the manual say
  so. AGENTS.md's user-write callback sentence names the controlled
  proposal, with the maintainer's approval.
- Changed: `InlineText::normalize`'s contract. The normalizer may read
  application state, never write it; it must be deterministic and
  idempotent for a given draft and unchanged observed state, a refusal may
  turn into acceptance when that state changes, and reading it creates no
  dependency, so a refused draft is checked again at the user's next save.
  The rustdoc gives a model-dependent example. `TextField::normalize` keeps
  its wording.
- Changed: AGENTS.md's and contracts.md's validation rule gain an exception,
  with the maintainer's approval: a value a description carries from
  application data, such as a controlled control's value, is refused at
  adoption with the last valid presentation kept, rather than rejected by
  `Component::check_description`.
- Documentation (F16): `ListItems::build` and `TableItems::cell` name the
  controlled form as the way for `Checkbox`, `Toggle` and `InlineText`,
  keeping the per-row and retained forms, with their costs, for other
  controls. The guide's editable list recipe builds its checkbox with
  `Checkbox::controlled` and its title with `InlineText::controlled(...)
  .in_row(true)`, keyed `"title"`, instead of a per-row signal and a
  retained `Title` component, so `on_activate` finds the title at
  `[key, "title"]`. The inventory example's Active column uses
  `Checkbox::controlled` in `TableItems::cell` instead of its retained
  `ActiveCell` component. The guide's controls chapter and the manual's
  component table name the controlled constructors.
- The runtime's `api/` listings (default and `serde`) add
  `Checkbox::controlled`, `Toggle::controlled` and `InlineText::controlled`,
  at the root and under the `prelude`; the facade listings are unchanged.
- Added: tests in the facade's binaries: `controls`' new `controlled` module
  and `collections`' new `controlled_rows` module (F16 in a list, under a
  filter and in a table cell), each test marked as a new-behavior,
  contract or compatibility test.

## 0.3.69 (2026-10-26)

Editing and collection capabilities (API audit fix plan, part 6f2, after its
design checkpoint `plans/usability/api-audit/design-6f2.md`). Callbacks start
background work (F13), code begins an edit (F15), a label can be part of a
selectable list row (F27), and a collection filter can keep the active item
on a shown row while the user types in its field (F7b). Not breaking.

- Added (F13): `EventContext::spawn`, which starts background work from a
  callback as `Runtime::spawn` does. It queues the start, the operation
  `|ui| ui.spawn(work, deliver).map(|_| ())`, bound to the widget's window as
  `EventContext::defer_detached` is: the thread starts at the beginning of
  the next flush, also when the widget was hidden or removed by then, and
  the start is cancelled only when the window closed or the runtime shut
  down first. Its `TaskReceipt` reports the start: `Succeeded` once the
  thread started, `Failed` with `Runtime::spawn`'s refusal (which also fails
  that flush), or `Cancelled`. The call itself fails with `TaskQueueFull`
  and the phase and shutdown errors. There is no cancel handle; the rustdoc
  points to `defer` with `Runtime::spawn` for a cancellable delivery.
- Added (F15): `TextCommand::BeginEdit`, which begins editing in an editor
  that has a resting state, as its own Enter or F2 does, without depending
  on the keyboard's state. `InlineText` answers it at rest by beginning to
  edit with the text selected (`Handled`), and with `Unavailable` while it
  is disabled, refuses its binding or is already editing. Inspection
  reports the edit in each editor's own property: `editing` for
  `InlineText`, `text_editing` for the inline rows. Editors without a
  resting state (`TextField`, `TextArea`, `SearchField`, `IntegerInput`,
  `DecimalInput`, `DraftEditor`) and `SelectableText` answer `Unavailable`.
- Added (F27): `InlineText::in_row(bool)`, a list-row mode, default `false`.
  The resting label is not focusable and adds no Tab stop, so a press
  focuses the list; an unmodified press is left to the row (which selects
  it and may arm a reorder drag) unless it is the second of a click series,
  which begins editing; a press with Shift, Control, Alt or Super is left
  to the row. A series ends when the pointer leaves the label (also when a
  reorder drag starts from it), on a modified press, on cancelled input,
  hiding, disabling and when an edit begins. Editing that ends by key
  leaves focus to the nearest focusable ancestor, the list. Inspection
  reports `in_row` while the mode is on; redescription replaces it and
  never ends a draft.
- Added (F7b): `HiddenActive` (`Keep`, the default; `Nearest`; `Clear`),
  `#[non_exhaustive]`, `Copy`, `Eq`, `Hash`, `Default`, at the root and in
  the prelude beside `CollectionFilter`, and
  `CollectionFilter::hidden_active(HiddenActive)`. The policy acts once per
  delivered filter change evaluated while focus is outside the collection:
  `Nearest` moves the active item to the nearest row still shown when this
  change hid it and a row remains, `Clear` leaves no item active whenever
  the active item is hidden after the change, and `Keep` leaves it, as
  before. With focus in the collection the earlier rule holds under every
  policy. Only the active item changes; hidden selected keys stay selected.
  Inspection reports `hidden_active` (`keep`, `nearest` or `clear`), and
  redescription replaces the policy. Internally a table's or tree's source
  replacement now reports the active key it hid with the nearest row, so
  `Nearest` acts only on its own transition.
- Changed (F15): the text-command contract. A text command still never
  moves focus, except `TextCommand::BeginEdit`, whose `Handled` means the
  editor began editing and requested focus for its editing part. That
  request settles at the next flush after the synchronous
  `Runtime::text_command`, and at the end of the delivering flush for
  `EventContext::request_text_command`. The flush's order of updates,
  removals, layout and focus is unchanged. The one difference is in focus
  requests: a request whose mounted target is hidden only until an
  ancestor's update still pending in the same flush shows it, as the
  editor a command began is until its label updates, is kept for the next
  pass of that flush instead of being dropped, so focus reaches the editor
  when the delivering flush settles successfully; a flush that fails first
  leaves the request pending with the update, and a later flush retries
  both. A focus restoration in the same step cancels it, as any focus
  transfer does, except that a focused component containing the target
  (the label whose editor it is) keeps focus for the transfer although it
  stopped being focusable to make it. A target still hidden once no such
  update is pending loses the request as before. `EventContext::request_focus`
  and contracts.md's focus paragraph say so. The `TextCommand`,
  `Component::text_command`, `Runtime::text_command` and
  `request_text_command` rustdoc and contracts.md say so.
- Changed (F15): `InlineInteger` and `InlineDecimal` answer `BeginEdit` at
  rest by entering text mode as Enter/F2 does, read-only text mode
  included, where they answered `NotAnEditor`. They answer `Unavailable`
  while disabled, without a valid value since mount, in text mode, and
  during a step gesture (a step drag, a pressed stepper or a held step key),
  which is narrower than the key path; the gesture still ends on its
  release. Every other command at rest is still `NotAnEditor`.
- Changed (F15): `InlineSlider` and `InlineReadout` also answer `BeginEdit`
  at rest, where they answered `NotAnEditor`, since Enter/F2 enters their
  text mode too. `InlineReadout` enters its read-only text mode (`Handled`).
  `InlineSlider` enters text mode exactly where Enter/F2 does, and answers
  `Unavailable` where that key does nothing: while the parameter takes no
  edit (disabled, unavailable, without edit authority or not editable),
  during a gesture that holds the row's edit (a pressed value, a track drag
  or a held step key) and in text mode. Both answer `Unavailable` in text
  mode; `Slider` and `Knob` have no text mode and still answer
  `NotAnEditor`.
- Documentation (F27): `InlineText`'s default press behavior, unchanged in
  code: a single primary press is handled and focuses the label, a
  double-click edits, and a row around it never sees the press.
- Documentation (F15): `Runtime::key_input` says a synthesized press of a
  key that is physically held is a repeat, which controls acting on a fresh
  press ignore, and points to `TextCommand::BeginEdit`.
- Documentation (F13): `Runtime::spawn` points callbacks to
  `EventContext::spawn`, and `EventContext::defer` no longer calls itself
  the way to start background work. The guide's "Worker results and live
  data" uses `cx.spawn` and says who owns the start, the data-contract table
  names `EventContext::spawn`, and contracts.md's background-work paragraph
  describes it.
- Documentation (F7b, F15, F27): the guide's editable list recipe builds
  its titles with `InlineText::in_row(true)`, renames the active row from
  `VirtualList::on_activate` with `TextCommand::BeginEdit` instead of
  writing an "opened" signal, uses `HiddenActive::Clear`, and asserts that
  a press selects a row, a double click and Enter rename, and typing in the
  field leaves no row active. `CollectionFilter`'s "Selection and focus"
  paragraph and `FilterOutcome` name the policy, and the rustdoc gives a
  guard over the selected keys for a `Multiple` selection.
- Added: contract tests in the facade's binaries: `foundation`'s
  `application_services` for `EventContext::spawn`, `editing`'s new
  `begin_edit` module, and `collections`' new `inline_text_rows` and
  `filter_hidden_active` modules, with the compatibility checks named as
  such. The filter's behavior tests run for a list, a table and a tree.
- The runtime's `api/` listings (default and `serde`) add
  `EventContext::spawn`, `TextCommand::BeginEdit` (also under the `prelude`
  and `component` paths), `InlineText::in_row`, `HiddenActive` with its
  variants and trait implementations (also under the `prelude` path) and
  `CollectionFilter::hidden_active`. The facade's listings do not change,
  since the facade re-exports the runtime by glob.

## 0.3.68 (2026-10-23)

Testing and window close (API audit fix plan, part 6f1). A test driver and a
headless application close the round 1 findings F2 and F5, a close request
saves focused drafts before the close policy runs (F4), and the packaged
examples' sessions use the driver (F34). Breaking in behavior: see F4 below.

- Added (F2, G5-32): the module `pert_ui_runtime::testing`, re-exported as
  `pert_ui::testing`, for headless tests:
  - `testing::Driver<'a>`, which borrows a `Runtime` and drives it as a user
    would, one settled step at a time: `Driver::new`, `runtime`,
    `runtime_mut`, the constant `FRAME` (16 ms), `settle` and `advance` (one
    flush at a checked later clock), `wait_until` (bounded polling at the
    same clock, for background work), `find` and `semantics` (one
    `Runtime::query` with `Selector::id` at `Detail::Semantics`), `click`
    (reveal, then a primary press and release at the middle of the bounds),
    `replace_text` (focus, select all, then type or Backspace),
    `type_text`, `key` and `chord` (synthetic keys through
    `Runtime::command_key_input`, a chord's modifiers pressed in the fixed
    order Control, Alt, Shift, Super and reported with
    `Runtime::native_modifiers`). Input goes to the selected window; `key`,
    `chord`, `replace_text` and `click` require neutral keyboard input, and
    a compound action releases everything it pressed when a step fails. Its
    `Debug` shows no runtime state.
  - `testing::TestError`, `#[non_exhaustive]`, `Clone`, `Eq`, with the
    variants `Runtime` (converted from `RuntimeError`), `NotFound { path }`,
    `OtherWindow { key }`, `NotRevealed { key, refusal }`,
    `NotLaidOut { key }`, `NotAnEditor { key }`, `NoSemantics { key }`,
    `TimedOut { waited }`, `ClockOverflow`, `InputHeld` and
    `Cleanup { error, cleanup }`, whose source is the cleanup failure.
  - re-exports of `CaptureRequest`, `CaptureTarget`, `FrameImage`,
    `InspectionQuery`, `Selector` and `Detail`, which stay at the crate root
    too; the facade's module adds `CpuRenderer` with `cpu` and
    `HeadlessApp` with `native`. The prelude pages point to `testing`.
- Added: `Runtime::can_focus`, a query that answers whether a component is
  mounted, visible, focusable by its own `Component::focusable` and not
  blocked by a modal surface, without revealing, flushing or changing
  anything; an unmounted or hidden component answers `false`, and only a
  foreign ID is an error. Close admission uses it before focusing a released
  editor again.
- Added (F5): `App::headless`, which validates the title, sets the main
  window's configured size at scale 1 as the viewport, runs the initializer
  and lays the visible windows out, without an event loop, window or GPU,
  and shuts the runtime down when that fails; and
  `pert_ui::testing::HeadlessApp` (with `native`), `#[must_use]`, with
  `runtime`, `runtime_mut`, `main_window`, `admit_close`, which runs close
  admission and `App::on_close` for a window and returns the decision
  without applying it, validating a `Defer` signal at once, and `shutdown`,
  which reports cleanup failures as `AppError::Shutdown`. Its `Debug` shows
  the title and the main window's geometry. `App`'s `#[must_use]` message
  names `headless`.
- Breaking (F4): a close request, from the window system or
  `Runtime::request_close`, releases focus in every window the close would
  discard (the detached window's own, or every window for the main window)
  before `App::on_close` runs, so each focused editor applies its
  focus-loss policy: a valid changed draft is saved before the policy reads
  the application's state, and a parameter-bound draft is cancelled. When
  no window is in component-error backoff, those windows are then flushed,
  and a failed `focus_lost` aborts the close through `on_error`. On a
  synchronous `KeepOpen` the affected windows are flushed again and the
  released component is focused again when nothing in its window holds
  focus after the callback and `Runtime::can_focus` admits it, so a
  component the policy disabled stays unfocused; during backoff nothing is
  restored, since that flush and focusing's reveal can retry a failing
  update. `Defer` and `Close` restore nothing, and the native host forgets
  the affected windows' reactivation records. Replacement usage: drop any `Runtime::clear_focus`
  workaround in `on_close`; the `diagram_editor` example's close policy
  drops its own. `App::on_close` and contracts.md ("Closing a window")
  state the order and the editor cases.
- Fixed: an accepted close of a detached window that was waiting to retry a
  component failure kept its pending recovery error, which `App::run` now
  returns at exit as it returns a surviving window's, instead of dropping it
  with the window.
- Changed (F2): rustdoc of `Runtime` and its input methods
  (`text_input`, `key_input`, `command_key_input`, `pointer_input`) says
  input is delivered at once and written by its component's rules, and
  the tree, derived bindings and layout show it after the next flush;
  `text_input` says empty text inserts nothing and leaves a selection;
  `Runtime::focus` and `TextField` say focus leaves the caret where the
  last edit or movement left it; `inspection_id`'s example uses
  `Runtime::query` with `Selector::id`; `pointer_input` has a click
  example. `Runtime::request_close` and `take_presentation_requests` say
  how a host other than `App` completes a close request.
- Added (F2): the guide chapter "Testing an application"
  (`pert_ui::guide::testing`): reading what the runtime shows with
  `Runtime::query` and `Selector::id`, clicks and click timing, typing and
  replacing text, keys and chords with the synthetic events' limits,
  settling, list rows, background work, `HeadlessApp` and the close policy,
  and a CPU capture. The crate page's map links it.
- Changed (F34): the diagnostics guide's keyboard example uses
  `Driver::chord`, and names `command_key_input` and `native_modifiers` as
  the low-level entry points for layouts, text and composition. The
  `inventory`, `log_explorer` and `diagram_editor` sessions use the driver
  for widget clicks, keys, chords, text and settling, one flush per settle
  (lead U2: the extra two flushes did nothing); the log explorer keeps its
  debounce loops as settle loops and its measuring loops, and the table
  cells and canvas keep their point-based helpers, with the canvas pause
  written as `Driver::advance`. The custom-widgets guide's close sentence
  points to admission and `HeadlessApp::admit_close`.
- Changed: `TextField`'s key list says a session-attached field claims
  Enter, Tab and Shift+Tab, and Escape outside a composition or drag, that a
  claimed key does not always report, and points to `TextField::session`
  for what each key reports.
- Added: contract tests: the runtime's `tests/testing.rs` for each driver
  action, each refusal, the frame step, one deadline jump per `advance`,
  the settle after keys and chords, and the cleanup rules; the facade's
  `surfaces` binary's `window_close` module for headless start and close
  admission, and a `can_focus` contract in its `transients` module; unit
  tests of the shared admission without and with settling, of the native
  runner's backoff check, reactivation records and retired-window error,
  and of `run`'s terminal-error precedence, now a function of its own; and
  a native check that delivers the window system's `CloseRequested` for a
  real main window while an editor holds a draft.
- The `api/` listings add `pub mod pert_ui_runtime::testing` and
  `pub mod pert_ui::testing` with their items and re-exports,
  `pub mod pert_ui::guide::testing`, `App::headless` and `HeadlessApp` with
  `native`, `pert_ui::testing::CpuRenderer` with `cpu`, and
  `Runtime::can_focus`. The runtime listings repeat each re-exported item's
  block under its `testing` path.

## 0.3.67 (2026-10-23)

Prelude and `component` module (API audit fix plan, part 6e). The prelude
holds what its membership rule names, as decided, and a new
component-authoring module holds every name implementing `Component`
writes. Not breaking: every change adds a re-export.

- Added (G5-7 to G5-24, G5-32, F3): names in the runtime prelude, and so in
  `pert_ui::prelude`:
  - keyboard shortcuts, commands and held keys: `Modifiers`, `KeyChord`,
    `CommandKey`, `CommandScope`, `CommandId`, `CommandReceipt`,
    `CommandStatus`, `CommandSource`, `CommandContext`, `CommandEntry`,
    `CommandOutcome`, `HeldInputId` and `HeldCancellation`;
  - reordering: `ListDrop`, `ListDropPosition`, `ListDropPoint`,
    `ListDropPlacement` and `ListDropMark`;
  - the live widgets: `LevelMeter`, `Trace` and `MeterAxis`;
  - parameters and bindings: `PowerExponent`, `EditOutcome`,
    `PreviewOutcome`, `ParameterSnapshot`, `EditToken`, `ParameterValue`,
    `Revision`, `ParameterFamily`, `DeliveryStatus` and `AuthorityStatus`;
    the erased forms `AnyParameter`, `AnyParameterHost`, `AnySpec` and
    `AnyValue`;
  - the host sink and recovery types (decision G5-16): `DeliveryMonitor`,
    `DeliverySnapshot`, `TerminalEvent`, `PreviewEvent`, `SinkError`,
    `SinkErrorKind` and `DeliveryResolution`;
  - layout, theme and drawing values: `ViewScale`, `QuarterTurn`,
    `OpaqueColor`, `MaskImage`, `ImageId`, `MaskId`, `VectorId`,
    `VectorGeometry` and `TextLayout`;
  - typography: `FontManager`, `IconManager` and `IconGlyph`;
  - widget configuration and results: `FilterCounts`, `FilterOutcome`,
    `FilterTarget`, `EditorSession`, `EditorStart`, `ListingErrorKind`,
    `LinkTarget` and `WindowPlacement`;
  - the workspace document: `WorkspaceSettings`, `WorkspaceValue` and
    `IgnoredEntry`;
  - `ApplicationError` (decision G5-32), which an application constructs
    to report its own failure.
- Added: by the same rule, beyond the audit's lists:
  - `FocusDirection`, which `Component::tab_navigation` names;
  - `ExpansionChange`, which `Accordion::on_change` reports;
  - `Expansion`, which `ToolWindowChange` carries;
  - `AnyTerminalEvent`, `AnyPreviewEvent` and `AnyDeliveryMonitor`, which
    `AnyParameterHost`'s sinks take and return, as their typed forms do;
  - `AnyParameterSnapshot`, which `AnyParameter::snapshot` returns.
  - the remote-authority host by decision G5-16's "hosts" rule:
    `RemoteHost`, which `Runtime::remote_parameter` returns and an
    application keeps; `RemoteCommit`, which its submission sink receives,
    `respond` and `retry_submission` take and `AuthorityStatus` carries;
    `AuthorityResponse`, which `respond` takes; `ResponseDisposition`,
    which `respond` returns; `SubmissionDelivery`, the status
    `submission_delivery` returns; and `AuthorityReceipt`, the receipt
    `RemoteHost::receipt` returns.

  `SelectionExtension` stays at the root with `TextSelection`, the only
  type whose methods name it.
- Added (G5-26, G5-27, G5-14): the module `pert_ui_runtime::component`,
  re-exported as `pert_ui::component`, for `use pert_ui::component::*`
  beside the prelude. It holds `Component`, its contexts and every name an
  implementation has to write: an argument it constructs, an enum it
  matches (including a kept receipt's status and the outcome that status
  carries), a field or payload it builds for a `Component` method, and a
  value it keeps where the rustdoc says to keep it. Two globs re-exporting
  one item do not conflict. Its names:
  - the trait and its contexts: `Component`, `UpdateContext`,
    `EventContext`, `MeasureContext`, `ArrangeContext`, `ChildInfo`,
    `ChildReveal`, `ComponentDetails`, `ContentRows`, `Invalidation`,
    `Notifier`, `WidgetId`, `Element`, `Signal`, `ReadSignal`,
    `MotionPreference`, `RuntimeError` and `DrawError`;
  - layout: `Constraints`, `Size`, `SizeLimits`, `Rect`, `Point`,
    `Viewport`, `ViewTransform`, `ViewScale`, `Transform2D`, `SurfaceKind`,
    `SurfaceDismissal` and `CursorIcon`;
  - input and commands: `PointerEvent`, `PointerKind`, `PointerButton`,
    `Magnification`, `KeyEvent`, `KeyCode`, `KeyAction`, `KeyTransition`,
    `KeyboardState`, `Modifiers`, `FocusDirection`, `InputOutcome`,
    `CommandKeyInput`, `CommandKey`, `TextCommand`, `TextCommandOutcome`,
    `TextInputState`, `TextKeyOutcome`, `TextPreedit`, `CommandId`,
    `CommandSource`, `CommandContext`, `CommandReceipt`, `CommandStatus` and
    `CommandOutcome`;
  - dragging: `DragEvent`, `DragOutcome`, `DragPayload`, `DragPhase`,
    `DragRole`, `DropOperation`, `DropOperations`, `DropResponse`,
    `DragPreview` and `DRAG_THRESHOLD`;
  - drawing: `Painter`, `DisplayList`, `DrawCommand`, `Color`, `ColorRole`,
    `MetricRole`, `Length`, `Shape`, `Path`, `PathBuilder`, `PathElement`,
    `FillRule`, `LineCap`, `LineJoin`, `StrokeStyle`, `QuarterTurn`,
    `TextStyle`, `TextOverflow`, `TextLayout`, `RichText`, `IconSource`,
    `Icon`, `RasterImage`, `ImageSampling`, `MaskImage`, `ImageId`,
    `MaskId`, `VectorId` and `VectorGeometry`;
  - the inspection description: `Semantics`, `SemanticRole`,
    `SemanticValue`, `SemanticAction`, `SemanticItem`, `InspectValue`,
    `ValidationHints`, `OverlapPolicy`, `ShortcutBinding`, `ShortcutScope`,
    `BoxModelInspection` and `BorderInspection`;
  - workspace state: `ViewState`, `WorkspaceValue`, `WorkspaceError` and
    `RestoreRejection`;
  - receipts and native windows: `TaskReceipt`, `TaskStatus`,
    `RevealReceipt`, `RevealStatus`, `RevealOutcome`, `RevealRefusal`,
    `PresentationReceipt`, `PresentationStatus`, `NativeWindowOptions`,
    `DetachedClosePolicy` and `NativeWindowId`;
  - editor sessions: `EditorSession`, `EditorStart`, `EditorAttachment`,
    `EditorTrigger` and `EditorOutcome`.

  `FittedText`, `EditorCompletion` and `VectorStyle` are only received, so
  not required; `ProfileSnapshot` and `EditSource` stay at the root.
- Added (G5-28, G5-30): `NativeWindowId` and `WindowError` in
  `pert_ui::prelude` with the `native` feature, so the prelude holds `App`
  and everything it reports, errors included.
- Changed (G5-11, G5-16, G5-25, G5-26, G5-30, G5-32): the prelude pages'
  membership rule names hosts, local and remote, with the types their sink,
  response and recovery methods take and return and the receipts and status
  values they hand out, the
  errors an application constructs (`SinkError`, `ApplicationError`) and,
  in the facade, `WindowError`. The exclusions read "inspection queries and
  records, profiling, capture, live-data plumbing and host transport", and
  one sentence keeps a live widget's `SeriesSource` and the bridge that
  feeds it at the root. The custom-component sentences of both prelude
  pages and the crate page point to `component`, and the pages' lists of
  root names to add keep only the live-data plumbing and the `component`
  module.
- Changed: the custom-widgets guide chapter opens with an "Imports"
  section: application code globs the prelude, and a widget that describes
  itself, starts a drag, keeps a receipt, presents in its own native window
  or hosts an editor session also globs `pert_ui::component`, with an
  example and a link to the module's page.
- Added: contract tests in the facade's `foundation` binary: the prelude
  test names each added prelude name and overrides
  `Component::tab_navigation`, a native-only test names what `App`
  reports, and a component written from `pert_ui::component::*` alone
  keeps a task receipt, starts a drag, claims Tab, describes itself for
  inspection, builds detached-window options and matches a reveal refusal,
  and saves its workspace state; a second module imports each `component`
  name on its own. Each module has a running rustdoc example.
- The `api/` listings add `pub mod pert_ui_runtime::component` and
  `pub mod pert_ui::component` with their re-exports, the new prelude
  re-exports, and the facade prelude's `NativeWindowId` and `WindowError`
  with `native`. The runtime listings repeat each re-exported item's block
  under its new `prelude` and `component` paths.

## 0.3.66 (2026-10-23)

Editors, data and defects (API audit fix plan, part 6d2). Fallible
signatures where a value could be lost or coerced, the number inputs'
refusals reported to the application, and defects in focus, signals,
inspection, capture, tables and the conformance check fixed. Breaking in
signatures and behavior, and in the inspection schema, which advances to 15
by the maintainer's decision.

- Breaking (G1-4): `Workspace::set_settings` returns
  `Result<(), WorkspaceError>` and is no longer `const`. Settings that would
  take the encoded document past `Workspace::MAX_DOCUMENT_BYTES` are refused
  with `WorkspaceError::TooLarge`, keeping the previous settings, so a
  document the library accepted always decodes. Replace
  `workspace.set_settings(settings);` with `workspace.set_settings(settings)?;`.
- Breaking (G1-11): `TextSelection::new` returns
  `Result<Self, TextEditError>`. An offset past the end of the text or inside
  a grapheme is rejected with `TextEditError::InvalidSelection`, carrying both
  offsets, instead of being clamped or snapped; a backward selection, with the
  anchor after the focus, stays valid. Replace
  `TextSelection::new(&layout, anchor, focus)` with
  `TextSelection::new(&layout, anchor, focus)?`, and pass the text's length
  rather than `usize::MAX` for its end. `TextEditError::InvalidSelection`'s
  rustdoc and message ("selection {anchor_byte}..{focus_byte} is not a valid
  range of boundaries in the text") cover both its uses.
- Breaking (G4-26): `CaptureJob`'s fields are private, read through
  `CaptureJob::id`, `CaptureJob::window` and `CaptureJob::include_overlay`;
  only the runtime creates jobs. A job carries the identity of the request it
  was listed for, and `Runtime::capture_scene` looks it up among the pending
  captures by that identity and renders what the request asked for; a
  completed job, or one another runtime listed, is refused with
  `CaptureError::UnknownTarget`, even when its id, window and overlay flag
  match a pending request's, as two runtimes' first requests do. `CaptureJob`
  equality follows that identity, and its `Debug` shows only the three
  public observations.
- Breaking (G4-26): `Runtime::complete_capture` takes the job,
  `&CaptureJob`, instead of its `u64` id, and identifies the request as
  `capture_scene` does, so a completed job, or one another runtime listed
  with the same id, completes nothing and returns `false` instead of
  answering another request. Replace
  `ui.complete_capture(job.id(), frame)` with
  `ui.complete_capture(&job, frame)`; the facade's native presenter and
  `CpuRenderer::serve_captures` do so. Replace `job.id`, `job.window` and
  `job.include_overlay` with `job.id()`, `job.window()` and
  `job.include_overlay()`.
- Breaking in behavior (G2-20, F6): `IntegerInput`, `DecimalInput`,
  `InlineInteger` and `InlineDecimal` report their own refusals of a changed
  draft through `on_rejection`, as `TextField` reports its length limits:
  text that does not parse, a number outside the bounds and, for a
  parameter, a number off its step, each as an `InputRejection` whose message
  comes from the installed `UiStrings`. A step that saves at once (a step
  button, or an inline row's resting step) and whose normalized value the
  field refuses, such as a normalizer returning 100 in a field bounded to
  0..10, is reported the same way instead of silently. An `on_rejection` that published
  every report as a `FieldStatus` now also shows these, with no further code;
  one that expected only the normalizer's refusals checks the message.
  Inspection's `validation_error` keeps the `NumberInputError` text.
  `docs/contracts.md` "Text and draft editing" states the one rule for
  `on_rejection`, and `docs/manual.md` mentions the field's own refusals.
- Added (G2-20): `StringRole::NotANumber` ("Enter a number"),
  `StringRole::NotAWholeNumber` ("Enter a whole number"),
  `StringRole::NumberOutOfRange` ("Enter a number from {min} to {max}") and
  `StringRole::NumberOffStep` ("Enter a number in steps of {step}"), with
  `UiMessage::NumberOutOfRange { min, max }` and
  `UiMessage::NumberOffStep { step }` for the two templates. The gallery's
  Swedish table translates the four roles.
- Breaking in behavior (F21): a naming parent that reports itself invalid,
  such as a `Field` with an error status, or whose description is a warning,
  such as a `Field` with a warning status, gives its description to a child
  even when the child describes itself, so a refused draft's control reports
  the field's error or warning rather than "Unsaved changes". A parent whose
  description is ordinary text, such as help, still leaves a child's own
  description. `Semantics::names_children`, `Field` and `docs/contracts.md`
  "Inspection and diagnostics" say so.
- Breaking, inspection schema (F21, maintainer's decision): the new public
  field and builder `Semantics::warning` mark a description as a warning; a
  `Field` sets it while its warning status has a message, the mark travels
  with the description a naming parent gives its children, and a snapshot
  drops it while the description is absent. Custom components reach the same
  precedence by setting it. `InspectionSnapshot::SCHEMA_VERSION` is 15
  (was 14): the serialized semantics require the field, so a schema 14
  snapshot does not decode as this revision. Code that builds `Semantics`
  through `Semantics::new` and its builders is unaffected; a tool that
  decodes snapshots accepts revision 15.
- Breaking in behavior (F22): outside macOS, a single-line editor
  (`TextField`, and the number, search, inline and draft editors built on
  it) leaves Up and Down with Alt or Super to its ancestors and command
  bindings, where they have no editing meaning; a `TextArea` still moves by
  line with them. An ancestor that handles these chords now receives them
  from an editor inside it: Alt+Down in a field inside a reorderable
  `VirtualList`'s row moves the list's active row. `TextField`'s new "Keys"
  section lists the chords the editors claim on each platform, and what a
  disabled, composing or read-only editor claims instead, and `TextArea`
  refers to it.
- Breaking in behavior (F29): an empty plain `Label`, with no icon, measures
  as zero size, as an empty `Field` block does; a container's gap beside it
  remains.
- Breaking in behavior (F26, F31): `CollectionFilter`'s search field sits in
  the slot `filter-field`, as its rustdoc said, so its address is
  `…/filter-field/filter` instead of `…/filter`, and a collection keyed
  `filter` no longer shares the field's address. Replace
  `ui.find(&root, &["tasks", "filter"])` for the field with
  `ui.find(&root, &["tasks", "filter-field", "filter"])`. The field keeps no
  workspace state, so saved workspaces are unaffected.
- Fixed (N:focus-reveal): `Runtime::focus` and the other focus transfers that
  reveal their target queue a `focus_lost` or `focus_within_changed` error for
  a later flush when revealing fails, instead of dropping it; the reveal
  error is returned.
- Fixed (N:publish-revision): signals the library publishes itself, such as
  `Parameter::snapshot`, advance `Signal::revision` like any other changed
  write, so a `Dynamic`, `Show` or `Each` over one describes again, for
  example during a slider gesture. Live-data sources' presentation
  publication advances it too.
- Fixed (G4-25): an inspection snapshot keeps `Semantics`' and
  `SemanticItem`'s documented invariants when a component assigned their
  public fields directly: a protected value is withdrawn, an empty name or
  description is absent, a warning mark without a description is dropped,
  and actions are sorted, distinct and absent while disabled.
- Fixed (V3-2): restoring a workspace refuses a saved table column width
  below `TableColumn::MIN_WIDTH`, which capture never writes, with
  `RestoreRejection::OutOfRange` naming the column, instead of raising it to
  the minimum and recording it as the user's. The stored number is checked
  before it is narrowed to `f32`, so a width just below the minimum that
  would round up to it is refused too, as is one beyond the largest `f32`.
- Fixed (F25): `check_redescription` mounts its reference tree under a root
  that declares the overlap with the re-described tree intentional, so a
  runtime with a viewport no longer logs a false `UnexpectedOverlap`; its
  rustdoc points to `Redescribed::component`.
- Changed (V3-1): `TableColumn::MIN_WIDTH` states the one exception, a
  column that starts at the saturated largest finite position and so has a
  width of 0, and the layout comment says the same.
- Changed: `TextSelection`'s rustdoc describes caret stops as the shaped
  text's grapheme boundaries, without claiming anything about ligatures.
- Changed: the `api/` listings of `pert-ui-runtime` show the changed
  `Workspace::set_settings`, `TextSelection::new` and
  `Runtime::complete_capture`, `CaptureJob`'s accessors in place of its
  fields with its `PartialEq`, `Eq` and `Debug` implemented by hand (no
  longer `StructuralPartialEq`), the new
  `Semantics::warning` field and builder, and the new `StringRole` and
  `UiMessage` variants. The listings do not show constant values, so
  `InspectionSnapshot::SCHEMA_VERSION`'s change to 15 does not appear in them.
- Added: tests for a reveal error with a held focus error and the
  `Dynamic` over a parameter snapshot (`controls`), the settings boundary
  round trip (`surfaces`), explicit selections, the number inputs'
  refusals with English and replaced strings, an immediate step normalized
  out of range and the single-line chords (`editing`), directly assigned
  semantics, a field error or warning over a draft's description, a job
  from another runtime with identical public observations, which neither
  renders nor completes the local request, and the warning
  field's round trip and closed schema (`foundation`), the column width
  rejection just below the minimum, the saturated widths, the filter
  field's address and a row field's Alt+arrow reaching its reorderable list
  (`collections`), the empty label (`presentation`), and the conformance
  check's validation output (a `pert-ui-runtime` unit test).

## 0.3.65 (2026-10-23)

Redescription and reconciliation (API audit fix plan, part 6d1). Describing
a mounted collection again now applies its new callbacks and permissions, as
every other component does; the controller, source binding and structural
options stay. Reconciliation publishes no signal except one documented
parameter lifecycle cleanup, and three smaller redescription defects are
fixed. Breaking in behavior; no signature changes.

- Breaking (G2-2, G2-14): `VirtualList`, `Tree` and `Table` adopt a
  compatible redescription's callbacks and permissions, including their
  removal, where they kept the first description's: `VirtualList`'s
  `on_activate`, `on_text_input`, `on_selection_change`, `reorderable`,
  `placement`, `reorder_permission` and `drag_operations`; `Tree`'s
  `on_activate`, `on_selection_change`, `reorderable`, `reorder_permission`
  and `drag_operations`; `Table`'s `on_activate`, `on_selection_change`,
  `on_sort`, `cell_editor`, `resize_permission` and `reorder_permission`.
  The controller, source binding and structural options (a list's axis,
  lanes, overscan, selection, `active_outline`, `item_tab_stops` and
  `keep_mounted`, a table's row height and `cell_tab_stops`) stay. A
  description without `reorderable`, or with a fixed denial, cancels an armed
  or active reorder, column move or resize at the update the redescription
  schedules, as a revocation does; a removed `on_selection_change` discards
  the changes it has not delivered, and a reinstalled one reports from the
  selection it first sees; a table's new `cell_editor` builds the edits that
  begin afterwards, while an open edit keeps its editor. Code that relied on
  the first description's callbacks surviving now describes them every time,
  for example `VirtualList::new(&items, 24.0)?.on_activate(open)` in each
  description rather than only in the first; code that redescribed a
  collection to change its restriction no longer needs to bind it to a signal
  first, and `reorder_permission(false)` in a later description now locks a
  mounted list.
- Breaking (G2-2): `FileBrowser::folder_filter` follows a redescription on a
  browser mounted with it: a later description's permission replaces it, and
  a description without `folder_filter` denies it, hiding the field. Whether
  the tree has the field stays fixed at mount, since it is part of the
  browser's documented child structure; mount the browser with
  `folder_filter(false)` to be able to offer it later.
- Changed (G2-13): a test shows that when reconciliation cancels a parameter
  gesture, a component and a derived binding reading the parameter's
  snapshot both settle in the reconciling flush, so the immediate
  publication is documented as the one lifecycle exception: `docs/contracts.md`
  "Component redescription" and the number inputs' reconciliation comment say a
  rebind or a fixed denial cancels the gesture during reconciliation and
  publishes the snapshot at once, with the terminal event at the next safe
  point, and AGENTS.md's reconciliation rule refers to that exception.
  `ContextMenu` keeps whether any row can run as plain shared state instead of
  a private signal its reconciliation published; its controllers behave as
  before.
- Fixed (G1-1): `RadioGroup`, `SegmentedControl` and `InlineChoice` keep their
  name and selection when a redescription changes their value binding, until
  the new binding's option list validates, so an invalid list leaves the last
  valid presentation; input aimed at the old binding is still cancelled.
- Fixed (G2-12): `RegionLayout` gives its dividers a forwarding `on_resize`
  only while it has a callback, so a description without one drops the
  changes they recorded, and a callback installed again before delivery no
  longer receives a change recorded earlier.
- Changed: `docs/contracts.md` "Component redescription" narrows the
  controller exception to the controller, source binding and structural
  options, and the rustdoc of `VirtualList`, `Tree`, `Table`, `FileBrowser`
  and their affected builders describes what follows a redescription.
  `AppError`'s rustdoc says an error that is not `Send + Sync`, rather than
  not `Send`, cannot be carried.
- Changed: the `api/` listings of `pert-ui-runtime` show
  `Component::reconcile` overridden for `VirtualList`, `Table`, `Tree` and
  `FileBrowser`: their `reconcile` lines lose the default method's
  `where Self: Sized`. No signature changes.
- Added: tests for the replaced and removed callbacks of the three
  collections (`collections` binary), a removed-then-reinstalled selection
  callback before delivery, adopted permission bindings and fixed denials
  that cancel list, tree and column gestures (`audit`), the file browser's
  folder filter, the choice controls' rebinding with an invalid list
  (`controls`), a region callback removed and reinstalled before delivery
  (`surfaces`), and G2-13's dependency test for number inputs and sliders
  (`foundation`).

## 0.3.64 (2026-10-23)

Compiler diagnostics (API audit fix plan, part 6c2). The compiler messages
for the mistakes the diagnostics survey measured now name pert-ui's intent
instead of a private path or the wrong type, and a new snapshot check keeps
them that way on the pinned toolchain. Not breaking: the renamed trait is
private.

- Changed: `Component`, `ListItems`, `TreeItems`, `TableItems`,
  `ContentRows`, `Clipboard` and `StateOwner` carry
  `#[diagnostic::on_unimplemented]`. A type that does not implement one of
  them is reported with a headline such as ``Vec<String>` cannot be used as
  list rows`` and notes naming the required methods; an `Element` passed
  where a component is expected is told that it is already described. The
  notes hold for every type that can fail the bound.
- Changed: the private seal of `StateOwner` is renamed from
  `signal::sealed::Sealed` to `signal::sealed::StateOwnerSeal` and carries
  its own message, so implementing `StateOwner` reports
  ``StateOwner` is sealed and cannot be implemented for `MyOwner`` with the
  types that own state, and no longer points at a dependency's `Sealed`
  trait. The `api/` listings show the changed supertrait path of
  `StateOwner` (at the root and in the prelude). `StateOwner`'s rustdoc has a
  `compile_fail` example of the rejected implementation, and its summary
  names `DescribeContext` among the owners.
- Changed: the private seals of `pert_ui_core`'s other sealed traits get
  distinct names and their own messages: `family::sealed::Sealed` is now
  `family::sealed::ParameterValueSeal` and `live::item::sealed::Sealed` is
  now `live::item::sealed::LiveItemSeal`. Implementing `ParameterValue` or
  `LiveItem` reports that the trait is sealed for that type and names the
  types that implement it (`f32`, `i64`, `bool` and `ChoiceId`; `f32`, `f64`,
  `i32`, `i64`, `u32` and `u64`); the snapshot cases `p01` and `v01` cover
  both messages. The `api/` listings show the changed
  supertrait paths of `ParameterValue` and `LiveItem` and the changed
  projection path in `ParameterSpec`'s and `ParameterState`'s auto-trait
  bounds.
- Changed (F11): `impl<E> From<E> for AppError` carries
  `#[diagnostic::do_not_recommend]`, so `AppError::from` or `?` on an error
  it does not accept, such as `std::io::Error`, reports the missing
  `From` for `AppError` rather than for `DrawError`. `AppError`'s rustdoc
  lists the accepted errors, which rustc no longer lists, and points to
  `ApplicationError::new` for any other error.
- Changed: `Modifiers`' fields carry rustdoc search aliases (`control`:
  `CONTROL`, `CTRL`; `alt`: `ALT`, `OPTION`; `super_key`: `SUPER`, `META`,
  `WINDOWS`), and its rustdoc says there are no per-key constants, shows
  `Modifiers { control: true, ..Modifiers::default() }`, and says
  `PRIMARY` is Control on Windows and Linux and Command on macOS.
  `PRIMARY` has no alias, since Control means Command there only on macOS.
- Tooling: the new unpublished workspace member `pert-ui-diagnostics`
  (`tests/diagnostics`) compiles one case per message pert-ui shapes against
  the facade and compares rustc's output with committed `trybuild` `.stderr`
  snapshots. Its test is ignored in ordinary runs, which compile it and skip
  it; CI's new `diagnostics` job runs `cargo test --locked -p
  pert-ui-diagnostics -- --ignored` on Linux with the pinned toolchain, and a
  toolchain update regenerates the snapshots with `TRYBUILD=overwrite`.
  `trybuild` 1.0.122 is its only dev-dependency; adding it locks
  `toml_datetime` 1.1.2 and `toml_parser` 1.1.5, which `toml_edit` shares;
  winit uses that only on Android, through `proc-macro-crate`. The packaging commands in CI, AGENTS.md, the README
  and the usability kit exclude the member. AGENTS.md and
  `docs/verification.md` describe the check and say that a `compile_fail`
  doctest does not check its error code on stable Rust.

## 0.3.63 (2026-10-23)

Error values (API audit fix plan, part 6c1). An error that rejects a key,
identity, path or integer value now carries it and names it in its message,
causes appear once in a source chain, and the frame image limit has one
variant. Argument variants the caller still holds stay unit:
`GeometryError`'s, `LayoutError::{InvalidWeight, InvalidRatio}` and
`ParameterError::{InvalidRange, OutOfRange, OffLattice}`. By the
maintainer's decision of 2026-10-23, a variant that rejects a floating-point
value also stays unit, so every error keeps `Eq`:
`NormalizedValueError::OutOfRange`, `PathError::InvalidParameter` and
`NumberInputError::{InvalidRange, InvalidStep, OutOfRange, OffStep}` are
unchanged. The error policy in `docs/contracts.md` states the rule. Breaking.

- Breaking: `RuntimeError::AmbiguousKey`, `DuplicateKey` and
  `ChildTypeChanged` hold the key as a `String`, and `ShortcutConflict` holds
  the conflicting `KeyChord`. Match `RuntimeError::AmbiguousKey(key)` or
  `RuntimeError::AmbiguousKey(_)`; `Runtime::find` reports an ambiguous
  address with its segments joined by `/`. A held-key registration reports
  its key with the registration's modifiers, and a second `ToastHost` on one
  queue reports `DuplicateKey("toasts")`, the key of the part that shows the
  toasts. The messages name the value (`several descendants share the key
  `apply``).
- Breaking: `LayoutError::DuplicateKey`, `ReservedKey` and `UnknownKey` hold
  the key: match `LayoutError::UnknownKey(key)`. `LayoutError` is no longer
  `Copy`; clone a stored one.
- Breaking: `FontError::UnknownIcon` holds the name and `MissingGlyph` the
  `char`: match `FontError::UnknownIcon(name)`. `FontError` is no longer
  `Copy`.
- Breaking: `ParameterError::DuplicateOption` and `UnknownOption` hold the
  `ChoiceId`: match `ParameterError::UnknownOption(id)`. `ParameterError` is
  no longer `Copy`, and neither are the types that hold it: `EditError`,
  `BindingError` and `EditOutcome<T>` for every `T`. Clone an outcome read
  through a reference, as `event.outcome.clone()` in an edit sink.
- Breaking: `ChoiceError::DuplicateValue` is
  `DuplicateValue { first, second }`, the zero-based positions of the two
  equal options or sibling menu keys: match `DuplicateValue { .. }`.
  `ChoiceError` stays `Copy`.
- Breaking: `LiveError::CapacityExceeded` holds the requested item count:
  match `LiveError::CapacityExceeded(_)`.
- Breaking: `NumberInputError::InvalidPrecision` holds the rejected digit
  count as a `u8`: match `NumberInputError::InvalidPrecision(_)`.
- Breaking: `NumberLocaleError::SameSeparators` is
  `SameSeparators { decimal, grouping }`, the rejected separators: match
  `NumberLocaleError::SameSeparators { .. }`.
- Breaking: `TextEditError::InvalidComposition` is
  `InvalidComposition { start_byte, end_byte }` and `InvalidSelection` is
  `InvalidSelection { anchor_byte, focus_byte }`, the rejected offsets: match
  `TextEditError::InvalidSelection { .. }`.
- Breaking: `pert_ui::images::ImageError::Io` is `Io { path, source }`,
  naming the file `images::load` could not read, and
  `From<std::io::Error> for ImageError` is removed; match
  `ImageError::Io { .. }`.
- Changed: `ListingError` names the path it could not read:
  `ListingError::with_path` sets it, `ListingError::path` reads it, and
  `list_directory` names the directory, or an unreadable entry's path, on
  every error it returns. Its `Display` writes `path: message` when it has a
  path; the browser still shows `ListingError::message`. `ListingError` is no
  longer derived with `thiserror`; it implements `Display` and `Error` by
  hand.
- Breaking: `DrawError::InvalidImage` is split by cause.
  `InvalidImage { width, height, bytes }` is now only the rejection by
  `MaskImage::new` and `RasterImage::from_srgba8`, holding what was passed:
  match `DrawError::InvalidImage { .. }`. A coverage mask that drawing needs
  beyond `MaskImage::MAX_PIXELS` (a shape, an arc, a vector at its raster
  scale) is the new unit `DrawError::CoverageTooLarge`. A command naming a
  resource its display list does not hold, reported by
  `VectorRasterizer::prepare` and composition, is the new
  `DrawError::MissingVector(VectorId)`, `MissingMask(MaskId)` or
  `MissingImage(ImageId)`, whose message names the ID's index. Vector
  coverage is cropped to its clip before the limit is checked; shape and arc
  masks are checked at full size, so a clip does not avoid
  `CoverageTooLarge` for them. Code that matched `InvalidImage` for an oversized
  stroke or arc matches `CoverageTooLarge`.
- Changed: `WorkspaceError::Participant`'s message no longer repeats its
  source (`workspace participant `main/table` could not build its state`),
  and `RestoreRejection::State` and `RestoreRejection::Runtime` expose their
  cause through `source()` (`RestoreRejection`'s `Error` impl now has one)
  instead of appending it to the message, so a chain prints each cause once.
- Breaking: `FrameImage::MAX_BYTES` has one variant, `LimitError::FrameSize`,
  whose message is now `a W×H frame exceeds the frame image size limit`.
  `LimitError::CaptureSize` is removed: match `LimitError::FrameSize`.
  `CaptureError::TooLarge` is replaced by `CaptureError::Limit(LimitError)`:
  match `CaptureError::Limit(LimitError::FrameSize { width, height })`.
  `pert_ui_cpu::CpuRenderError::TooLarge { width, height }` is replaced by
  `CpuRenderError::Limit(LimitError)`, and a frame buffer the system cannot
  allocate is the new `CpuRenderError::AllocationFailed { bytes }` rather
  than a size error. `LimitError`'s rustdoc names who reports the limit.
- Changed: with the `serde` feature, `LimitError` and `ResourceKind`
  implement `Serialize` and `Deserialize`, and `CaptureError`'s serialized
  form changes: `"TooLarge"` becomes
  `{"Limit":{"FrameSize":{"width":…,"height":…}}}`. This is a transport form
  outside any versioned schema; `InspectionSnapshot::SCHEMA_VERSION` and
  `Workspace::SCHEMA_VERSION` are unchanged.
- Added: the runtime's `errors` contract tests check that a rejected key,
  integer or limit is named in the message, a `vector_scene` unit test that a
  missing reference names its ID, and that the new causes print once
  in a source chain; the facade's `presentation` binary checks
  `ImageError::Io`'s path and source, its `collections` binary
  `list_directory`'s path, and its `foundation` binary `CaptureError::Limit`'s
  serialized form.

## 0.3.62 (2026-10-22)

`Debug` for descriptions; the lint denied (API audit fix plan, part 6b3).
With part 6b2 every public type implements `Debug`, and the workspace now
denies `missing_debug_implementations`, so a new public type without it fails
the build. Each new impl is written by hand and ends with `..`. A description
shows only its type, or the key of a keyed item; it formats no content,
binding or callback, reads no signal and puts no `Debug` bound on its type
parameter. Not breaking.

- Added: `impl Debug` for `Element`, showing its key, its fixed label (`None`
  for a bound one) and its children, so a described tree formats as its keys.
- Added: `impl Debug` for the keyed item descriptions `Tab`, `Section`,
  `MenuItem`, `Property`, `ToolGroup`, `ToolWindow` and `FloatingWindow`,
  showing their key.
- Added: `impl Debug`, showing only the type, for the widget and container
  descriptions `Accordion`, `AlertDialog`, `Banner`, `Breadcrumb`, `Button`,
  `Checkbox`, `Chip`, `Collapsible`, `CollectionFilter`, `ColorPicker`,
  `ColorSwatch`, `Column`, `CommandPalette`, `Container`, `ContextMenu`,
  `DecimalInput`, `Dialog`, `Divider`, `DragPreview`, `Field`, `FileBrowser`,
  `Form`, `Grid`, `Group`, `Image`, `InlineDecimal`, `InlineInteger`,
  `InlineReadout`, `InlineSlider`, `InlineText`, `InlineToggle`,
  `IntegerInput`, `Knob`, `Label`, `LevelMeter`, `Link`, `Menu`, `MenuBar`,
  `Popover`, `ProgressBar`, `PropertyGrid`, `QuickPick`, `RegionLayout`,
  `Row`, `ScrollArea`, `SearchField`, `SelectableText`, `Show`, `Slider`,
  `Spinner`, `SplitPane`, `Table`, `Tabs`, `TextArea`, `TextField`, `Toast`,
  `ToastHost`, `Toolbar`, `Tooltip`, `ToolWindowLayout`, `Trace`, `Tree`,
  `View`, `VirtualList`, `WindowLayer`, `Wrap` and `ZoomView`, and for every
  `T` of `Dynamic<T>`, `Each<T>`, `HistoryButtons<T>`, `HistoryNavigator<T>`,
  `InlineChoice<T>`, `InlineSelect<T>`, `MultiSelect<T>`, `RadioGroup<T>`,
  `SegmentedControl<T>` and `Select<T>`.
- Added: `impl Debug` for `pert_ui::Svg` (`svg` feature), showing its
  intrinsic size, requested size, tint and description, not the parsed
  drawing or a prepared raster.
- Changed: the workspace denies `missing_debug_implementations`, and
  AGENTS.md's lint sentence names it.
- Added: contract tests in the facade's `foundation` binary check that the
  descriptions implement `Debug`, also over a type parameter without it, and
  how an element tree and keyed items format; the `presentation` binary checks
  `Svg`'s.

## 0.3.61 (2026-10-22)

`Debug` for state owners, handles, contexts and backends (API audit fix plan,
part 6b2). An application struct that holds one of these can now derive
`Debug`. Each impl except `FilterTarget`'s, which is derived, is written by
hand and ends with `..`. None shows more than cheap identifying fields, reads a
signal or borrows shared state (which could conflict inside a runtime phase),
formats a callback, or puts a `Debug` bound on application values. Not
breaking.

- Added: `impl Debug` for the state owners and handles `ListSelection` (its
  mode), `ListController`, `ListFilter`, `TableController` and
  `TreeController` (the last two with their selection's mode), `FieldHandle`
  (its key), `FormFields`, `ContextMenuController`, `ThemeManager`,
  `FontManager`, `IconManager`, `UiSettings`, `NumberFormat<T>` for every `T`
  (its width hint and edit unit, not its callbacks), `ChildInfo` (key,
  identity, visibility, focus and pointer capture) and `Runtime` (its primary
  window).
  `FilterTarget` derives `Debug` from its now-formattable variants.
- Added: `impl Debug` for the callback contexts `EventContext`,
  `UpdateContext` and `MeasureContext` (the last two with their child
  counts), `ArrangeContext` (child count and placements so far),
  `DescribeContext`, `CellEdit` (row, column and how the edit began) and
  `Painter` (size, clip and the number of recorded commands).
- Added: `impl Debug` for the backends `VectorRasterizer`,
  `pert_ui_wgpu::Renderer` (its configured surface extent) and `pert_ui::App`
  (title, main-window geometry and software presentation, not the runtime or
  the handlers).
- Added: contract tests in the facade's `foundation` binary check that these
  types implement `Debug` and that an application struct holding them,
  including a `NumberFormat` over a value without `Debug`, derives it;
  `pert-ui-wgpu`'s unit tests check `Renderer`.

## 0.3.60 (2026-10-22)

Lints and `#[must_use]` (API audit fix plan, part 6b1). The workspace denies
three more Clippy lints, and `#[must_use]` follows one rule: an identity whose
drop changes nothing carries none, an owner whose drop tears state down says
so, and a receipt is `#[must_use]` only where dropping it hides an outcome that
nothing else reports. No public item or signature changes; the `api/` listings,
which do not show `#[must_use]`, are unchanged. Not breaking, but code that
discards a signal, a parameter host or a series source now sees an
`unused_must_use` warning.

- Changed: the root `[workspace.lints.clippy]` denies
  `clippy::missing_panics_doc`, `clippy::return_self_not_must_use` and
  `clippy::fn_params_excessive_bools`, and AGENTS.md's lint sentence names
  them. `ThemeMetrics::for_density` carries the one
  `#[expect(clippy::missing_panics_doc)]`: its assertions check the fixed
  default table, which is evaluated for every density at compile time.
- Changed: `CommandId` and `HeldInputId` are no longer `#[must_use]`.
  Dropping either keeps the registration, as `Runtime::register_command` and
  `Runtime::register_held_input` now say, so an application-lifetime command
  needs no `let _ =`; the shortcut example in the guide's diagnostics chapter
  compiles without a warning.
- Changed: `ToastReceipt` is no longer `#[must_use]`: a failing toast action
  propagates from the input call that activated it, and expiry or dismissal
  is presentation. The
  guide's notification example, the runtime README, the
  `ToastHost::dismiss_label` example and the gallery's surfaces page push
  toasts as plain statements.
- Changed: `TaskReceipt`'s `#[must_use]` reason says the receipt is the only
  report of a `Cancelled` operation, and `EventContext::defer` says a dropped
  receipt still surfaces `Failed` through the flush but `Cancelled` only
  through the receipt. The public naming rules in `docs/contracts.md` state
  the receipt rule.
- Changed: `ParameterHost`, `AnyParameterHost` and `SeriesSource` are
  `#[must_use]` with a reason, and so is `Runtime::series_source`: dropping a
  host removes its parameter and releases its sinks, and dropping the last
  clone of a series source closes its bridge.
- Changed: `Runtime::signal`, `UpdateContext::signal`, `EventContext::signal`,
  `StateOwner::signal` and `Signal::sibling` are `#[must_use]`; a signal
  created and discarded is dropped at once.
- Changed: the bare `#[must_use]` on `PresentationRequest`, `PreparedDetach`,
  `PresentationReceipt` and `CommandReceipt` gains a reason: dropping an
  unfinished request cancels it, dropping a prepared detach leaves the
  movement uncommitted, and the two receipts are the only report of their
  request's result (for `PresentationReceipt`, unless the host reports window
  errors itself).
- Changed: `RevealReceipt`'s reason no longer reads as optional: the receipt
  is the only report of the reveal's outcome, including an error that stopped
  it, since the queued reveal's result is stored in it and not returned.
- Changed: repository code that bound a `CommandId`, a `HeldInputId` or a
  `ToastReceipt` only to silence the old warning (`let _ =` or an unused
  `let _name =`) now calls the function as a plain statement: the
  `diagram_editor` example's save and open failure toasts, the gallery's
  commands and held input, the multi-window smoke check and the facade's
  tests.

## 0.3.59 (2026-10-22)

Types and traits (API audit fix plan, part 6a3). Identities, keys and
validated values implement the standard traits their equality supports,
validated quantities convert into their raw numbers, and keyed descriptions
convert into elements. Inspection records that only the runtime builds become
`#[non_exhaustive]`, and every component description is now `!Send`.

- Breaking: `InspectionSnapshot`, `WindowInspection`, `ComponentInspection`,
  `ParameterInspection`, `SourceInspection`, `DragInspection`, `EventRecord`,
  `ValidationReport`, `ValidationIssue` and `ValidationMarker` are
  `#[non_exhaustive]`, and so are the enums `EventKind` and `ValidationCode`.
  Only the runtime builds them (`Runtime::inspect`,
  `Runtime::validation_report`), and they grow with
  `InspectionSnapshot::SCHEMA_VERSION`. Code outside the library can no longer
  build them with a struct expression or destructure them without `..`: read
  their fields, or decode a stored record with serde, whose representation is
  unchanged (the facade's serialization tests now decode JSON fixtures). A
  `match` on `EventKind` or `ValidationCode` needs a wildcard arm. The records
  a component builds for `Component::inspect` (`ComponentDetails`,
  `ValidationHints`, `BoxModelInspection`) stay exhaustive; their rustdoc says
  to build them from `Default` (`..ComponentDetails::default()`). The
  compatibility policy in `docs/contracts.md` states the rule, and that status
  and outcome enums are closed unless their rustdoc says they grow.
- Breaking: `Divider` and `pert_ui::Svg` are no longer `Send`. They were the
  only component descriptions that were, by accident of their fields (for
  `Svg`, of a dependency's parsed tree). Code that moves either to another
  thread, such as building an `Svg` in `std::thread::spawn` or a
  `Runtime::spawn` worker and returning it, stops compiling: return the
  source bytes, or the pixels of `pert_ui::svg::rasterize` or
  `rasterize_mask`, and build the component on the UI thread. A `compile_fail`
  example on each type checks this, and `docs/contracts.md` says component
  descriptions are UI-thread values and not `Send`.
- Added `Hash`, consistent with equality, on `CommandId`, `FontId`,
  `HeldInputId`, `KeyCode`, `Modifiers`, `CommandKey`, `KeyChord`,
  `PointerButton`, `WorkspacePath`, `ColorRole`, `MetricRole`, `FontRole`,
  `Icon`, `Revision`, `EditSequence`, `ThemeMode`, `Density`, `TextStyle`,
  `ProfileOperation`, `ValidationCode`, and `RequestTicket<K>` when `K:
  Hash`. `CommandId`, `FontId` and `RequestTicket` hash the registry or
  tracker identity their equality compares.
- Added the traits validated values support, treated as `NormalizedValue`
  is: `PartialOrd` and `Ord` on `Capacity`; `Ord` and `Hash` on
  `PowerExponent`; `Eq`, `Ord` and `Hash` on `Magnification`, `UiZoom` and
  `ViewScale`; `Eq`, `PartialOrd`, `Ord` and `Hash` on `PresentationRate`.
  The floats order numerically and hash their bit pattern; their invariants
  exclude NaN and negative zero. `PowerExponent`, `Magnification`, `UiZoom`
  and `ViewScale` keep their order, now implemented by hand.
- Added `Eq` and `Hash` on `RasterImage` and `MaskImage`, hashing the
  content key computed at construction, so equal content from different
  allocations hashes alike.
- Added `impl Display for Color`, which writes `Color::to_hex`'s text:
  quantized lowercase hexadecimal sRGB that `str::parse` reads back.
- Added `From<NormalizedValue> for f32`, `From<Capacity> for usize`,
  `From<Length> for f32`, `From<Magnification> for f32`,
  `From<ViewScale> for f32` and `From<PresentationRate> for f64`, beside
  `get()` and `hertz()`, as `PowerExponent` and `UiZoom` already had. The
  naming rules in `docs/contracts.md` say a validated quantity converts into
  its raw number with `From`.
- Added `AsRef<str>` for `ChoiceId`, so an identity can be passed where
  `impl AsRef<str>` is taken, such as `ContentQuery::new`.
- Added `From<Vec<u8>>` and `From<&'static [u8]>` for `FontData`, beside
  `FontData::from_owned` and `FontData::from_static`.
- Added `From<Tab>`, `From<Section>`, `From<Property>` and
  `From<FloatingWindow>` for `Element`, beside their `into_element` methods,
  so `.map(Into::into)` collects them for `Runtime::reconcile_children`.
- Added `Eq` on `pert_ui_cpu::CpuRenderError` (the facade's
  `cpu::CpuRenderError`), and `Clone` and `PartialEq` on
  `RedescriptionError`.

## 0.3.58 (2026-10-22)

Names and roles (API audit fix plan, part 6a2). Reads, roles, units and
families get consistent names, boolean operation flags become enums, and the
wgpu renderer takes a typed physical extent. Every repository consumer is
migrated. Breaking changes, with their replacements:

- Breaking: reactive reads that also have an owned form are named
  `x_binding()`. `UiSettings::{zoom_signal, text_rendering_signal,
  motion_signal, strings_signal, debug_overlay_signal}` are
  `UiSettings::{zoom_binding, text_rendering_binding, motion_binding,
  strings_binding, debug_overlay_binding}`, `ThemeManager::{density_signal,
  mode_signal}` are `ThemeManager::{density_binding, mode_binding}`, and
  `DeliveryMonitor::observe` is `DeliveryMonitor::snapshot_binding`; the
  arguments and return types are unchanged. The naming rules in
  `docs/contracts.md` state the convention: `_signal` names a method that
  takes a one-way `ReadSignal`, the bare name is the owned read where both
  exist, and a type with only a reactive read keeps the bare name. The rules
  record that a retained `ReadSignal` reads the current value at each `get()`;
  `Parameter::snapshot`'s rustdoc says its signal is live and
  `AnyParameter::snapshot`'s that its result is an owned copy.
- Breaking: the role-keyed tables use noun accessors like the context readers.
  `ThemeMetrics::get(role)` is `ThemeMetrics::metric(role)`,
  `UiStrings::get(role)` is `UiStrings::text(role)`, and
  `UiStrings::with(role, text)` is `UiStrings::with_text(role, text)`.
  Validation and fallback are unchanged.
- Breaking: `UiStrings::MAX_LENGTH` is renamed `UiStrings::MAX_TEXT_BYTES`
  (still 1024), naming its unit: UTF-8 bytes, not characters.
- Breaking: `DragPayload::get::<T>()` is `DragPayload::downcast_ref::<T>()`,
  the std name for a typed downcast.
- Breaking: `EditorSession::is_open` is `EditorSession::is_active`, the word
  `DraftEditorController::is_active` uses for the same begun, unended edit.
- Breaking: `pert_ui_wgpu::Renderer::new(target, width, height, notify)`,
  `Renderer::new_surface(target, width, height)` and
  `Renderer::resize(width, height)` take a `PhysicalExtent` in place of the two
  `u32`s: `Renderer::new(target, PhysicalExtent::new(width, height), notify)`,
  `renderer.new_surface(target, extent)` and `renderer.resize(extent)`. An
  empty extent unconfigures the renderer as a zero width or height did. The
  facade's platform code uses the new signatures.
- Breaking: `QuickPick::width` and `CommandPalette::width` take a `Length`
  instead of an `f32`, like `TableColumn::width`; zero is still
  `GeometryError::Empty`, and `Length::new` rejects non-finite and negative
  values before the call. Replace `.width(640.0)?` with
  `.width(Length::new(640.0)?)?`.
  `QuickPick::DEFAULT_WIDTH` and `CommandPalette::DEFAULT_WIDTH` have type
  `Length` (520 logical pixels); read the number with `.get()`.
- Breaking: `TextLayout::paint(painter, x, y, color)` and
  `TextLayout::paint_turned(painter, x, y, turn, color)` take a `Point` origin:
  `layout.paint(painter, Point::new(x, y)?, color)`, or `Point::ZERO` for the
  local origin.
- Breaking: byte offsets are named for their unit. `TextSelection::{anchor,
  focus, range}` are `TextSelection::{anchor_byte, focus_byte, byte_range}`,
  `TextPreedit::cursor` is `TextPreedit::cursor_byte_range`, and
  `ResourceKind::scene_budget` is `ResourceKind::scene_budget_bytes`. The
  parameters of `TextSelection::new` (`anchor_byte`, `focus_byte`),
  `TextPreedit::new` (`cursor_byte_range`) and `RichText::highlighted`
  (`byte_ranges`) are renamed to match.
- Breaking: boolean operation flags become enums.
  - The new `FocusDirection` (`Forward`, `Backward`) replaces the `reverse`
    flag: `Runtime::focus_next(false)` is
    `Runtime::focus_next(FocusDirection::Forward)` and `focus_next(true)` is
    `focus_next(FocusDirection::Backward)`; `EditorTrigger::Tab { reverse }` is
    `EditorTrigger::Tab { direction }`.
  - `Component::tab_navigation(cx, reverse: bool, repeat: bool)` is
    `tab_navigation(cx, direction: FocusDirection, transition: KeyTransition)`;
    the transition is `KeyTransition::Pressed` for a press and
    `KeyTransition::Repeated` for an operating-system repeat. An implementation
    replaces `reverse` with `direction == FocusDirection::Backward` and
    `repeat` with `transition == KeyTransition::Repeated`.
  - `EventContext::request_reveal(target, focus)` is split:
    `request_reveal(target)` reveals without moving focus and the new
    `request_reveal_and_focus(target)` also focuses, mirroring
    `Runtime::reveal` and `Runtime::reveal_and_focus`.
  - The new `SelectionExtension` (`Collapse`, `Extend`) replaces the `extend`
    flag of `TextSelection::moved` and `TextSelection::at_point`: `false` is
    `SelectionExtension::Collapse`, `true` is `SelectionExtension::Extend`.
  - The new `Expansion` (`Collapsed`, `Expanded`) replaces the `expanded` flag
    of `ToolWindowChange::new`; the `expanded` field stays a `bool`.
- Breaking: `DropPosition` is renamed `ListDropPosition`, the prefix of the
  `ListDrop` family it belongs to; its variants are unchanged.
- Breaking: cloneable handles to state a mounted widget owns are
  `*Controller`. `ContextMenuOpener` is renamed `ContextMenuController`, and
  `ContextMenu::opener()` is renamed `ContextMenu::controller()`, the accessor
  name `FloatingWindow::controller` uses: replace `menu.opener()` with
  `menu.controller()`. `ToastQueue` is renamed `ToastController`;
  `ToastHost::new`'s first parameter is named `controller`.
- Breaking: `FormStatus` is renamed `FormFields`. It is application-owned, so
  it is named for what it holds rather than as a controller; the naming rules
  in `docs/contracts.md` now say that application-owned state widgets follow
  through handles the application keeps (`ListSelection`, `FormFields` and its
  `FieldHandle`s) is named for what it holds and that `*Controller` is reserved
  for state a mounted widget owns.
- Breaking: `PendingJump::status` and `PendingJump::status_binding` are
  replaced by `PendingJump::receipt`, which lends the held
  `ListRevealReceipt`: `pending.receipt().status()` and
  `pending.receipt().status_binding()`. `PendingJump` keeps its name as the
  continuation `Runtime::resume_content_jump` takes.
- Breaking: `TaskStatus::Completed` is renamed `TaskStatus::Succeeded`, the
  word `ProgressStatus` and `EventStatus` use.
- Breaking: `NativeCloseAction::Defer` carries a
  `ReadSignal<Option<WindowCloseAction>>` instead of a
  `ReadSignal<Option<bool>>`: `Some(WindowCloseAction::Close)` closes and
  `Some(WindowCloseAction::KeepOpen)` keeps the window, where `Some(true)` and
  `Some(false)` did. The `diagram_editor` example no longer needs a comment to
  say which is which.
- The naming rules in `docs/contracts.md` state the length convention:
  geometry is in logical pixels unless its name says physical; a length a
  component property takes and stores is a `Length`; the geometry value
  constructors (`Point::new`, `Size::new`, `Rect::new`) validate bare finite
  `f32` logical pixels, with zero sizes allowed; a constructor whose argument
  must be positive may take `f32` and return `GeometryError`; and geometry the
  library reports or reads back for arithmetic, such as
  `ToolWindowChange::extent`, is bare `f32` logical pixels. `Painter::rich_text`,
  `MeasureContext::rich_text` and `ToolWindowChange::with_geometry` now say
  their widths and extents are logical pixels.

## 0.3.57 (2026-10-22)

Construction and builders (API audit fix plan, part 6a1). Constructor order,
construction names, limit constants and builder signatures. Every reordered
constructor's swapped arguments have different types, so an old call fails to
compile rather than binding the wrong value. Breaking changes, with their
replacements:

- Breaking: bound state comes first after `ui`. `Checkbox::new(label, value)`
  and `Toggle::new(label, value)` are `Checkbox::new(value, label)` and
  `Toggle::new(value, label)`; `TableController::new(ui, columns, source,
  selection)` is `TableController::new(ui, selection, columns, source)`;
  `TreeController::new(ui, source, selection)` is `TreeController::new(ui,
  selection, source)`; `FileBrowserController::new(ui, roots, selection,
  request)` is `FileBrowserController::new(ui, selection, roots, request)`.
- Breaking: a keyed item's key leads and its bound state follows.
  `MenuItem::toggle(key, label, value)` is `MenuItem::toggle(key, value,
  label)`, and `MenuItem::radio(key, label, selection, value)` is
  `MenuItem::radio(key, selection, value, label)`. The naming rules in
  `docs/contracts.md` state the order: the state owner (`ui`), the stable key
  of a keyed item, the state the constructor binds, then content and options.
- Breaking: `Color::rgba` is renamed `Color::from_linear`, completing the
  `from_<space>` family beside `from_srgb` and `from_srgb8`; replace
  `Color::rgba(r, g, b, a)` with `Color::from_linear(r, g, b, a)`. Its rustdoc
  points normalized sRGB fractions to `from_srgb`, and `Color::channels` names
  it as the constructor of the linear channels it returns.
- Breaking: two-way bindings are named `bind_<property>`.
  `Collapsible::bound(signal)` is `Collapsible::bind_expanded(signal)`,
  `ToolWindowLayout::names(signal)` is `ToolWindowLayout::bind_show_names(signal)`
  and `PropertyGrid::shared_label_width(signal)` is
  `PropertyGrid::bind_label_width(signal)`. The naming rules state the
  convention beside the one-way `_signal` builders.
- Breaking: `Button::command`, `Button::command_icon` and `MenuItem::command`
  take `FnMut(&mut EventContext, CommandReceipt)`, the context first like
  every other component callback; replace `|receipt, cx| ..` with
  `|cx, receipt| ..`.
- Breaking: `Accordion::on_change` takes `FnMut(&mut EventContext,
  &ExpansionChange)`. The new `ExpansionChange` carries the open keys in
  display order (`expanded`) and the `ChangeOrigin` (`origin`), like
  `SelectionChange` and `ToolWindowChange`, and has a `new` constructor;
  replace `|cx, keys, origin| ..` with `|cx, change| ..` reading
  `change.expanded` and `change.origin`.
- Breaking: `SplitRule::Ratio` carries a `NormalizedValue` instead of an
  `f32`; read the share with `.get()`. `SplitSizing::rule` is no longer a
  `const fn`; `SplitSizing::ratio` still validates an `f32`.
- Breaking: `PowerExponent::MIN` and `PowerExponent::MAX` have type `Self`
  instead of `f32`, so they are valid exponents; replace
  `PowerExponent::new(PowerExponent::MAX)?` with `PowerExponent::MAX` and
  read the number with `PowerExponent::MAX.get()`.
- `Runtime::set_wake_delivery` takes `impl Fn() -> bool + Send + Sync +
  'static` instead of a `Box<dyn Fn() -> bool + Send + Sync>`. Callers may drop
  `Box::new`; a boxed closure still compiles.
- Added: `SizeLimits::exact(size)`, a `const`, infallible constructor for one
  fixed outer size, the limits `SizeLimits::new(size,
  Constraints::bounded(size))` returns.
- Added: limit constants. `PresentationRate::MIN` and `MAX` (1 and 1000 Hz,
  type `Self`), named by `PresentationRate::new`'s `# Errors` and by
  `RuntimeError::InvalidPresentationRate`, whose message is built from them;
  `ViewState::MAX_NAME_BYTES` (64), named by `ViewState::new` and
  `WorkspaceError::InvalidName`; `VectorRasterizer::MAX_FLATTENED_POINTS` and
  `VectorRasterizer::MAX_STROKE_EDGES` (1,048,576 each), the budgets its
  rustdoc described, which the private rasterizer constant backs.
- The naming rules in `docs/contracts.md` place limits: a constant bounding the
  value itself has type `Self`, one bounding a measure has the measure's
  integer type; a validation limit shared by several types lives on the owner
  of the shared format, otherwise on the error type whose one variant reports
  it; a rendering cap lives on the error type of the operation it keeps from
  failing. `NumberInputError::MAX_PRECISION`, `TextError::MAX_FONT_PIXELS` and
  `TextLimit::MAX_RASTER_SIDE` say why they live where they do.
- `WorkspaceError::InvalidName`'s rustdoc also names the empty map key that
  `WorkspaceValue::map` reports with it.

## 0.3.56 (2026-10-22)

Documentation: the facade's spelling in examples (API audit fix plan, part
5d). No public item, signature or behavior changes; the `api/` listings are
unchanged.

- **Item examples import `pert_ui::…`:** every rustdoc example in
  `pert-ui-runtime` and `pert-ui-core` that named its own crate now imports
  `pert_ui::…` through a hidden alias line (`# use pert_ui_runtime as
  pert_ui;`, or `pert_ui_core` in the core crate), so the item pages the
  facade inlines show what an application writes, and the examples still
  build as the defining crate's doctests. The runtime's own pages show the
  same spelling; the facade root page, the runtime README and the core README
  say so and name the defining crate's spelling for code that depends on it
  directly. The crate READMEs' own examples keep their crate's name.
- **`CpuRenderer` examples:** the `pert-ui-cpu` examples the facade's
  `cpu::CpuRenderer` page inlines import `pert_ui::cpu::CpuRenderer` and
  `pert_ui::…` through a hidden module that mirrors the facade's paths; the
  `pert-ui-cpu` README says so and names the direct spelling.

## 0.3.55 (2026-10-22)

Documentation: guide chapters and recipes of the API audit (API audit fix
plan, part 5c), describing the API as it is. No public item, signature or
behavior changes; the `api/` listings are unchanged.

- **Getting started:** the chapter leads with the registry dependency
  `pert-ui = { version = "0.3" }`, gives a checkout as the alternative before
  the first release or for unreleased changes, names the minimum Rust version
  and says that `cargo run -p pert-ui --example …` needs a checkout. The
  appearance guide's image formats and the diagnostics guide's serialization
  give the same requirement, `version = "0.3"`, with their features instead
  of a placeholder. New tests in the facade's foundation tests keep every
  dependency line of the guide and the Rust version in step with the
  package, and check that `status.md` names the same Rust version.
- **Editable list recipe:** the layout guide has "An editable list of
  records": application records in a `Signal<Vec<_>>` mapped to a
  `ListSource`, a `ListFilter` with a `CollectionFilter` field, a
  `VirtualList` with a `ListSelection`, `on_activate` and `reorderable`, a
  per-row checkbox binding and a retained title row whose draft survives
  scrolling and changes to other records. "Children from values" points to
  it.
- **Per-row bindings:** `ListItems::build`, `TableItems::cell` and
  `Signal::sibling` describe the per-row binding made with `sibling`, its cost
  (a new binding for every mounted row each time the collection describes its
  rows, on a source replacement and on a scroll or resize that changes the
  mounted rows, which ends a text draft) and the retained row component as
  the alternative.
- **Hidden selection:** `CollectionFilter`'s "Selection and focus" and the
  recipe show how to tell that the selected row is hidden, from
  `ListFilter::items`, `ListItems::index_of` and `ListSelection::active`
  through `ReadSignal::combine`.
- **Background work:** `EventContext::defer` and `Runtime::spawn` link each
  other, and the state guide's "Worker results and live data" has a complete
  button that defers a spawn and shows its result.
- **Smaller additions:** `Each` has an example of a lookup through its default
  `items` segment; `VirtualList::new` and the guide's "Large collections" say
  that a list binds a `ListSource` wrapping the `ListItems` value; the
  diagnostics guide shows `CaptureTarget::window(&ui.primary_window())` and
  captures the whole window in its CPU example; the state guide says signal
  values need `PartialEq` to be written and `Clone` to be read.

## 0.3.54 (2026-10-22)

Documentation: item rustdoc of the API audit (API audit fix plan, part 5b),
describing the API as it is. No public item, signature or behavior changes;
the `api/` listings are unchanged. Some parameter names change, which the
listings omit and callers do not see.

- **Parameter names:** `ListController::new` and `ZoomController::new` name
  their state owner `ui`; `Dialog::new` and `AlertDialog::new` name their
  controller `controller`; `AxisMapping::visible`, `ticks` and `zoomed_at`
  take `extent_pixels` and `anchor_pixel`; `TextField` and `TextArea`
  `min_length`/`max_length` take `graphemes` and name the unit first.
- **`# Errors` sections:**
  - `Runtime` and `Parameter` define a *runtime phase error* (`Closed` or a
    `WriteDuring*` variant), which the binding, host, session and runtime
    sections name.
  - Complete phase variants on `ListSelection::{set_active, select, clear}`,
    `TreeController::{set_expanded, toggle, set_source, filter, reveal}`,
    `TableController::{set_source, set_sort, filter, set_active_column,
    reveal}`, `ListController::reveal`, `ZoomController::{set, set_changed,
    fit}`, `FieldHandle::{set, set_changed, set_modified, clear, remove}` and
    `UiSettings::{set_motion, set_debug_overlay}`; the reachable ones on
    `Runtime::{mailbox, set_viewport, inspection_service, set_visible}`.
  - Named variants on `EditSession::{update, commit, cancel}` (with
    `EditError::Pending`), `ParameterHost::{write_base, replace_spec,
    set_effective, remove}`, `AnyParameterHost::remove`,
    `RemoteHost::{on_submit, clear_submission_sink, respond, resynchronize}`,
    `Parameter::begin_edit`, `ParameterState::{begin, replace_spec,
    write_base}`, `Runtime::{parameter, remote_parameter, cancel_input,
    set_presentation_rate, service_inspection, mount, mount_tree, children,
    invalidate, unmount, focus, event, paint}`, `FloatingWindow::{position,
    initial_size, fixed_size}`, `EventContext::{set_timer, start_drag}`,
    `Permission::get`, `FileBrowserController::selected_paths`,
    `Color::rgba`, `MenuBar::new`, `ViewState::{new, with}`,
    `Workspace::{new, decode}`, `WorkspacePath::new`, `WorkspaceValue::text`,
    `Renderer::recreate`, the row-height constructors of `FileBrowser`,
    `Table`, `Tree` and `VirtualList` (and `VirtualList::estimated`), and
    with their `RuntimeError::Geometry` wrapper on `App::{size, min_size,
    max_size}` and `NativeWindowOptions::size`.
  - Found during the part, with the same category wording: named variants on
    `ContextMenuOpener::open`, `ToolWindowController::{expand, set_names,
    is_expanded}` (`collapse` and `toggle` refer to `expand`) and
    `RemoteHost::retry_submission`.
  - `Runtime::focus` documents the failures it forwards after the transfer
    (`Component::focus_lost`, `Component::focus_within_changed`,
    `Component::reveal` and the settling flush) and their timing;
    `Runtime::event` and `Runtime::reveal_and_focus` follow it.
  - One-line forwarding on `ChoiceId`'s `TryFrom<&str>` and `TryFrom<String>`
    and on `TryFrom` for `Magnification`, `UiZoom` and `ViewScale`.
- **Errors across layers:** `Runtime::paint` and the diagnostics guide say a
  helper that paints returns `Result<_, DrawError>`, or `AppError` with
  `native`; `ApplicationError::new` and `RuntimeError::Application` point to
  `ApplicationError::from` for a boxed `dyn Error + Send + Sync`.
- **Escape and cancellation:** `Runtime::key_input` lists the order in which
  an Escape press is taken: a drag session, delayed help, the focused editors
  while a surface is open, the topmost surface, and only then the focused
  component, whose capture, timer, drag and edit sessions end before it
  receives Escape as an ordinary key. Escape's ordinary delivery never calls
  `Component::cancel_input`; an earlier consumer skips that cleanup and
  cleans up only through its own lifecycle (a cancelled drag releases its
  source's capture; a closed surface hides its contents, which calls
  `cancel_input`), as `EventContext::set_timer`,
  `Component::{cancel_input, focus_lost}` and the controls and custom-widget
  guides now say. `Component::cancel_input` runs outside the evaluation
  phases, may restore the component's own bindings, must not run application
  actions, and reports a failed restore through a `Notifier`.
  `ColorPicker`'s cancellation comment matches the hide and removal paths.
- **Owners and data:** `DraftEditor`'s owner contract names
  `Component::retain_initial_children` for a redescribed owner and says that
  beginning a session runs the owner's `cancel_input`. `Component`,
  `ListItems`, `TreeItems`, `TableItems`, `ContentRows` and `Clipboard` state
  their `'static` requirement, explained at `Component::content_rows`.
  `RichText` and `TextSpan` are UI-thread values because a span can name a
  `FontId`; a sendable font key is recorded in `plans/follow-ups.md`.
- **Filters:** `FilterMatcher`, `ListFilter::filter`, `TableController::filter`
  and `TreeController::filter` say a non-blank query is trimmed before the
  matcher sees it and point to `ListFilter::items`; `MultiSelect::filter` has
  a doctest reusing the `FilterMatcher` presets and says its query is not
  trimmed.
- **Inspection:** `Semantics::value`, `SemanticValue` and `Label` say static
  text is the name and a value is an editable or stateful control's content;
  `SemanticValue` and the custom-widget guide describe a multi-component value
  as `Text` plus one `InspectValue::Number` property per component.
- **Prelude pages:** both preludes list the crate-root names code most often
  adds; the custom-component guarantee, here and on the crate page, is
  narrowed to a component that lays out, draws and handles input; the facade
  prelude names `AppShutdownError` and says its native members come with
  `native`, which `desktop` and `desktop-cpu` enable.
- **Smaller corrections:** `Runtime::text_command` performs "a text command
  (clipboard or selection)"; `Constraints` states logical pixels;
  `Workspace::MAX_DOCUMENT_BYTES`, `VectorRasterizer::MAX_COVERAGE_WORK` and
  `Renderer::READBACK_TIMEOUT` state their values.

## 0.3.53 (2026-10-22)

Documentation: the rules of the API audit's approved policy decisions
(API audit fix plan, part 5a), written as the code already follows them.
No public item, signature or behavior changes; the `api/` listings are
unchanged.

- **`docs/contracts.md`:**
  - "Public naming rules": `get()` reads single-value types and a validated
    string borrows its text with `as_str()`; compound values have component
    accessors and a whole-value accessor named for its content; only limits
    the library chooses are `MIN_*`/`MAX_*` constants, intrinsic bounds are
    documented on the constructor; clamping follows AGENTS.md's data
    invariants; a no-argument builder moves one fixed option from its
    default, while options decided from data and data descriptions take a
    `bool`; element builders append, whole-list and one-slot builders
    replace; the three meanings of `with_` by receiver; `*State` by
    ownership, with `*Snapshot`, `*Inspection`, `*Report` and descriptive
    nouns for other passive data; a cross-thread reply such as `Pending<T>`
    is not a receipt; `*Outcome` is the non-error result of an interaction
    or operation, and a result may be named for what it delivers and carry
    a continuation.
  - "Choice sources": collections whose rows are content and keyed
    containers are outside the `new`/`bound`/`for_parameter` vocabulary.
  - "Permissions and availability": a capability the component provides
    is restricted by the adjective taking a `Permission`, one a callback or
    flag creates is restricted by `*_permission`; title-bar parts and
    `App::resizable` are fixed `bool` part choices.
  - "Component notifications": `on_select` reports each settled tab
    change, `on_selection_change` a coalesced bound selection.
  - "Error policy": application and OS text displays verbatim; one-line
    `TryFrom`/`FromStr` forwarding and type-level `# Serialization`
    sections for serde; a kind-only conversion may return its input as the
    error; the callback-boundary exception names where an application meets
    a `WindowError`, a `PresentationStatus::Failed` report, since no runtime
    method returns one.
- **AGENTS.md:** the clamp rule allows documented control or layout limits
  and documented presentation mappings total over their input, and
  requires rejecting an invalid explicit text offset, identity or stored
  preference value.
- **Rustdoc:**
  - `ProgressStatus::of`, `TableColumn::width` and
    `TableController::set_column_width` name the kind of clamp they apply;
  - `FloatingWindow::closable` and `minimizable` are fixed part choices,
    not Permissions;
  - `SemanticItem::disabled` and `Semantics::disabled`: a later
    `disabled(false)` does not restore withdrawn actions;
  - `Semantics::actions` appends; `Toast::action` replaces;
  - `VirtualList::on_selection_change` defines its timing without
    reference to `Tabs::on_select`;
  - `# Serialization` sections on `PowerExponent` and `FrameImage`;
  - `RuntimeError`, `ApplicationError` and the manual correct the
    callback-boundary rationale for `WindowError`, and
    `WorkspaceValue::number` follows the clamp rule.

## 0.3.52 (2026-10-21)

Toolchain:

- **Rust 1.99.1:** the pinned toolchain and the declared minimum Rust
  version (`rust-version`) move from 1.99.0 to 1.99.1, and CI's minimum
  version job follows. The point release fixes a missing `wcslen` symbol on
  UEFI and embedded targets, a miscompilation of FFI `bool` return values
  on x86_64 and the `libLLVM` rpath of rust-lld on macOS; no source change
  is needed.

## 0.3.51 (2026-10-20)

Documentation repairs (plan-audit-repairs slice 7):

- **`Runtime::command_key_input`:**
  - its rustdoc lists the complete routing order, including shown help
    taking Escape first and the Tab offer to `Component::tab_navigation`;
  - `# Errors` names `tab_navigation` and the dismissals;
  - the diagnostics guide's summary follows it.
- **`DraftEditor`:** an owner can claim Tab through `tab_navigation`.
- **`Painter::stroke_path` and `stroke_polyline`:** they link to the inward
  `stroke_shape`.
- **Editor sessions:** `TextField::session`, the number fields' `session`
  and the contracts state that disabling an editor reports nothing and
  keeps the edit open, and that a hidden native window delivers its
  `FocusLeft` once shown again.
- **The manual:**
  - `on_binding_refusal`, command buttons, menu row `on_change`, power
    mapping, `background_role_signal`, `KeyChord::named`;
  - `?` on runtime errors in command actions, `CaptureTarget::window`,
    `Runtime::find`, `View::fixed` among the bounding containers;
  - the sorted-row caveat with `TableController::reveal`;
  - the `live_mixer` example.
- **The guide:**
  - power mapping, `background_role_signal`, `on_binding_refusal` and a
    tree source's re-flattening;
  - in the custom widgets chapter, an `App::on_error` that reaches the
    application through the initializer's slot, and a section on exporting
    a widget as a picture.
- **The `diagram_editor` example:**
  - it installs that error handler;
  - its export uses `CaptureTarget::window`.

## 0.3.50 (2026-10-20)

Missing contract tests of table-editing slices 4 to 6 (plan-audit-repairs
slice 6), with no behavior change:

- Traversal scopes:
  - inside a detached view and a tool window in every mode;
  - dialogs of every modality opened inside a scope;
  - a scoped popover reached in its deferred place and closed when left;
  - tooltips kept out of traversal;
  - the menu bar and F10 with a scope active;
  - Tab repeats offered like presses.
- Editor sessions:
  - a consumer-written list host whose rows open editors, moving the edit
    with Tab and keeping a refused one open;
  - normalization-equivalent drafts reporting `Unchanged`;
  - refused Tab;
  - composition protecting Enter, Tab and Escape, and a Unicode start
    replacing a composition;
  - permission revocation and disabling;
  - window focus loss versus hiding;
  - a click in an open `Select`;
  - `DecimalInput`, and parameter-backed numbers ignoring sessions, also
    with changed and invalid drafts;
  - Tab during a selection drag.
- Table cell editing:
  - an editor that cannot take focus;
  - read-only revocation keeping focus;
  - a disabled editor closed by Escape on the body;
  - unmounting with an edit open;
  - the keyboard scenarios on both key paths.

## 0.3.49 (2026-10-20)

Missing contract tests of the diagram plan and table-editing slice 1
(plan-audit-repairs slice 5), with no behavior change:

- `ColorPicker`:
  - alpha End;
  - hex and channel drafts dropped by hiding or removal, which write and
    report nothing;
  - a handler replaced while the popover is open.
- `Knob`: an enabled knob whose edit is refused by a delivery fault does not
  arm a double-click.
- `ClickCounter`:
  - a later builder replaces an earlier one;
  - a threshold changed during a series applies from the next press;
  - the count saturates.
- The text editors' clear button and scrollbar end a click series.
- Every built-in editor opens with its value selected over a draft set
  while it mounts.

## 0.3.48 (2026-10-20)

Missing contract tests of the app-author and mixer plans (plan-audit-repairs
slice 4):

- `Signal::update` documents that its change cannot fail and what a
  panicking change leaves: no write of its own and no notification, while
  writes the change made itself stay made.
- New tests:
  - an application interval surviving focus loss and cancelled input and
    stopping at shutdown;
  - binding refusals: a second value refused for the same reason, a
    replaced callback, and a failed report that is not replayed;
  - a panicking `Signal::update`;
  - an application error's own cause kept in the chain;
  - a full command queue as `Button::command`'s activation error;
  - `CaptureTarget::window` and `region` serialized as their variants;
  - pointer choices in nested and context menus, and a direct menu-bar
    title chosen by keyboard;
  - a stretched row under unbounded height, and a short vertical slider
    whose readout stays inside it;
  - a secondary window with zero height, an in-view window keeping its
    minimum size, and minimized window content not validated until
    restored.

## 0.3.47 (2026-10-20)

Example sessions in CI (plan-audit-repairs slice 3):

- CI runs the headless sessions of the `live_mixer`, `dynamic_documents`,
  `diagram_editor`, `log_explorer` and `inventory` examples after the
  workspace tests, with the features that build unifies and a step timeout.
  Previously CI only compiled them. AGENTS.md lists the commands in the
  workspace gate.
- The `live_mixer` session bounds every wait on its engine: the hold
  confirmation, the resume, and the wait until the engine's queue is closed
  and its thread has returned, so dropping the mixer then joins at once.
  The `dynamic_documents` worker bounds its wait for a proposal, and the
  `log_explorer` session joins its inspection thread only once it has
  returned.

## 0.3.46 (2026-10-20)

Display text beyond the text limits (plan-audit-repairs slice 2):

- Ellipsis truncation (`TextOverflow::Ellipsis` and `MiddleEllipsis`,
  `MeasureContext::fitted_text`) cuts a first line longer than
  `TextEditError::MAX_TEXT_BYTES` before fitting it, so a long log line in
  an ellipsis table cell shows its start (or start and end) with "…"
  instead of failing the flush. The cut keeps the line's own grapheme
  cluster boundaries, shows the ellipsis and stays within the limit.
  Previously the whole line was segmented and copied, then refused.
- `Label` refuses text it still cannot prepare, as `SelectableText` does:
  - wrapped or rich text over the byte limit;
  - a layout beyond a raster limit;
  - rich content beyond `RichText::MAX_SEGMENTS`.

  The flush no longer fails. The label keeps its last prepared text and
  size, or is empty before any success, reports `text_error` in
  inspection with invalid semantics, and recovers when the text or size
  fits. `Tooltip::new`, `AlertDialog` titles and messages and `Group`
  headings inherit this.
- Help labels (an ellipsis label's help and `Tooltip::new`) show nothing
  while refused instead of earlier text. An ellipsis label offers no help
  for text over the byte limit.
- Truncation trims whitespace beside the ellipsis by the line's cluster
  boundaries. The remembered cuts are keyed by the first line, so a long
  text after a line break is no longer copied into the key.

## 0.3.45 (2026-10-20)

Failure sites for rejected children (plan-audit-repairs slice 1):

- When the children a component requested through
  `UpdateContext::request_children` are rejected during a flush (an empty
  or duplicated key, a changed component type under a kept key, an error
  from `Component::check_description`, or children given to a surviving
  component that manages its own), `Runtime::last_failure` now names that
  component with the new `FailurePhase::Reconcile`. `Dynamic`, `Show` and
  `Each` request their children this way. Previously the flush failed
  without a site. The new `FailureSite::description` holds the keys from the
  requested children down to the rejected description, and `Display` adds
  them, as in `reconciliation of root/list at items/row-3`; the default `App`
  error report prints it.
- `FailureSite` and `FailurePhase` stay `#[non_exhaustive]`; the new field
  takes part in equality and `Debug`. Direct calls to
  `Runtime::reconcile_children` and `mount_tree` still return their error to
  the caller and record no site.
- The `last_failure` rustdoc names every operation that starts without a
  site, including `flush_windows` and `service_intervals`.

## 0.3.44 (2026-10-20)

Review fixes for table-editing plan slice 7 (the `inventory` example):

- Row commands act on, and are enabled by, the selected rows that the
  category and Active filters and the table's search leave shown; a hidden
  selected row is no longer deleted, copied or duplicated.
- The store bounds quantities and prices and sums totals in `i128`, parses
  pasted Active flags strictly, refuses a value of the wrong kind for a
  field, and checks a group of edits in order before applying any of it.
- The headless session also checks choosing a category, saving a price,
  Shift+Tab, redo, a refused edit left by focus, the Add button, the
  Duplicate shortcut, copied and pasted values, refused pastes, the
  filters' effect on row commands and the store's refusals.

## 0.3.43 (2026-10-20)

An editable-table example (table-editing plan slice 7):

- The `inventory` example: a table edited in place with one editor per
  column built from `CellEdit`, a checkbox column whose state a retained
  cell component makes lazily, the table as one Tab stop, an
  application-owned store with checked edits, undo and redo, row copy and
  paste, and a toolbar of icon command buttons with shortcut help. Its
  headless session checks editing, undo, copy and paste.
- The gallery's Collections table edits its Note column in place.

## 0.3.42 (2026-10-20)

Review fixes for table-editing plan slice 6:

- Unmounting a table drops its cell editor callback, so a callback that
  holds the controller no longer keeps it alive, and a later table on the
  same controller edits only with its own editor.
- `Table::cell_tab_stops` takes effect when the table mounts; building or
  applying a later description no longer changes the mounted table's F2 and
  Escape route.
- Beginning an edit on the cell whose edit is open focuses its editor again
  even when the source no longer allows editing that cell.
- Cell content keyed like the editor's host sits under a table-owned
  wrapper, so such a key no longer fails the edit.
- `CellEdit` documents that signals made through its owner are ordinary
  runtime signals.

## 0.3.41 (2026-10-20)

Cell editing in tables (table-editing plan slice 6):

- `TableColumn::editable`, `TableItems::editable` and `Table::cell_editor`
  with `CellEdit`: the table edits cells in place. Enter begins with the value
  selected, F2 and a double-click with the caret at the end, typed text with
  that text; the table hosts the editor in the cell through an
  `EditorSession`, keeps the row mounted for the whole edit, and ends or moves
  the edit from the editor's reports (Enter and Escape return to the body,
  Tab and Shift+Tab move to the next or previous editable cell, a refused
  edit stays open). An edit closes when its row leaves the rows, when another
  begins, and when the table is hidden or unmounted.
- `Table::cell_tab_stops(false)` makes the body the table's one Tab stop; F2
  then enters a cell's resting content and Escape returns.
- `VirtualList::keep_mounted` keeps one item mounted outside the range, and
  `VirtualList::on_text_input` receives text typed on the focused list.
- Behavior change: Enter on an editable cell of a table with a cell editor
  edits it instead of running `on_activate`.

## 0.3.40 (2026-10-20)

Review fixes for table-editing plan slice 5:

- A session editor keeps Enter during a selection drag without saving; it
  previously saved without reporting. A number field's Escape during a drag
  ends the drag only instead of discarding the draft.
- Reattaching a focused editor to a new edit, with nothing else changed,
  applies the new edit's start in the same flush.
- `EditorStart::Typed` replaces the value with the text even when it is empty,
  and refuses a line break in a single-line editor with its text error instead
  of turning it into a space.
- A `Select` list's choice completes the edit open when the list opened, so a
  list left open across a new edit cannot end it.
- `Select::for_parameter` ignores `.session`, as the number fields do.
- A number field that refused its bound value since mount reports
  `Unchanged` instead of `Refused`.
- The text editors' draft-mode rustdoc names sessions.

## 0.3.39 (2026-10-20)

Editor sessions (table-editing plan slice 5):

- `EditorSession`, `EditorAttachment`, `EditorStart`, `EditorCompletion`,
  `EditorTrigger` and `EditorOutcome`: a host that runs one editor at a time
  learns how each edit ended (`Saved`, `Unchanged`, `Refused`, `Cancelled`)
  and by what (Enter, Tab, Escape, a choice, focus leaving), reported before
  the editor's own fallible callbacks and delivered through the host's
  notifier. Reports of an older edit are dropped.
- `TextField::session`, `IntegerInput::session`, `DecimalInput::session` and
  `Select::session`: with a session the editor keeps Enter, Tab and Escape
  (ahead of capture bindings, surfaces and ancestors), is always in draft
  mode, applies the edit's start (value selected, caret at the end, typed
  text), and a number field's steps change the draft instead of the binding.
- Behavior fix: shown tooltip help takes an unmodified Escape before any
  editor on both key paths, as the contracts state; previously an editor
  answered first on the native path and while a surface was open. The
  gallery's interaction check dismisses help before its search Escape step.

## 0.3.38 (2026-10-20)

Review fixes for table-editing plan slice 4:

- `Component::tab_navigation` documents its failure cleanup: committed
  effects stay, and a queued focus request follows the entry point's
  cleanup, which on the native path cancels it.
- Tests: a failing offer cleans up its recipient and keeps its committed
  write on both key paths; a held Tab registration wins over the offer;
  pointer presses and focus restoration ignore a scope; a dialog opened
  inside a scope takes its initial focus.

## 0.3.37 (2026-10-20)

Keyboard traversal scope and a Tab hook (table-editing plan slice 4):

- `Component::descendant_tab_stops`: a component returning false keeps its
  presentation subtree out of Tab traversal and `focus_next`, up to the next
  surface or native window root, which starts its own scope; pointer
  presses, explicit focus and focus restoration are unaffected.
- `Component::tab_navigation`: an unmodified Tab or Shift+Tab that reaches
  default traversal is offered to the focused component and its bubbling
  ancestors, innermost first, before focus moves; a claim stops traversal.
  On the native path held keys, command presses, the editing phase, capture
  bindings and held input still come first, and the release is dispatched
  as before.
- `VirtualList::item_tab_stops(false)` makes a list with interactive rows
  one stop; `Tree` forwards the scope. `Table` does not use it yet.

## 0.3.36 (2026-10-20)

Inspection ids (table-editing plan slice 3) and slice 2 review fixes:

- `Runtime::inspection_id` returns a mounted component's
  `ComponentInspection::id`, so a test finds that component's snapshot entry
  without matching keys; the diagnostics guide names it.
- `Tooltip::command` documents that unregistering the command leaves its
  last help, and the binding-derivation errors it can return.
- The gallery's Stop test reads the toolbar status bar's own message.

## 0.3.35 (2026-10-20)

Editing icon roles (table-editing plan slice 3):

- `Icon::Copy`, `Icon::Cut`, `Icon::Paste` and `Icon::Duplicate`, drawn from
  the bundled Material Icons Outlined font; a unit test checks that every
  role's default character has a glyph in that font.

## 0.3.34 (2026-10-20)

Command buttons with icons and shortcut help (table-editing plan slice 2):

- `Button::command_icon` builds the icon-only form of `Button::command`,
  with the command's enabled binding, receipts and its name as accessible
  name, replacing the hand-wired `Button::icon` and `invoke_command` recipe.
- `Tooltip::command` wraps such an anchor in help naming the command and its
  first shortcut, for example `Undo (Ctrl+Z)`, following
  `Runtime::command_shortcuts` when the command is rebound.
- The gallery's toolbar Stop button is a command (F7) enabled while playing.

## 0.3.33 (2026-10-20)

Review fix for table-editing plan slice 1:

- `IntegerInput` and `DecimalInput` keep Escape during a selection drag over
  unchanged text and end the drag, instead of letting it bubble to an owner
  or close a surface; the Escape test covers the drag on both key paths.

## 0.3.32 (2026-10-20)

Number inputs and Escape (table-editing plan slice 1):

- Behavior: `IntegerInput` and `DecimalInput` leave Escape to their owner
  when there is nothing to restore, as `TextField` does, so a dialog or cell
  editor around a clean number field closes on it; a changed or invalid
  draft is still restored and the key consumed, on both key paths.
- Tests: every built-in editor opens with its value selected whether focus
  or `SelectAll` is requested first, also over a draft binding set while the
  editor mounts. The probe's "select-all on open is lost" observation did
  not reproduce and is withdrawn.

## 0.3.31 (2026-10-19)

Review fixes for log-explorer plan slices 4 and 5:

- `log_explorer`: Follow turned on while a file loads starts the tail only
  after it, and tailed lines take identities from one allocator shared by
  every worker, so no two lines share an identity; lines a full mailbox
  refused or a stopped worker left queued are counted as lost and reported;
  follow reveals the last row left by every filter, the filter field
  included, and the shown count comes from the table's filter counts;
  timestamps are validated as dates and times of day; the session's
  inspection request waits with a timeout. The session checks each.
- `TableItems::text` and `TableController::filter` say that a row's text
  joins its cells with a space.
- Ellipsis evidence: the fit tests evict through the bounded layout cache
  and the fit memory instead of clearing them, and cover icon and gap
  entries; CPU captures of 56 ellipsis cases match 0.3.28 pixel for pixel.

## 0.3.30 (2026-10-19)

A large-data example (log-explorer plan slice 5):

- The `log_explorer` package example: a log explorer over a generated log
  of hundreds of thousands of lines with bounded tables and a tree, line
  identities independent of storage with a reverse index, batched loading
  on a worker with typed failures and skipped malformed lines, two tables
  with their own sorts, a filter field and level and source filters,
  appended lines inserted into each order, a live tail stopped by
  generation that reports dropped lines, follow that reveals the last
  shown row and observes its receipt, a detachable Errors view, Copy lines
  bound per table view and selectable detail text. Its headless session
  (`--no-default-features`) reports mounted counts and flush times and
  checks that they stay bounded; `--window` opens it with `native`.
- The layout guide's large-data section and the manual link it.

## 0.3.29 (2026-10-19)

Ellipsis text preparation (log-explorer plan slice 4):

- An ellipsis or middle-ellipsis label no longer shapes its whole text on
  every layout pass. Whether the whole text fits is read from a cached
  unwrapped layout under the same key when there is one; a text that fits
  is shaped once and rasterized from that shaping; a truncated text uses
  its remembered cut before measuring the whole string again. Fit
  decisions and output are unchanged. A wheel step over the
  `log_explorer` example's 50 000-line table went from 4.78 to 4.13 ms
  (release, Fedora 44).

## 0.3.28 (2026-10-19)

Review fixes for `SelectableText` (log-explorer plan slice 3):

- Cancelled input (`Runtime::cancel_input`, focus loss, hiding) ends a
  selection drag and keeps the selection; a later move no longer extends it.
- On the native path every key is read in the text phase by its layout
  identity, as the editors read theirs: the command modifier with the key
  that types A or C selects all or copies on any keyboard layout, and a
  physical key that types another letter does nothing.
- A redescription with another source ends a drag even when its text is
  the same; the same source keeps it.
- Tests for these, multiline Up/Down, word and document movement,
  composition, native Escape during a drag, a width change, selected glyph
  colors and copying during a refusal.

## 0.3.27 (2026-10-19)

Selectable display text (log-explorer plan slice 3):

- `SelectableText`: read-only text a reader can select and copy, such as a
  log message or an error's details. It measures and wraps like a multiline
  `Label`, takes one Tab stop (none while empty), selects by drag, double
  and triple click, Shift+press and the editors' movement keys with Shift,
  and copies with Ctrl/Cmd+C, its context menu (Copy and Select All) or
  `TextCommand::Copy`. Escape ends a drag, then collapses the selection,
  then bubbles, answered in the text phase on both key paths. A replaced
  text keeps a selection that still fits, snapped to graphemes; text that
  cannot be prepared is refused with a `text_error` property and the last
  shown text stays.
- `TextCommand::SelectAll`, answered by the text editors and
  `SelectableText`, and `StringRole::TextSelectAll` ("Select All").
- The gallery's Text page, the manual and the controls guide show it.

## 0.3.26 (2026-10-19)

Review fixes for log-explorer plan slices 1 and 2:

- A shrinking column computes in f64 and from final lengths: a grown child
  whose natural height is hundreds of millions of pixels (a 20-million-row
  list) now gets exactly the space left, and the column reports its limit,
  or the sum of its final lengths when it overflows at its floor, instead of
  a size that disagreed with its children's placement.
- `ValidationCode::UnboundedCollection` follows each arrangement: a
  collection's allocated and estimated extents are refreshed with its
  committed geometry, so a resize that lets it scroll itself clears the
  issue (and the reverse reports it) without another update.
- `VirtualList` and the layout guide said a `Row` never bounds a list; a
  bounded row bounds a vertical list's height and only never a horizontal
  list's width.
- Tests: the huge-extent, fractional, propagated-limit, gap-floor,
  stretch/space-between, grown-row, outer-only, horizontal, scaled,
  estimated-settling and arrangement-change cases; runtime clipboard
  refusal in every phase, from inside the service and from a failing
  service; and a serialized report carrying the new code.

## 0.3.25 (2026-10-19)

Collections in unbounded layout (log-explorer plan slice 1):

- Behavior change: a grown child of a column whose height is bounded now
  shrinks when the children's natural heights do not fit, by weight and
  down to its minimum, so a toolbar above a grown table or list fills the
  rest and the collection mounts only the rows it shows (it mounted every
  row before). The column overflows only past the ungrown children's
  natural heights plus the grown children's minimums. Rows are unchanged.
  `Element::with_grow`, the contracts, the manual and the layout guide say
  so; `foundation/row_stretch.rs` now expects a grown row to shrink to the
  height left.
- `ValidationCode::UnboundedCollection`: in debug builds, a virtualized
  collection of at least 100 rows that was given its whole content extent
  while less than half of it is visible (in a default `Tabs` tab, a default
  `RegionLayout` region or a `ScrollArea`) is reported with its owner and
  a pointer to the bounded-extent documentation. The inspection snapshot
  schema is 14.

## 0.3.24 (2026-10-19)

Clipboard text from the runtime (log-explorer plan slice 2):

- `Runtime::clipboard_text` and `Runtime::set_clipboard_text` read and write
  the installed clipboard service from application code and command actions,
  with the errors of the `EventContext` methods and without involving a
  component or moving focus.

Collection sizing documentation (log-explorer plan slice 0):

- `VirtualList`, `Table` and `Tree`, the manual and the layout guide state
  that a collection mounts the rows inside its allocated height and which
  containers give it a bounded one; the guide's new large-data section names
  the costs of source replacement, filters, `index_of`, tree children and
  table columns.

## 0.3.23 (2026-10-19)

Review fixes for the `diagram_editor` example of 0.3.22:

- The inspector is described under the selected node's identity and each
  save names the node it edits, so a draft can no longer be saved to
  another node when the selection changes, for example by duplicating.
- Close admission clears focus first, which saves an open inspector draft
  or in-place rename, so it asks before discarding them.
- Only the latest open applies, and only to an unchanged document; a save
  that finishes after another document was opened records nothing for it,
  and saves run one at a time.
- A rename ends when its document is replaced or the canvas is hidden,
  refuses a label longer than the file format stores, and its editor has an
  accessible name.
- Labels escape backslashes and line breaks in one pass, so they round-trip;
  the file's identity counter is bounded and allocation cannot wrap.
- A directory listing that finds the mailbox full is retried instead of
  dropped.
- The inspector checks a label's length in characters, as the file format
  does, instead of graphemes; the document's incarnation is a signal, so a
  rename also ends when the replacement equals the old document; a second
  close attempt after a cancelled one takes the earlier dialog outcome
  instead of failing.
- The headless session also covers the outline selecting on the canvas, a
  draft across a selection change, close with a draft and during a rename,
  an overtaken open, rename refusal, hiding and document replacement, label
  length and escaping, and runtime cleanup errors. The
  custom widgets chapter describes the identity-keyed inspector and checking
  background results against current state.

## 0.3.22 (2026-10-19)

A custom canvas example and a chapter for widget authors (diagram-probe plan
slice 4):

- The `diagram_editor` example: a canvas container with one child per node,
  pan and zoom through `child_transform`, stacked children in document
  order, selection, dragging that converges on the repeated pointer move,
  rubber-band selection, connecting nodes, a pan owned by its button, Space
  held for panning through `register_held_input` while the canvas has focus,
  double clicks counted by `ClickCounter`, renaming in place with a retained
  `DraftEditor`, and typed semantics. Around it: an application-owned
  document and undo history, one `ListSelection` shared by the canvas and an
  outline, an inspector bound to a view model synchronized in event phases,
  `ColorPicker::on_change` recording picked fills, commands in a menu bar, a
  context menu, a toolbar and a command palette, files opened through a
  `FileBrowser` dialog and saved on background threads, `defer_detached` for
  work a closing dialog starts, a saved layout, close admission and, with the
  `cpu` feature, PNG export in the light theme. Without arguments it runs a
  deterministic headless session; `--window` opens it with the `native`
  feature.
- The guide's new custom widgets chapter: who may write and when (with a
  `Notifier` example), pointer gestures that converge, clicks, held keys,
  described and focused children, overlap, inspection, an inspector over an
  application document (with an example) and building an application. The
  getting-started chapter and the manual point to it and to the example.

## 0.3.21 (2026-10-19)

Click recognition for custom widgets (diagram-probe plan slice 3):

- `ClickCounter` counts a component's presses as single, double, triple and
  longer clicks: a press of the same button within 500 ms and four logical
  pixels (per axis, in window coordinates) of the previous press continues
  the series. `press_event` feeds pointer events on the runtime's clock,
  `press` takes an explicit time and position, and the interval and
  distance can be replaced. The component resets it itself, on cancellation
  and after acting on a double click.
- `Knob`, the file browser's entry rows, `InlineText` and the text editors
  use it. The knob's double click, a file row's activation and
  `InlineText`'s editing now also require the second press within four
  logical pixels of the first, as the text editors already did; four quick
  presses on a knob or a row are two double clicks.

## 0.3.20 (2026-10-19)

A user-write callback for the color picker (diagram-probe plan slice 2):

- `ColorPicker::on_change` reports each committed user write that changed
  the bound color, with the color written, after the binding and the text
  fields hold it: a drag's release, a key press or repeat, a saved hex or
  channel field. Application writes, cancelled or revoked gestures, refused
  text and edits that leave the color unchanged report nothing; an error
  propagates from the input that committed, keeps the write and is not
  replayed. The handler follows same-key redescription, also through the
  retained composition of `ColorPicker::into_popover`.
- Fix: an edit whose new picker state still describes the bound color, such
  as a hue step on a grey or moving the hue between 0° and 360°, no longer
  rewrites the binding with a value rounded through HSV, and a saved hex or
  channel field writes the color it parsed exactly.
- The gallery's color picker reports its committed edits.
- Text editors keep their save model: the identity normalizer documented in
  0.3.18, `.normalize(Ok)`, is the way to save finished edits without other
  validation.

## 0.3.19 (2026-10-19)

The pointer refresh after layout made visible (diagram-probe plan slice 1):

- `PointerEvent::is_refresh` tells the move the runtime repeats at the last
  position after a layout or stacking change from host motion. Routing is
  unchanged: the capture owner, or else the component under the pointer,
  receives it, and a drag session receives none.
- `PointerKind::Move`, `Component::pointer_input` (with a runnable drag that
  converges), `EventContext::invalidate`, the input contract, the layout
  guide and the manual state that a handler must invalidate only when what
  it derives from the event changed, since an unconditional layout invalidation
  makes every layout produce the next move.
- In debug builds, an `UpdateLimit` validation report names a component
  whose handling of the refresh left new work scheduled more than once in
  the failing flush, as evidence of such a loop.

## 0.3.18 (2026-10-19)

Documentation repairs found by a diagram-editor probe (diagram-probe plan
slice 0):

- The manual no longer says that `on_save` reports whether Enter, focus loss
  or a step button saved a value; the callback receives the value only.
  `TextField::on_save`, `TextArea::on_save` and the manual name the identity
  normalizer, `.normalize(Ok)`, for a field that saves finished edits
  without adjusting them (a `TextField` on Enter or focus loss, a
  `TextArea` on focus loss).
- `Painter::stroke_shape` states that its stroke lies inside the rectangle,
  unlike the centered path strokes, and how to draw the same outline both
  ways.
- `EventContext::now` states that only `flush` and `service_intervals`
  advance the runtime's time, so input between them shares one instant,
  with a runnable double-click example for headless hosts.
- The `UnexpectedOverlap` and `EscapesParent` development messages, the
  `ValidationCode::UnexpectedOverlap` documentation and the layout guide
  name the ways to permit an intentional overlap: `OverlapPolicy::Allow` in
  `ComponentDetails::validation`, or a stacking parent.

## 0.3.17 (2026-10-19)

Recheck fixes for the `live_mixer` example:

- A commit enters the undo history once, when it is committed, whether or
  not the engine took it; a retried outcome no longer adds a stale entry.
- Retry reports the sink's own error, such as a stopped engine, and the
  headless session covers an undo while the queue is full and a stopped
  engine.
- The status message has a line of its own, cut to the window's width, so a
  long message no longer overflows the toolbar.

## 0.3.16 (2026-10-19)

Review fixes for the `live_mixer` example of 0.3.15:

- Sinks send the engine the base each edit leaves, so a cancelled audition
  no longer leaves the engine at the previewed value, and a retried outcome
  cannot overwrite a newer write.
- A full or stopped engine is shown in the toolbar, and Retry delivers
  faulted outcomes through `retry_outcome` and undo or reset writes that did
  not reach the engine. The mute toggles are shown.
- The headless session also covers a cancelled audition, undo during a
  drag, a full queue with its recovery and a partial layout restore.

## 0.3.15 (2026-10-19)

A packaged live-data example (mixer-probe plan slice 6):

- The `live_mixer` example: a producer thread behind a bounded command
  queue, `series_bridge` levels in `LevelMeter`s, parameter sinks that send
  the engine the base each edit leaves without blocking and fault on a full
  or closed queue, a Retry path through `retry_outcome`, an undo history
  refreshed through a mailbox, undo and reset during a drag, and a saved
  workspace whose decode failures and partial restores are shown. Its
  deterministic headless session covers each of these, including a full
  queue; `--window` opens it natively.

## 0.3.14 (2026-10-19)

Mapping and presentation (mixer-probe plan slice 5):

- `ParameterMapping::Power(PowerExponent)` maps a position to
  `minimum + range * position^exponent` over any finite domain, including
  one that spans zero. `PowerExponent` is finite and within 0.1 to 10
  (`ParameterError::InvalidExponent` otherwise); with `serde` it is a number
  validated on reading. Linear and logarithmic positions are unchanged.
- `Container::background_role_signal` fills with a bound theme role and
  repaints without layout when it changes, replacing and replaced by
  `background` and `background_role`.
- `Icon::Pause`, from the bundled icon font.
- `InlineToggle` and the toolbar guide tell authors to give an inline toggle
  an icon or glyph in a toolbar, since one without shows its name's first
  letter.

## 0.3.13 (2026-10-19)

Errors, lookup and command conveniences (mixer-probe plan slice 4):

- Breaking: `Runtime::find` now resolves addressable segments, the path
  inspection reports and `Selector::path` matches; it was `find_address`,
  which is removed. The full internal path lookup is `Runtime::find_path`
  (the old `find`). Replacement: `find_address(..)` becomes `find(..)`,
  `find(..)` becomes `find_path(..)`; an inspection path is passed without
  its root segment.
- `?` in an application callback converts a `WorkspaceError`, `WindowError`
  or `CaptureError` into `RuntimeError::Application` holding the whole error,
  except that one holding a runtime failure becomes that failure. This is a
  documented exception to the error policy; `DrawError` gets no conversion.
- `CaptureTarget::window` and `CaptureTarget::region` build window targets
  from a `NativeWindowId`, such as `Runtime::primary_window()`, keeping the
  portable number in the request.
- `Button::command` mirrors `MenuItem::command` for buttons, and
  `KeyChord::named` is a chord of one named key without modifiers.

## 0.3.12 (2026-10-19)

Reacting to user writes (mixer-probe plan slice 3):

- `MenuItem::on_change` reports each user write of a toggle or radio row with
  the row's new checked state, under the user-write callback contract:
  application writes, choosing the selected radio row again and refused
  writes report nothing. Choosing an already selected radio row no longer
  writes its value again.
- The state guide shows how a parameter sink refreshes bindings through a
  capacity-one `Mailbox`, and the surfaces guide how a menu applies a theme
  density with `selected_signal`.
- `ParameterHost::write_base` documents that it delivers the edit it ends
  before returning and calls no sink otherwise; the writer forwards its own
  writes. No base-change notification or general signal observer is added.

## 0.3.11 (2026-10-19)

Review fixes for the geometry validation of 0.3.9:

- `Runtime::validation_report` is `pending` while any window has pending
  updates or layout, not only the selected one, since a validation run
  then leaves that window's geometry unchecked.
- A geometry issue withheld while its window is not measured is not logged
  again when it reappears unchanged.

## 0.3.10 (2026-10-19)

Rows that fill their height (mixer-probe plan slice 2):

- Behavior change: a `Row` with `Align::Stretch` fills the height it is
  allocated instead of its tallest child's, so a row grown in a column
  stretches a vertical `Slider` or `LevelMeter` over the whole strip. A
  row's measured height is unchanged, children keep their maximum and
  minimum heights, and `Start`, `Center` (the default) and `End` still place
  against the tallest child. A root row with `Stretch` now fills the window.
- `Slider::orientation` documents how the track and readout share a short
  allocation, and the layout guide has a section on growing and stretching.

## 0.3.9 (2026-10-19)

Geometry validation without a viewport (mixer-probe plan slice 1):

- Development validation checks geometry only for a native window whose
  latest layout ran with a viewport that has area. Content flushed before
  the first viewport, as an `App` initializer can, and a window laid out
  while its viewport is empty, no longer report `EmptyHitArea` and the other
  geometry codes for every component; a component that is empty after a
  layout with area is still reported. Until the next layout after a
  viewport change the report keeps the last validation and is `pending`.
- `Viewport` no longer claims that no geometry is produced for an empty
  extent: layout runs against it and inspection reports the empty boxes.
- The native runner was confirmed to supply the viewport before its first
  flush, so `App` startup is unchanged.

## 0.3.8 (2026-10-18)

Desktop without wgpu (plan slice 7):

- The `desktop-cpu` feature builds `App` presented by the CPU renderer through
  softbuffer, without compiling wgpu; use it with `default-features = false`.
  `desktop` is now `gpu`, which builds on the new `native` feature; `native`
  alone has no renderer and refuses to compile.
- `LimitError` and `ResourceKind` are defined in `pert-ui-runtime` and
  re-exported unchanged by the facade and `pert-ui-wgpu`, so `AppError` no
  longer depends on the GPU crate and the types exist in every configuration.
- CI and the verification record cover the new configuration; the record
  lists build time and program size for both desktop configurations.

## 0.3.7 (2026-10-18)

Text refusal reports (plan slice 6, first part):

- `TextField::on_binding_refusal` and `TextArea::on_binding_refusal` report in
  an event phase when a value the application wrote to the binding is refused
  (with its `TextEditError`, such as `TooLarge`) and when an acceptable value
  ends the refusal. Refused typing and normalizer rejections do not call it.

## 0.3.6 (2026-10-18)

Application resources, background work and diagnostics (plan slice 5):

- `Runtime::keep_alive` owns application-lifetime values, such as a mailbox
  made in the `App` initializer, until shutdown, which drops them last and in
  reverse order.
- `Runtime::spawn` runs work on a background thread and delivers its result to
  a UI-thread callback; `BackgroundTask::cancel` discards an undelivered
  result. At most `Runtime::MAX_BACKGROUND_TASKS` tasks wait at once. New
  errors: `RuntimeError::BackgroundFull`, `BackgroundSpawn` and
  `BackgroundPanicked`.
- `Runtime::start_interval` repeats application work, such as an autosave, on
  the UI thread, independently of focus and hidden windows; the native host
  services due timers even while a redraw is withheld. `IntervalTimer::cancel`
  stops it, `Runtime::interval_deadline` and `service_intervals` serve custom
  hosts, and `RuntimeError::IntervalsFull` bounds the count.
- `Runtime::last_failure` names the component (key path and callback) where the
  latest failing flush or paint arose, and the native application's default
  error report includes it.

## 0.3.5 (2026-10-18)

Flex layout in rows and columns (plan slice 4):

- `Element::with_grow(weight)` gives a child of a `Row` or `Column` a share of
  the spare main-axis space, in proportion to its weight and up to its own
  maximum; `Element::spacer(key)` is an empty element with weight 1.
  `MeasureContext::grow` exposes the weight to custom containers.
- `Row::justify` and `Column::justify` place the space no child takes
  (`Justify::Start`, `Center`, `End`, `SpaceBetween`), and `align` places
  children across the main axis (`Align::Start`, `Center`, `End`, `Stretch`).
  The defaults keep the previous layout: start-aligned columns and rows
  centered on their tallest child.

## 0.3.4 (2026-10-18)

Selection notifications (plan slice 3):

- `VirtualList::on_selection_change`, `Tree::on_selection_change` and
  `Table::on_selection_change` report each change of the collection's
  `ListSelection` as a `ListSelectionChange` (active item and selected keys,
  before and after) with its `ChangeOrigin`, after the flush settles: `User`
  for a click or a key on any collection sharing the selection, `Application`
  for a write through the selection or a controller, including a filter that
  moves the active item off a hidden row. Writes
  between two updates of a collection arrive as one change; the initial
  selection is not reported.
- `Dynamic`, `Show` and `Each` detect a new value by the binding's write
  counter instead of comparing values, so a value with shared interior
  mutability cannot hide a write, and they no longer require `PartialEq`.
  Their documentation states that only the source binding is tracked.

## 0.3.3 (2026-10-18)

Value-driven children (plan slice 2):

- `Dynamic::new(&value, |_, value| element)` describes one child from a
  binding, `Show::new(&flag, |_| element)` mounts a child only while a flag
  reads true and releases it while false, and `Each::new(&items, key, build)`
  describes one keyed child per list item inside a `Column` or the container
  `within` gives. Children that keep their key keep their state and focus; a
  new description of the component itself always describes its children
  again. The description callbacks receive a `DescribeContext`, a state owner
  for constructors such as `Select::new`.
- The gallery's tag editor and breadcrumb locations use `Dynamic` instead of
  hand-written components.

## 0.3.2 (2026-10-18)

Binding and element conventions (plan slice 1). Breaking changes, with their
replacements:

- Breaking: `Signal::set` returns `Result<(), RuntimeError>`, so a callback can
  end with the write instead of `.map(|_| ())`. `Signal::set_changed` returns
  whether the value changed, as `set` did. `FieldHandle::set` and
  `ZoomController::set` follow the same split, with `set_changed` for the flag.
- `Signal::update(|value| ..)` changes a copy of the current value and writes
  it, and `toggle` inverts a `Signal<bool>`; both check the write phase before
  running, and a failure leaves their own write unmade.
- A `Signal` dereferences to its `ReadSignal`, so `&signal` is accepted wherever
  `&ReadSignal` is (for example `UpdateContext::read` and `combine`), and
  `signal.map(..)` derives directly. Constructors and builders that take a
  binding by value accept `&signal` (and a `Signal` where only reading is
  needed) through `Into`; `Permission` and `TextSource` convert from every
  binding form. Replace `X::new(signal.clone())` with `X::new(&signal)` and
  `signal.read_only().map(..)` with `signal.map(..)`.
- `IntoElement::key` describes a component as a keyed element in place:
  `Label::new("Ready").key("status")` is `Element::new("status",
  Label::new("Ready"))`. The trait is in the prelude. A custom component with
  its own `key(&self)` method called through `&mut self` now needs
  `Self::key(self)` or another name, which the compiler reports.
- `VirtualList` and `Tree` key the row they build with the source's key instead
  of rejecting a row whose element carries another key, so `ListItems::build`
  and `TreeItems::build` may use any key.
- Breaking: `MenuItem::enabled_signal` is removed; `MenuItem::enabled` takes the
  same binding and combines restrictions in the same way.

## 0.3.1 (2026-10-18)

- The facade's integration tests are grouped by area into a few test binaries
  under `crates/pert-ui/tests/<area>/`, so each feature configuration links a
  few binaries instead of one per test file. The tests themselves are
  unchanged.
- CI builds the dev and test profiles without debuginfo or incremental caches,
  which kept filling the Linux runner disk and exceeded the Windows PDB limit.

## 0.3.0 — first prerelease (2026-10-15)

First public preview of pert-ui, a Rust desktop GUI library with retained
components, general controls, typed parameters, reactive bindings, inspection,
workspace persistence and a native gallery. The public facade supports GPU
rendering and optional CPU rendering and software presentation.

The API may change before 1.0. The libraries have not been published to a
registry.
