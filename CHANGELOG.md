# Changelog

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
