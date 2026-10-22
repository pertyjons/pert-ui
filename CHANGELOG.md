# Changelog

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
  geometry is in logical pixels unless its name says physical, a length given
  to the library and stored is a `Length`, a constructor whose argument must
  be positive may take `f32` and return `GeometryError`, and lengths read back
  for arithmetic are `f32` logical pixels. `Painter::rich_text`,
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
