# Changelog

Entries are condensed per milestone and name what a user of the library
needs: additions, behavior changes, breaking changes with their replacement,
and notable fixes. The detailed per-version entries up to 0.3.101 are in Git
history (this file at commit f5fde76).

## 0.3.104–0.3.111 (2026-11-12): Analytic shapes

A rounded or cut rectangle can be drawn as one command whose coverage the
renderer computes per pixel, with a size and a kind per corner and with fill
and border as one silhouette. Both renderers draw it, the wgpu renderer
from records in a storage buffer. The uniform interior of a larger rectangle is not
evaluated per pixel: it is filled directly, or skipped when nothing is
drawn there. Built-in rounded and cut
rectangles, arcs and triangles are drawn this way.

- **Breaking:** `Shape` is a struct with one `Corner` per position instead
  of an enum.

  | Old | New |
  |---|---|
  | `Shape::Rounded(r)` | `Shape::rounded(r)` |
  | `Shape::RoundedTop(r)` | `Shape::rounded_top(r)` |
  | `Shape::CutTopRight(c)` | `Shape::cut_top_right(c)` |
  | `Shape::Rectangle` | `Shape::RECTANGLE` |
  | `match` on `Shape` | `Shape::corner` |
- **Added:** `Painter::draw_shape` with `ShapeStyle`; `Corner`,
  `CornerKind`, `CornerPosition` and `Shape::with_corner`;
  `DrawCommand::DrawShape` with `ShapeGeometry`, `ArcGeometry`,
  `TriangleGeometry`, `Paint` and `Border`; the shared preparation a backend
  uses, `FrameMapping`, `PreparedShape`, `ShapeParameters` (with `Boundary`,
  `Inner`, `PreparedCorner`), `ShapeCoverage` and `ShapePrepareError`; the
  errors `CpuRenderError::Shape`, `RenderError::Shape`,
  `CaptureError::Shape` and `LimitError::ShapeRecords`;
  `PreparedShape::solid_run`, `solid_box` and `PreparedShape::PARITY_MARGIN`;
  `FrameMapping::frame_width` and `frame_height`.
- **Parity:** the CPU and wgpu renderers agree on a shape's edge pixels
  within one step per channel, measured on one adapter; the rendering
  contract states the tiers.
- **Changed:** `fill_shape`, `stroke_shape`, the focus ring, `stroke_arc`
  and `fill_triangle` record one `DrawShape` each; a fill without corners
  stays a `FillRect`. The shape coverage cache is gone: no shape is
  rasterized on the UI thread, and a knob turned through many values adds no
  mask. On screen, a rectangular shape's edges snap to physical pixels and
  its border is a whole number of physical pixels; arcs and triangles keep
  sub-pixel placement; corners, arcs and triangles are antialiased by
  distance; corners that do not fit are scaled as CSS does
  (a `rounded_top` radius may exceed half the smaller side), and shapes stay
  sharp during a zoom and beyond 8× magnification.
- **Changed:** a built-in frame whose fill is directly followed by an
  outline of the same shape is one `DrawShape`: the inactive control frame
  of buttons and of the Select and MultiSelect anchors, field frames,
  toasts, detached tab slots, and the focus ring of buttons, chips and
  links. No fill shows beside the border's curve. A frame or ring drawn
  over a caption or over children stays a separate stroke.
- **Added:** `pert_ui::component` also exports `ShapeStyle`, the style
  `Painter::draw_shape` takes, and `Corner`, `CornerKind` and
  `CornerPosition`, which build and read a `Shape`.
- **Changed:** shape drawing no longer returns
  `DrawError::CoverageTooLarge`. A shape beyond the evaluation range is
  refused by the renderer with `ShapePrepareError::OutOfRange`.
- **Removed:** the interim `Runtime::set_shape_recording`,
  `Runtime::shape_recording` and `ShapeRecording` of 0.3.105–0.3.107; every
  shape is recorded as `DrawShape`, so there is no mode to select.
- **Tests that inspect a display list:** a shape is a `DrawShape` with a
  fill, a border or both, not rectangles and corner masks.

## 0.3.103 (2026-11-06): Resize and scale-change cost

Resizing a window or changing its scale does far less UI work: components
that do not depend on the window extent keep their layout, hidden subtrees
are not walked, and text that is only measured is neither reshaped nor
rasterized.

- **Added:** `Painter::pixel_scale`, physical pixels per local logical pixel
  without a dependency on the window extent.
- **Changed:** a resize at an unchanged scale no longer invalidates every
  component. It reaches the components that read the window extent through
  `MeasureContext::viewport` (layout), `UpdateContext::viewport` (update and
  layout) or `Painter::viewport` (paint), and the surfaces placed against the
  window; other layout follows from changed constraints. A scale or zoom
  change still invalidates every component.
- **Breaking:** `Painter::viewport` is no longer `const`, since it records
  the read. A drawing that needs only the scale reads `Painter::pixel_scale`.
- **Changed:** updating a component no longer walks the visibility of its
  whole subtree, hidden pages included; the walk stops where visibility is
  unchanged.
- **Changed:** prepared text is cached for up to 4096 entries within the
  unchanged 16 MiB budget, evicts the least recently used entry and is looked
  up by hash, so a page with a thousand labels no longer reshapes them on
  every layout pass.
- **Changed:** preparing text places its glyphs without rasterizing them;
  a `TextLayout`'s coverage is rendered when it is first painted, so text
  that is only measured costs no rasterizing. Sizes and painted pixels are
  unchanged.

## 0.3.102 (2026-11-05): Documentation consolidation

Documentation only; no public item changed. The rustdoc guide becomes the
single application guide.

- **Changed:** the guide takes over what only `docs/manual.md` covered:
  table cell editing, folder browsing and `RequestTracker`, revealing,
  content search and navigation history, `ColorSwatch` and `ColorPicker`,
  `UiStrings`, `NumberLocale` and color conversions, application state from
  the initializer, `Runtime::keep_alive`, `Runtime::start_interval`,
  `Runtime::last_failure`, custom-host shutdown, `Runtime::commands_for` and
  `check_redescription`. `docs/manual.md` is a short entry page pointing to
  the guide's chapters and the example applications.
- **Changed:** the `pert-ui-runtime` crate page keeps what a custom host or
  graphics integration needs beyond the guide (reading the display list,
  vector paths, color images, retained composition and layout primitives)
  and points to the facade's guide for the rest.
- **Repository:** `docs/verification.md` keeps only current evidence and
  measurements, and this changelog is condensed per milestone; AGENTS.md
  and CONTRIBUTING.md ask for concise entries from now on.

## 0.3.95–0.3.101 (2026-11-04): Repairs of the TODO entries that need no Windows or macOS

Content search now reaches semantic items, more built-in controls report clickable item rectangles, and composite popups expose named parts for tests. Collections build keyless rows, fixed-extent lists report rows that do not fit their slot, and disabled dropdown anchors match the disabled Button frame. PNG captures are now compressed.

- **Added:** `VirtualList::report_items(role)` reports the list's rows as `Semantics::items`, with rectangles for rows placed in view so `Driver::click_item` clicks them; `ListItems::semantic_item(index)` (default `None`) describes each row's item.
- **Added:** `UpdateContext::children_adopted` and `ArrangeContext::children_adopted` tell whether the runtime applied a component's latest child request, so an external collection can retry a rejected request as `VirtualList` does.
- **Added:** `RowItem`, a keyless row wrapping a complete element, with `From` for every component and for `Element`; `RowItem::into_element(key)` keys it.
- **Added:** `Runtime::content_search` matches semantic item names (a toast's message, a menu command, an unselected tab's title) through `ContentField::Item { index }`; the hit targets the component, and `jump_to_content` reveals and focuses the component, not the item. Items of protected components, of components that name their children and of components behind a modal surface are not searched.
- **Added:** `QuickPick::searchable(bool)` (default true) decides whether content search finds the picker's own content; a picker that shows search results turns it off.
- **Added:** exposed parts `cancel` and `accept` on `AlertDialog`'s generated content at `dialog/confirmation`; `control` and `options` on `MultiSelect`, beside `search`.
- **Added:** item rectangles (`SemanticItem::local_bounds`) for `Accordion` headers, the options of `RadioGroup`, `SegmentedControl` and `InlineChoice`, tool-window stripe buttons and the `MultiSelect` popup's options. `MultiSelect`'s popup list is a `ListBox` with checked, selected and disabled state per option.
- **Added:** `ArrangeContext::place_fixed_slot(index, slot, clip, axis)` places a child and declares that it must fit the slot along `axis`; development validation reports overflow as `ValidationCode::ExceedsFixedHeight` or the new `ValidationCode::ExceedsFixedWidth`. Every fixed-extent `VirtualList`, and so `Tree`, places its items this way.
- **Added:** `DraftAlign::Center`: a draft narrower than its editing area stays centered, so a centered label keeps its place when it opens for editing.
- **Changed:** disabled `Select` and `MultiSelect` anchors draw the neutral disabled frame a disabled framed `Button` uses (`Surface` fill, `Muted` content, inward `Outline` at `BorderWidth`).
- **Changed:** `Button` and the `Select`/`MultiSelect` anchors report the colors they last painted (`background`, `border`, `foreground`) in their inspection box model; absent before the first paint, and no border at zero `BorderWidth`.
- **Changed:** `FrameImage::encode_png` compresses (per-row PNG filters, zlib level 6 through `miniz_oxide`); output is deterministic for a given version, and a dependency update may change the bytes but not the decoded pixels.
- **Changed:** a tool window described with `ToolWindowMode::Window` and `expanded(true)` opens its own native window from the start; a refused detach returns it to docking.
- **Changed:** `FileBrowser` keeps entry and folder names on one line with a middle ellipsis, and failure messages and placeholders end in an ellipsis; semantic names keep the full text.
- **Changed:** `UnexpectedOverlap` compares peers only within one window and parent in O(n log n); more than eight overlapping pairs under one parent are reported as one summary issue.
- **Changed:** an open `MultiSelect` option is found by content search as the trigger's item, the popup list's item and the row's name, as distinct reports. A disabled `QuickPick` or `CommandPalette` result offers no semantic action, while a click or Enter still reaches the choose callback.
- **Breaking:** `ListItems::build` and `TreeItems::build` return `Option<RowItem>` instead of `Option<Element>` → `Some(Label::new(name).into())` without a placeholder `.key(..)`, or `Some(Element::new("any", content).with_label("…").into())` for an element row.
- **Breaking:** `QuickPick`'s result `ListBox` moves from its internal `content` component to the `options` part inside `surface` → `Runtime::find(&picker, &["surface"])`, then its `options` part. With no results there is no `ListBox` and no `options` part, and the list's identity changes with every query edit and session.
- **Breaking:** tests that reached the `MultiSelect` trigger or popup through the internal `anchor` and `list` keys → `Runtime::find_exposed_part(&chooser, "control")` and `"options"`.
- **Breaking:** `InspectionSnapshot::SCHEMA_VERSION` advances to 18 for `ValidationCode::ExceedsFixedWidth`; schema 17 snapshots are rejected as unsupported.
- **Fixed:** a child left mounted by a rejected child request no longer makes every later `VirtualList` layout fail with `InvalidChild`, and the list requests its rows again at its next update until they are admitted.
- **Fixed:** the `diagram_editor` example no longer draws a node's label under the rename editor.
- **Dependencies:** `miniz_oxide` 0.9.1 is a new dependency of `pert-ui-runtime`; `clipboard-win` is updated from the yanked 5.4.1 to 5.4.3.
- **Repository:** doctests of `pert-ui-runtime` and `pert-ui` deny unused imports.

## 0.3.89–0.3.94 (2026-10-28 to 2026-10-29): Usability round 3 repairs

Composite controls gain named, public exposed parts for tests and custom composites, and tab titles and menu or Select rows report real pointer geometry. Disabled framed buttons get a distinct neutral frame. Menus, MultiSelect and popups behave better when keyboard and pointer input mix, and text layout no longer counts invisible tab and empty-glyph ink.

- **Added:** `Element::expose_part`, `Runtime::find_exposed_part`, `testing::Driver::{exposed_part, exposed_parts}` and `testing::ExposedPart` (public `name` and `id`): exact scoped part names, independent of keys, labels and workspace paths. Queries do not flush, reveal, focus or schedule work. They replace traversal of private structural keys.
- **Added:** errors `RuntimeError::EmptyExposedPartName`, `RuntimeError::AmbiguousExposedPart { owner, name }` and `testing::TestError::ExposedPartNotFound { owner, key, name, available }`.
- **Added:** built-in parts: `Select`/`InlineSelect` expose `control` and `options`; `Menu` exposes `trigger` and `items`; `MenuBar` exposes `title/<menu-key>`; menu levels expose `submenu/<row-key>` with nested `items`; editors' menus sit under `clipboard`; `MultiSelect` and `QuickPick` search fields under `search`.
- **Added:** `Tabs` title items and the option-list rows of menus and Select report local pointer rectangles, so `Driver::click_item` clicks them with real pointer input. It does not scroll to an item or open a closed list.
- **Added:** `ApplicationError` is re-exported under `component`.
- **Changed:** a disabled framed `Button` draws a neutral `Surface` fill, `Muted` content and an inward `Outline` at `BorderWidth` instead of the enabled Button fill; frameless buttons keep content only. An empty or all-disabled `Menu` trigger inherits the frame.
- **Changed:** `Menu`, `ContextMenu`, `Select` and `InlineSelect` keep their initial or selected row when opened under a stationary pointer; only real pointer motion, presses or scrolling take the highlight. `MenuBar` switches an open title only on real pointer motion.
- **Changed:** `MultiSelect` keeps search-key highlighting under a stationary pointer, and a completed row click returns focus to search before the bound-value write is reported.
- **Changed:** automatic pointer-press focus stops at an interactive surface root that already holds focus, so a short popup inside a scrolling page is no longer dismissed before its option handles the press.
- **Changed:** literal tab-only text atoms and zero-area glyph rasters contribute no ink or bounds. `TextLayout::size` can shrink, and public caret, selection, hit and word coordinates can shift; whole-frame pixel identity is not promised.
- **Changed:** a wrapping `TextArea` pins its displayed caret inside the text box for painting, reveal and IME anchoring; raw `TextSelection` geometry is unpinned. Repeated hanging whitespace no longer grows horizontal scrolling indefinitely.
- **Changed:** `RevealRefusal`'s `Debug` prints immutable `WidgetId` serials without live mount state, so a retained refusal's text stays the same after unmount; nested `Debug` of `RevealOutcome`, `RevealStatus`, `RevealReceipt` and `TestError` changes with it.
- **Breaking:** `ComponentInspection::exposed_part` is added and `InspectionSnapshot::SCHEMA_VERSION` advances to 17; snapshots and query results of other revisions are rejected.

## 0.3.76–0.3.88 (2026-10-28): Usability round 2 fixes

This range answers the second developer-usability round. Applications now get
typed state from `App::new`, derivations take several bindings in one
`ReadSignal::combine` call, and display controls take plain values as well as
bindings. A bounded row now shrinks its grown children. The test driver gains
row addresses, item rectangles, `clear_focus` and idle checks, and the
`testing` module now holds every type a test needs.

- **Added:** application state and forms
  - `HeadlessApp::state`, `state_mut` and `runtime_and_state_mut`. They give
    access to the state returned by `App::new`'s initializer (see Breaking).
  - `FormFields::discard_drafts`, which queues `TextCommand::DiscardDraft`
    for every visible field the form records and returns how many it queued.
    It is the way to end drafts from a Revert. The state chapter gains a
    "A settings form" recipe (dirty state, Apply, Revert, close policy).
  - `CombineInput` and `CombineInputs` (sealed, at the crate root), which
    accept tuples of 2 to 12 signals or read bindings in `ReadSignal::combine`.
  - `EachItem` (sealed). `Each::new`'s `build` can return a bare component,
    and `Each` then keys it by the item.
- **Added:** controls and layout
  - `ProgressBar::bound`, `Spinner::bound`, `Field::status_signal` and
    `Property::status_signal`, which follow a binding.
  - `ProgressStatus::fraction`.
  - `impl From<u16> for Spacing`, so `.padding(24)` and `.gap(8)` compile.
  - `ShownTaskReceipt`, the `#[must_use]` receipt of `defer_while_shown`.
  - `ArrangeContext::refresh_forwarded`, for a fixed-height component that
    forwards its child's measurement.
- **Added:** testing and inspection
  - `Driver::address_children` and `AddressChild` list the components one
    address segment below a parent, in child order. This covers `Each`,
    `VirtualList` and `Table` rows.
  - `SemanticItem::local_bounds` and `SemanticItem::bounds`, which report
    item rectangles in window coordinates (table header titles report
    them). `Driver::click_item` presses an item, with the errors
    `TestError::NoItem` and `TestError::ItemWithoutBounds`.
  - `Driver::clear_focus`, which clears focus so a test starts with nothing
    focused. A valid draft is saved, and held keys and modifiers are
    released.
  - `Driver::quiet_for` and `QuietReport` (`components_updated`,
    `next_deadline`, `needs_paint`, `baseline_painted`, `is_idle`) check
    that nothing is scheduled while idle.
  - `TestError::Draw` and `impl From<DrawError> for TestError`.
  - Development validation's `ValidationHints::fixed_height` and
    `ValidationCode::ExceedsFixedHeight` report table cell content taller
    than its row.
  - A `ToastHost`'s stack reports one `SemanticItem` per visible toast,
    with Activate/Dismiss actions and bounds, plus an `action_labels`
    property.
  - `pert_ui::testing` and `pert_ui_runtime::testing` now hold the types a
    test receives: `FlushReport`, `RevealRefusal`, `Semantics` and its
    role, action and item types, the query and inspection records,
    `ValidationReport` and related types, `Capture`, `CaptureError`,
    `Pending`, `InspectionReply`, `WaitError`, `NativeWindowId`, `Rect`,
    `Magnification`, `DrawError`, and `CpuRenderError` with `cpu`.
    Existing paths keep working.
  - `ThemeMode` is in both preludes.
- **Changed:**
  - `TaskReceipt` (from `defer`, `spawn` and `Runtime::defer`) is no longer
    `#[must_use]`, so `cx.defer(...)?;` compiles without a warning.
  - Each component keeps its four most recently used measurements instead
    of one.
  - The application state, and the values the callbacks capture, drop after
    the runtime shuts down on every path.
  - After a failed layout, the next layout arranges again every component
    the failed one arranged.
  - `CaptureError::Unsupported`'s message names
    `Runtime::set_capture_support(true)` and `CpuRenderer::serve_captures`.
  - The packaged examples return their application object as the `App`
    state instead of calling `Runtime::keep_alive`. The diagram editor asks
    before closing while an inspector draft is kept.
- **Breaking:**
  - `App<S = ()>` and `HeadlessApp<S = ()>`. The initializer returns
    `Result<S, AppError>`, and `on_close`, `on_error` and `on_window_error`
    receive `&mut S` after the runtime: `.on_close(|ui, state, window| ..)`,
    or `|ui, _, window|` without state. An initializer that never returns
    `Ok` writes `App::<()>::new(..)`.
  - `a.combine(&b, |a, b| ..)` → `ReadSignal::combine((&a, &b), |(a, b)| ..)`.
    A chain of combines becomes one tuple call.
  - `ProgressBar::new(status.read_only())` → `ProgressBar::bound(..)`.
    `new` now takes a `ProgressStatus`.
  - `Spinner::new` takes a `bool`; `Spinner::bound(..)` follows a binding.
  - `Field::status(signal)` and `Property::status(signal)` →
    `.status_signal(..)`. `status` now takes a `FieldStatus`. `status`,
    `status_signal` and `form` replace each other.
  - `InlineReadout::new` takes `impl Into<TextSource>`. A caller-defined
    conversion into `ReadSignal<String>` passes the signal instead.
  - `ProgressStatus::Failed` and `ProgressStatus::Cancelled` take an
    `Option<NormalizedValue>`. `Failed(None)` is the old full track, and
    `Failed(last.fraction())` keeps the fraction reached.
  - `Each::new`'s `build` returns any `EachItem`. A closure inferred only
    from `Element`, such as `|_, item| make(item).into()`, names `-> Element`.
  - `EventContext::defer_while_shown` returns `ShownTaskReceipt` instead of
    `TaskReceipt`. Code that stores the receipt names the new type.
  - Module `pert_ui::svg` → `pert_ui::svg_images` (for example
    `svg_images::SvgError`). The `svg` feature and `pert_ui::Svg` are
    unchanged.
  - `TestError::NotFound` gains `resolved`, the longest prefix of the path
    that resolves. Match it with `NotFound { path, .. }`.
  - Inspection schema 16 (`InspectionSnapshot::SCHEMA_VERSION`). Snapshots
    of other revisions fail to deserialize: capture them again.
  - `ValidationHints` has a new public field. Build it with
    `..ValidationHints::default()`.
  - Behavior: a grown child of a bounded row now gives up width by weight,
    down to its minimum, when the natural widths do not fit. A shrunk child
    is measured again at its final width. To keep the old width, drop the
    child's growth weight or set a minimum with `SizeLimits::at_least`.
- **Fixed:**
  - A component's theme and font dependencies from earlier measurements
    stay recorded, so a cached ancestor measurement no longer misses a
    metric or font change.
  - A changed write that ends a refused `IntegerInput` or `DecimalInput`
    draft no longer leaves its validation error behind.
  - An `InlineReadout` whose bound or formatted text is refused keeps its
    last valid text and unit.

## 0.3.69–0.3.75 (2026-10-26 to 2026-10-27): API audit completion and the layering pass

This range completed the API audit's editing capabilities. Callbacks can start background work, code can begin an edit, inline labels can live in selectable list rows, and checkboxes, toggles and inline labels gained a controlled form. Then the API was layered. The prelude now holds what a first application writes, and thematic modules hold the rest. Host-only `Runtime` methods moved to a sealed trait, deferral collapsed into a single call, and every built-in editor follows one draft policy, which contracts.md documents in its new "Draft policy" section.

- **Added:** `EventContext::spawn` starts background work from a callback. It queues the start for the next flush, only shutdown cancels it, and its `TaskReceipt` reports `Succeeded`, `Failed` or `Cancelled`.
- **Added:** `TextCommand::BeginEdit` starts editing as Enter/F2 would. `InlineText`, `InlineInteger`, `InlineDecimal`, `InlineSlider` and `InlineReadout` answer it at rest, and its `Handled` requests focus for the editing part. `TextCommand::DiscardDraft` works like Escape without writing anything: every built-in editor answers it, also while disabled or read-only, with `Unavailable` when there is no draft.
- **Added:** `InlineText::in_row(bool)` is a list-row mode. The resting label takes no focus, a press selects the row, and a double click starts editing.
- **Added:** `HiddenActive` (`Keep` by default, `Nearest`, `Clear`) and `CollectionFilter::hidden_active` decide what happens to the active item when a filter change hides it while focus is outside the collection.
- **Added:** controlled controls: `Checkbox::controlled`, `Toggle::controlled` and `InlineText::controlled`. Each shows a value the application owns, proposes every user change to its callback and writes nothing itself, so list rows and table cells can edit records without per-row signals. A redescription with the same value keeps an open draft or pending activation. Described text the editor refuses is rejected at adoption, and the control keeps its previous text.
- **Added:** `EventContext::defer_while_shown`, the old `defer` rule: queued work is cancelled when its component is unmounted or not effectively visible.
- **Added:** thematic modules in `pert-ui` and `pert-ui-runtime`: `parameter`, `commands`, `windowing`, `inspection` and `live`. Each holds its members together with the types their constructors, builders and payloads take. `component` and `testing` gained the names custom components and tests need, and `NumberInputError` joined the prelude.
- **Added:** the `host` module, with the sealed trait `HostIntegration` (implemented by `Runtime`) and re-exports of the names a custom host writes. `HostIntegration` is also in `testing`.
- **Added:** `on_modified` on `IntegerInput`, `DecimalInput`, `InlineInteger`, `InlineDecimal` and `InlineText`, so close policies and `FormFields` see the drafts these fields keep.
- **Added:** `Table::on_edit_end` with `CellEditEnd` and `CellEditEndReason`. It reports every cell edit's end, including edits the table ends itself.
- **Added:** `DraftEditorController::set_read_only`, which makes a session read-only or editable again without ending it.
- **Added:** `CellEditEnd`, `CellEditEndReason`, `EditorCompletion`, `EditorOutcome`, `EditorTrigger` and `FocusDirection` in the prelude.
- **Changed:** `on_change` of `Checkbox` and `Toggle`, and `InlineText::on_save`, also report a controlled control's proposal. `InlineText::normalize` may read application state but not write it, and it must be deterministic and idempotent.
- **Changed:** a focus request whose target an ancestor's pending update is about to show is now kept for the next pass of the same flush instead of being dropped.
- **Changed:** a session field reports `on_modified` before each completion it reports to its `EditorSession`, Escape included. `DraftEditorController::end` releases the pointer capture of a gesture it discards.
- **Changed:** a `Table` has a second, invisible structural child keyed `edit end reporter`, which `Runtime::children` and inspection list. The body stays the first child.
- **Breaking:** the prelude now holds only what an application's first program writes. Other names need a module glob, and every name stays at the crate root:
  - `use pert_ui::component::*;` for custom-component names (`Component`, `Painter`, `DraftEditor`, `ContentRows`, ...)
  - `use pert_ui::testing::*;` for input-driving names (`PointerEvent`, `Viewport`, `KeyAction`, ...)
  - `use pert_ui::commands::*;` for command names except `CommandScope`, `KeyChord`, `CommandKey`, `KeyCode` and `Modifiers`
  - `use pert_ui::parameter::*;` for parameter and authority types, `Slider`, `Knob` and `InlineSlider`
  - `use pert_ui::windowing::*;` for windows, regions, splits and tool windows
  - `use pert_ui::live::*;` for `LevelMeter`, `MeterAxis` and `Trace`
  - `use pert_ui::inspection::*;` (or `component`) for `ComponentDetails`, `Invalidation`, `KeyEvent` and `Rect`
  - Import by name from the root for theme, settings, fonts, strings and workspace names, for example `use pert_ui::{ThemeMode, Workspace};`.
- **Breaking:** the host-only `Runtime` methods (`flush_windows`, `needs_paint`, `capture_scene`, `set_clipboard`, ...) moved to `host::HostIntegration` with unchanged signatures. A custom host adds `use pert_ui::host::HostIntegration;`, while `testing::*` users and `App` users need nothing.
- **Breaking:** `Runtime::mount` → `ui.mount_tree(Element::new(key, component))`. The root is now laid out into the whole window.
- **Breaking:** `Runtime::find_path` → `Runtime::find` with the addressable path, or a walk with `Runtime::children` and `Runtime::key`.
- **Breaking:** `Painter::size` returns `Size` instead of `Option<Size>`, so `let size = painter.size();`.
- **Breaking:** an `Each`'s default container is structural and no longer adds `items` to item addresses, so `find(&each, &["items", key])` → `find(&each, &[key])`. The same applies to `Selector` paths, `CaptureTarget::Path`, workspace paths and search hit paths.
- **Breaking:** `Workspace::SCHEMA_VERSION` is 2, and revision-1 documents are rejected with `WorkspaceError::UnsupportedVersion`. Capture the workspace again.
- **Breaking:** `EventContext::defer_detached` → `EventContext::defer`. Hiding or removing the widget, or hiding or closing its window, no longer cancels `defer`. Only shutdown does. For the old behavior use `defer_while_shown`.
- **Breaking:** drafts in `IntegerInput`/`DecimalInput` now survive revoked `enabled`, input cancellation other than hiding and removal, and a held step button that loses editing or focus. The steps a press took are kept. An open text mode of `InlineInteger`/`InlineDecimal` becomes read-only when its editing permission is revoked instead of refusing saves. In each case, `TextCommand::DiscardDraft` (or Escape) discards the draft.
- **Breaking:** `SearchField` writes an edit it receives without focus at once. When focus leaves while editing is denied, it drops the pending debounced query. A disabled session field that loses focus before the next flush reports nothing instead of `Refused`. No replacement is needed for either.
- **Fixed:** a task queued during `Runtime::shutdown` is refused with `Closed` instead of keeping a `Pending` receipt.
- **Fixed:** a stepped draft was lost when focus left during a held step; a session field now reports `FocusLeft` with the outcome.
- **Fixed:** `ColorPicker`'s channel fields now keep their drafts, as its hex field does.
- **Repository:** contracts.md is now layered by audience, with a section for component and host authors. Mechanisms moved to the new `docs/internals.md`, and the guide gained an "Authoritative values" chapter (`guide::authoritative_values`) and "Choosing a way" tables.

## 0.3.53–0.3.68 (2026-10-22 to 2026-10-23): API audit fixes

The API audit's findings were applied across the library: consistent names, argument order and roles; error values that carry and name what they reject; `Debug` on every public type; a rule-based `#[must_use]`; a prelude and a new `component` module for imports; collections that follow redescription; and a headless `testing` module with an `App::headless` host. Close requests now save focused drafts before the close policy runs. The inspection schema advances to 15. Most of the range is breaking; each change below gives its replacement.

- **Added:** `pert_ui::testing` (from `pert_ui_runtime::testing`): `testing::Driver` drives a `Runtime` as a user would, one settled step at a time (`settle`, `advance`, `wait_until`, `find`, `semantics`, `click`, `replace_text`, `type_text`, `key`, `chord`, constant `FRAME`), with the `#[non_exhaustive]` `testing::TestError`; the module re-exports the capture and inspection query types, plus `CpuRenderer` with `cpu` and `HeadlessApp` with `native`. Guide chapter `pert_ui::guide::testing`.
- **Added:** `App::headless` builds the application without an event loop, window or GPU; the returned `HeadlessApp` offers `runtime`, `main_window`, `admit_close` (runs close admission and `App::on_close`, returning the decision) and `shutdown`.
- **Added:** `Runtime::can_focus`, a side-effect-free query of whether a component can take focus.
- **Added:** the module `pert_ui::component` (from `pert_ui_runtime::component`) for `use pert_ui::component::*` beside the prelude: `Component`, its contexts and every name an implementation writes (layout, input and commands, dragging, drawing, semantics, workspace state, receipts, native windows, editor sessions).
- **Added:** many names in the prelude by its membership rule: commands and held keys, `ListDrop*`, live widgets, parameter and binding types and their erased `Any*` forms, host sink and recovery types, the remote-authority host (`RemoteHost`, `RemoteCommit`, `AuthorityResponse`, …), drawing values, `FontManager`/`IconManager`, workspace values, `ApplicationError`, `FocusDirection`, `ExpansionChange`, `Expansion`; with `native`, `NativeWindowId` and `WindowError`.
- **Added:** `impl Debug` for every public type: state owners and handles, callback contexts, `Painter`, `Runtime`, `App`, `pert_ui_wgpu::Renderer`, `VectorRasterizer`, every component description and `Element` (descriptions show their type or key, never content or callbacks). The workspace now denies `missing_debug_implementations`.
- **Added:** standard traits: `Hash` on identities, keys and roles (`CommandId`, `KeyChord`, `Modifiers`, `ColorRole`, `TextStyle`, `RequestTicket<K>`, …); `Ord`/`Eq`/`Hash` on validated quantities (`Capacity`, `PowerExponent`, `Magnification`, `UiZoom`, `ViewScale`, `PresentationRate`); `Eq`/`Hash` on `RasterImage` and `MaskImage` by content; `Display for Color` (hex); `From` conversions into raw numbers (`From<NormalizedValue> for f32`, `From<Capacity> for usize`, `From<Length> for f32`, …); `AsRef<str>` for `ChoiceId`; `From<Vec<u8>>`/`From<&'static [u8]>` for `FontData`; `From<Tab>`, `From<Section>`, `From<Property>`, `From<FloatingWindow>` for `Element`.
- **Added:** `SizeLimits::exact`; limit constants `PresentationRate::MIN`/`MAX`, `ViewState::MAX_NAME_BYTES`, `VectorRasterizer::MAX_FLATTENED_POINTS`/`MAX_STROKE_EDGES`.
- **Added:** number-input messages `StringRole::NotANumber`, `NotAWholeNumber`, `NumberOutOfRange`, `NumberOffStep`, with `UiMessage::NumberOutOfRange { min, max }` and `UiMessage::NumberOffStep { step }`; `Semantics::warning` marks a description as a warning.
- **Added:** `ListingError::with_path`/`path` (`list_directory` names the path on every error); new `DrawError::CoverageTooLarge`, `MissingVector`, `MissingMask`, `MissingImage`; `CpuRenderError::AllocationFailed`; with `serde`, `LimitError` and `ResourceKind` serialize.
- **Added:** compiler diagnostics: `Component`, `ListItems`, `TreeItems`, `TableItems`, `ContentRows`, `Clipboard` and `StateOwner` report a missing implementation in pert-ui's terms, sealed traits say they are sealed and name their implementors, and `?` into `AppError` reports the missing `From` for `AppError`.
- **Changed:** `#[must_use]` follows one rule: `CommandId`, `HeldInputId` and `ToastReceipt` no longer carry it (dropping them changes nothing); `ParameterHost`, `AnyParameterHost`, `SeriesSource`, `Runtime::series_source` and the signal constructors (`Runtime::signal`, `Signal::sibling`, …) now do, so discarding them warns.
- **Changed:** `Runtime::set_wake_delivery` takes `impl Fn() -> bool + Send + Sync + 'static` (a boxed closure still compiles); error messages name the rejected key, identity, path or integer, and causes print once in a source chain.
- **Breaking (behavior):** number inputs (`IntegerInput`, `DecimalInput`, `InlineInteger`, `InlineDecimal`) report their own refusals of a draft (unparsable, out of range, off step, or an immediate step normalized out of bounds) through `on_rejection` as an `InputRejection` from `UiStrings`.
- **Breaking (behavior):** `VirtualList`, `Tree` and `Table` adopt a redescription's callbacks and permissions, including their removal; only the controller, source binding and structural options stay. A later description without `reorderable` or with a fixed denial cancels an active reorder, column move or resize. Describe callbacks in every description (`VirtualList::new(&items, 24.0)?.on_activate(open)` each time). `FileBrowser::folder_filter` likewise follows redescription (mount with `folder_filter(false)` to offer it later).
- **Breaking (behavior):** a naming parent that is invalid or carries a warning (such as a `Field` with an error or warning status) gives that description to children that describe themselves.
- **Breaking (behavior):** outside macOS, single-line editors leave Up/Down with Alt or Super to ancestors and commands (`TextArea` still moves by line); `TextField` documents its claimed keys per platform.
- **Breaking (behavior):** an empty plain `Label` measures as zero size; `CollectionFilter`'s field address is `…/filter-field/filter` (was `…/filter`): `ui.find(&root, &["tasks", "filter-field", "filter"])`.
- **Breaking:** a close request (window system or `Runtime::request_close`) releases focus in every window it would discard before `App::on_close`, so a valid changed draft is saved and a parameter-bound draft cancelled; on `KeepOpen` the editor is refocused when `Runtime::can_focus` admits it. Drop any `Runtime::clear_focus` workaround in `on_close`.
- **Breaking:** argument order: bound state comes first after `ui`, and a keyed item's key leads: `Checkbox::new(value, label)`, `Toggle::new(value, label)`, `TableController::new(ui, selection, columns, source)`, `TreeController::new(ui, selection, source)`, `FileBrowserController::new(ui, selection, roots, request)`, `MenuItem::toggle(key, value, label)`, `MenuItem::radio(key, selection, value, label)`.
- **Breaking:** callbacks take the context first: `Button::command`, `Button::command_icon`, `MenuItem::command` take `|cx, receipt|`; `Accordion::on_change` takes `|cx, change: &ExpansionChange|` (`change.expanded`, `change.origin`).
- **Breaking:** renames: `Color::rgba` → `Color::from_linear`; `Collapsible::bound` → `bind_expanded`, `ToolWindowLayout::names` → `bind_show_names`, `PropertyGrid::shared_label_width` → `bind_label_width`; `ThemeMetrics::get` → `metric`, `UiStrings::get` → `text`, `UiStrings::with` → `with_text`, `UiStrings::MAX_LENGTH` → `MAX_TEXT_BYTES`; `DragPayload::get` → `downcast_ref`; `EditorSession::is_open` → `is_active`; `DropPosition` → `ListDropPosition`; `TaskStatus::Completed` → `Succeeded`.
- **Breaking:** reactive reads with an owned form end in `_binding`: `UiSettings::{zoom,text_rendering,motion,strings,debug_overlay}_signal` → `*_binding`, `ThemeManager::{density,mode}_signal` → `*_binding`, `DeliveryMonitor::observe` → `snapshot_binding`.
- **Breaking:** widget-owned handles are `*Controller`: `ContextMenuOpener` → `ContextMenuController` (`ContextMenu::opener()` → `controller()`), `ToastQueue` → `ToastController`; application-owned `FormStatus` → `FormFields`. `PendingJump::status`/`status_binding` → `pending.receipt().status()`/`.status_binding()`.
- **Breaking:** byte offsets named for their unit: `TextSelection::{anchor, focus, range}` → `{anchor_byte, focus_byte, byte_range}`, `TextPreedit::cursor` → `cursor_byte_range`, `ResourceKind::scene_budget` → `scene_budget_bytes`.
- **Breaking:** boolean flags become enums: `Runtime::focus_next(FocusDirection::Forward | Backward)`; `EditorTrigger::Tab { direction }`; `Component::tab_navigation(cx, direction: FocusDirection, transition: KeyTransition)`; `EventContext::request_reveal(target)` plus `request_reveal_and_focus(target)`; `SelectionExtension::{Collapse, Extend}` for `TextSelection::moved`/`at_point`; `Expansion` for `ToolWindowChange::new`; `NativeCloseAction::Defer` carries `ReadSignal<Option<WindowCloseAction>>` (`Close`/`KeepOpen` instead of `true`/`false`).
- **Breaking:** typed geometry and values: `pert_ui_wgpu::Renderer::new`, `new_surface`, `resize` take a `PhysicalExtent`; `QuickPick::width`/`CommandPalette::width` take a `Length` (`.width(Length::new(640.0)?)?`) and their `DEFAULT_WIDTH` is a `Length`; `TextLayout::paint`/`paint_turned` take a `Point` origin; `SplitRule::Ratio` carries a `NormalizedValue`; `PowerExponent::MIN`/`MAX` are `Self` (read with `.get()`).
- **Breaking:** fallible signatures: `Workspace::set_settings` returns `Result<(), WorkspaceError>` (`TooLarge` past `Workspace::MAX_DOCUMENT_BYTES`; no longer `const`); `TextSelection::new` returns `Result<Self, TextEditError>`, rejecting offsets past the end or inside a grapheme with `InvalidSelection` (pass the text's length, not `usize::MAX`).
- **Breaking:** captures: `CaptureJob`'s fields are private (`id()`, `window()`, `include_overlay()`) and `Runtime::complete_capture` takes `&CaptureJob`: `ui.complete_capture(&job, frame)`. A completed job or one from another runtime is refused.
- **Breaking:** error variants carry the rejected value: `RuntimeError::{AmbiguousKey, DuplicateKey, ChildTypeChanged}(String)` and `ShortcutConflict(KeyChord)`; `LayoutError::{DuplicateKey, ReservedKey, UnknownKey}`, `FontError::{UnknownIcon, MissingGlyph}`, `ParameterError::{DuplicateOption, UnknownOption}`, `LiveError::CapacityExceeded`, `NumberInputError::InvalidPrecision` hold the value; `ChoiceError::DuplicateValue { first, second }`, `NumberLocaleError::SameSeparators { decimal, grouping }`, `TextEditError::InvalidComposition { start_byte, end_byte }`/`InvalidSelection { anchor_byte, focus_byte }`, `ImageError::Io { path, source }` (its `From<std::io::Error>` removed). `LayoutError`, `FontError`, `ParameterError`, `EditError`, `BindingError` and `EditOutcome<T>` are no longer `Copy`: clone them (`event.outcome.clone()`).
- **Breaking:** `DrawError::InvalidImage { width, height, bytes }` is only the rejection by `MaskImage::new`/`RasterImage::from_srgba8`; an oversized coverage mask is `DrawError::CoverageTooLarge`. The frame size limit is `LimitError::FrameSize` only (`CaptureSize` removed); `CaptureError::TooLarge` → `CaptureError::Limit(LimitError)`, `CpuRenderError::TooLarge` → `CpuRenderError::Limit(LimitError)`; `CaptureError`'s serde form changes accordingly.
- **Breaking:** the runtime-built inspection records (`InspectionSnapshot`, `WindowInspection`, `ComponentInspection`, `EventRecord`, `ValidationReport`, …) and the enums `EventKind` and `ValidationCode` are `#[non_exhaustive]`: read fields or decode with serde, and add a wildcard arm. `InspectionSnapshot::SCHEMA_VERSION` is 15 (serialized semantics require `warning`); schema 14 snapshots no longer decode.
- **Breaking:** `Divider` and `pert_ui::Svg` are no longer `Send`: build components on the UI thread and move source bytes or rasterized pixels across threads instead.
- **Fixed:** an accepted close of a detached window in component-error backoff no longer drops its pending recovery error; `App::run` returns it at exit.
- **Fixed:** focus transfers whose reveal fails queue the focus-change error instead of dropping it; library-published signals such as `Parameter::snapshot` advance `Signal::revision`, so a `Dynamic`, `Show` or `Each` over one describes again.
- **Fixed:** inspection snapshots keep `Semantics`/`SemanticItem` invariants for directly assigned fields; workspace restore refuses a table column width below `TableColumn::MIN_WIDTH` with `RestoreRejection::OutOfRange`; `check_redescription` no longer logs a false `UnexpectedOverlap`.
- **Fixed:** `RadioGroup`, `SegmentedControl` and `InlineChoice` keep their last valid presentation while a rebound option list is invalid; `RegionLayout` no longer delivers resize changes recorded before its callback was removed and reinstalled.
- **Repository:** the workspace denies `clippy::missing_panics_doc`, `clippy::return_self_not_must_use` and `clippy::fn_params_excessive_bools`; a new unpublished `pert-ui-diagnostics` member snapshots compiler messages with `trybuild`.

## 0.3.52 (2026-10-21): Rust 1.99.1

The pinned toolchain and the declared minimum Rust version (`rust-version`) move from 1.99.0 to 1.99.1; no source change is needed.

## 0.3.45–0.3.51 (2026-10-20): Repairs from the plan audit

Rejected children now report where a flush failed, display text beyond the text limits truncates or is refused instead of failing the flush, and the examples' headless sessions run with bounded waits. The rest of the range is documentation and contract tests with no behavior change.

- **Added:**
  - `FailurePhase::Reconcile` and `FailureSite::description`: when children requested through `UpdateContext::request_children` (as `Dynamic`, `Show` and `Each` do) are rejected during a flush, `Runtime::last_failure` names the component and the keys down to the rejected description, as in `reconciliation of root/list at items/row-3`; the default `App` error report prints it. Direct `Runtime::reconcile_children` and `mount_tree` calls still return their error without a site.
- **Changed:**
  - Ellipsis truncation (`TextOverflow::Ellipsis`, `MiddleEllipsis`, `MeasureContext::fitted_text`) cuts a first line longer than `TextEditError::MAX_TEXT_BYTES` at grapheme cluster boundaries before fitting it, so a long log line shows its start (or start and end) with "…" instead of failing the flush.
  - `Label` refuses text it cannot prepare (wrapped or rich text over the byte limit, a layout beyond a raster limit, rich content beyond `RichText::MAX_SEGMENTS`) as `SelectableText` does: it keeps its last prepared text, reports `text_error` in inspection and recovers when the text fits. `Tooltip::new`, `AlertDialog` titles and messages and `Group` headings inherit this; refused help labels show nothing.
  - `Signal::update` documents that a panicking change leaves no write of its own and no notification, while writes the change made itself stay made.
  - Documentation of `Runtime::command_key_input`'s full routing order, editor sessions on disable and hidden windows, and new manual and guide sections (power mapping, `background_role_signal`, `on_binding_refusal`, exporting a widget as a picture); the `diagram_editor` example exports with `CaptureTarget::window`.
- **Repository:** CI runs the headless sessions of the packaged examples, whose engine and worker waits are now bounded.

## 0.3.32–0.3.44 (2026-10-20): Table editing and the inventory example

Tables edit cells in place through a general editor-session contract that any host can use, with keyboard traversal scopes and a Tab hook for components. Command buttons gain an icon form with shortcut help, and the new `inventory` example shows an editable table with an application-owned store, undo, redo, copy and paste.

- **Added:**
  - Cell editing: `TableColumn::editable`, `TableItems::editable` and `Table::cell_editor` with `CellEdit`. Enter begins with the value selected, F2 and double-click with the caret at the end, typing with that text; Enter and Escape return to the body, Tab and Shift+Tab move between editable cells, a refused edit stays open, and the edit closes when its row leaves, another begins or the table is hidden or unmounted. `Table::cell_tab_stops(false)` makes the body the table's one Tab stop.
  - Editor sessions: `EditorSession`, `EditorAttachment`, `EditorStart`, `EditorCompletion`, `EditorTrigger` and `EditorOutcome` report how each edit ended (`Saved`, `Unchanged`, `Refused`, `Cancelled`) and by what, through the host's notifier. `TextField::session`, `IntegerInput::session`, `DecimalInput::session` and `Select::session` make an editor keep Enter, Tab and Escape, stay in draft mode and apply the edit's start; parameter-backed editors ignore sessions.
  - Traversal: `Component::descendant_tab_stops` keeps a subtree out of Tab traversal and `focus_next`; `Component::tab_navigation` offers an unmodified Tab or Shift+Tab to the focused component and its ancestors before focus moves. `VirtualList::item_tab_stops(false)` makes a list one stop (`Tree` forwards it).
  - `VirtualList::keep_mounted` and `VirtualList::on_text_input`.
  - `Button::command_icon`, the icon-only form of `Button::command`, and `Tooltip::command`, help naming the command and its first shortcut (`Undo (Ctrl+Z)`) that follows `Runtime::command_shortcuts`.
  - `Icon::Copy`, `Icon::Cut`, `Icon::Paste` and `Icon::Duplicate`.
  - `Runtime::inspection_id`, which returns a mounted component's `ComponentInspection::id`.
  - The `inventory` example: per-column cell editors, a lazily made checkbox column, checked edits, undo and redo, row copy and paste, filters and a toolbar of icon command buttons. The gallery's Collections table edits its Note column in place.
- **Changed:**
  - Enter on an editable cell of a table with a cell editor edits it instead of running `on_activate`.
  - `IntegerInput` and `DecimalInput` leave Escape to their owner when there is nothing to restore, as `TextField` does, so a dialog or cell editor around a clean number field closes on it; during a selection drag they keep Escape and end the drag.
  - `Button::command_icon` replaces the hand-wired `Button::icon` and `invoke_command` recipe.
- **Fixed:**
  - Shown tooltip help takes an unmodified Escape before any editor on both key paths; previously an editor answered first on the native path and while a surface was open.

## 0.3.24–0.3.31 (2026-10-19): Large data and the log explorer example

Collections now stay bounded inside ordinary columns, so a grown table or list mounts only the rows it shows. Read-only text became selectable and copyable, application code gained direct clipboard access, and ellipsis labels stopped reshaping their whole text on every layout pass. The `log_explorer` example demonstrates a log of hundreds of thousands of lines.

- **Added:**
  - `SelectableText`: read-only text that can be selected by drag, double or triple click and Shift with the editors' movement keys, and copied with Ctrl/Cmd+C, its context menu or `TextCommand::Copy`. It wraps like a multiline `Label`. Text it cannot prepare is refused with a `text_error` property, and the last text shown stays.
  - `TextCommand::SelectAll`, which the text editors and `SelectableText` answer, and `StringRole::TextSelectAll`.
  - `Runtime::clipboard_text` and `Runtime::set_clipboard_text`, for reading and writing the clipboard from application code and command actions without a component and without moving focus.
  - `ValidationCode::UnboundedCollection`: in debug builds, reports a virtualized collection of at least 100 rows that was given its whole content extent while less than half of it is visible. It is refreshed after each arrangement. The inspection snapshot schema is 14.
  - The `log_explorer` package example: bounded tables and a tree, batched loading on a worker, filters, a live tail with follow, a detachable view and selectable detail text. Its headless session checks that mounted counts and flush times stay bounded. The layout guide gained a large-data section.
- **Changed:**
  - A grown child of a column with a bounded height now shrinks, by weight and down to its minimum, when the natural heights do not fit, so a collection below a toolbar gets the space left over. The column reports its limit, or the sum of its children's final lengths when it overflows at its floor.
  - Ellipsis and middle-ellipsis labels reuse cached shaping and remembered cuts instead of shaping the whole text on every layout pass. Output is unchanged.
- **Fixed:** a shrinking column computes in f64, so a child with a natural height of hundreds of millions of pixels gets exactly the remaining space.

## 0.3.18–0.3.23 (2026-10-19): Custom canvases and the diagram editor example

This range added the building blocks a custom canvas needs: click counting, a user-write callback for the color picker and a visible pointer refresh after layout. It also added the `diagram_editor` example and a guide chapter for widget authors.

- **Added:**
  - `ClickCounter` counts single, double, triple and longer clicks. A press of the same button within 500 ms and four logical pixels of the previous one continues the series. It is fed through `press_event` or `press`, and the interval and distance can be replaced.
  - `ColorPicker::on_change` reports each committed user write that changed the bound color, also through `ColorPicker::into_popover`.
  - `PointerEvent::is_refresh` marks the move the runtime repeats after a layout or stacking change. In debug builds, an `UpdateLimit` validation report names a component whose handling of the refresh keeps scheduling work.
  - The `diagram_editor` example: a canvas with pan and zoom through `child_transform`, selection, dragging, connecting, held-Space panning, renaming in place with `DraftEditor`, an application-owned document and undo history, a shared `ListSelection`, background saves, close admission and, with `cpu`, PNG export.
  - The guide's custom widgets chapter covers write timing, converging pointer gestures, clicks, held keys, overlap, inspection, and an inspector over an application document.
- **Changed:**
  - `Knob`, file browser rows, `InlineText` and the text editors use `ClickCounter`. A double click on a knob, a file row or `InlineText` now requires the second press within four logical pixels of the first, and four quick presses count as two double clicks.
  - The documentation now states that `TextField`/`TextArea` `on_save` receives only the value, and that `.normalize(Ok)` saves finished edits without adjusting them. It also states that `Painter::stroke_shape` strokes inside the rectangle, that `EventContext::now` advances only on `flush` and `service_intervals`, and that an intentional overlap is permitted with `OverlapPolicy::Allow` or a stacking parent.
- **Fixed:** a color picker edit that still describes the bound color, such as a hue step on a grey, no longer rewrites the binding with a value rounded through HSV. A saved hex or channel field now writes exactly the color it parsed.

## 0.3.9–0.3.17 (2026-10-19): Live data, commands and the mixer example

Rows can stretch to their full height, parameters gained a power mapping, and errors, lookup and commands became more convenient. Geometry validation stopped producing false reports before the first viewport. The `live_mixer` example ties these together with a producer thread, level meters and parameter sinks.

- **Added:**
  - `ParameterMapping::Power(PowerExponent)` maps a position to `minimum + range * position^exponent` over any finite domain. `PowerExponent` must be finite and within 0.1 to 10 (`ParameterError::InvalidExponent`).
  - `MenuItem::on_change` reports user writes of toggle and radio rows. Choosing an already selected radio row no longer writes its value again.
  - `Container::background_role_signal`, a bound theme fill that repaints without layout.
  - `Icon::Pause`.
  - `CaptureTarget::window` and `CaptureTarget::region`, built from a `NativeWindowId`.
  - `Button::command`, and `KeyChord::named` for a chord of one named key.
  - In an application callback, `?` converts a `WorkspaceError`, `WindowError` or `CaptureError` into `RuntimeError::Application`.
  - The `live_mixer` example: a bounded command queue, `series_bridge` levels in `LevelMeter`s, non-blocking parameter sinks that fault when full, Retry through `retry_outcome`, an undo history refreshed through a mailbox, and a saved workspace.
- **Changed:**
  - A `Row` with `Align::Stretch` fills its allocated height instead of its tallest child's height, so a vertical `Slider` or `LevelMeter` can span a whole strip. A root row with `Stretch` fills the window.
  - Geometry validation runs only after a layout with a viewport that has area. `Runtime::validation_report` is `pending` until then, and while any window has pending updates or layout.
- **Breaking:** `Runtime::find_address(..)` → `Runtime::find(..)`, whose inspection path is passed without its root segment. The old full internal path lookup `Runtime::find(..)` → `Runtime::find_path(..)`.

## 0.3.2–0.3.8 (2026-10-18): Improvements for application authors

This range reworked binding conventions and added value-driven children and flex layout in rows and columns. It also added selection notifications, background work, interval timers, application-lifetime resources and a desktop build without wgpu.

- **Added:**
  - `Signal::update`, `Signal::toggle` and `Signal::set_changed`. A `Signal` dereferences to its `ReadSignal`, and builders accept `&signal` through `Into`.
  - `IntoElement::key` keys a component in place, as in `Label::new("Ready").key("status")`, and is in the prelude.
  - `Dynamic`, `Show` and `Each` describe children from a binding, with a `DescribeContext`. They detect a new value by its write counter, so values need not implement `PartialEq`.
  - `on_selection_change` on `VirtualList`, `Tree` and `Table` reports a `ListSelectionChange` with its `ChangeOrigin` after the flush settles.
  - `Element::with_grow`, `Element::spacer` and `MeasureContext::grow`. `Row::justify`/`Column::justify` take a `Justify`, and `align` takes an `Align` that includes `Stretch`. The defaults keep the previous layout.
  - `Runtime::keep_alive`, `Runtime::spawn` with `BackgroundTask::cancel`, and `Runtime::MAX_BACKGROUND_TASKS`.
  - `Runtime::start_interval` with `IntervalTimer::cancel`, `interval_deadline` and `service_intervals`.
  - `Runtime::last_failure`, which the native application's default error report includes.
  - New errors: `RuntimeError::BackgroundFull`, `BackgroundSpawn`, `BackgroundPanicked` and `IntervalsFull`.
  - `TextField::on_binding_refusal` and `TextArea::on_binding_refusal` report when a value the application wrote is refused and when an acceptable value ends the refusal.
  - The `desktop-cpu` feature builds `App` with the CPU renderer through softbuffer and without wgpu (use `default-features = false`). The default `desktop` feature now enables `gpu`, which builds on the new `native` feature; `native` alone has no renderer and refuses to compile.
- **Changed:**
  - `LimitError` and `ResourceKind` now live in `pert-ui-runtime` and exist in every configuration.
  - `VirtualList` and `Tree` key each row with the source's key, so `ListItems::build` and `TreeItems::build` may use any key.
- **Breaking:**
  - `Signal::set` returns `Result<(), RuntimeError>`. Use `set_changed` for the old changed flag. `FieldHandle::set` and `ZoomController::set` are split the same way.
  - `X::new(signal.clone())` → `X::new(&signal)`, and `signal.read_only().map(..)` → `signal.map(..)`.
  - A custom component's own `key(&self)` method called through `&mut self` now conflicts with `IntoElement::key`. Call it as `Self::key(self)` or rename it.
  - `MenuItem::enabled_signal` → `MenuItem::enabled`, which takes the same binding.

## 0.3.1 (2026-10-18)

This release has no library changes. Repository: the facade's integration tests are grouped by area into a few test binaries, and CI builds without debuginfo or incremental caches.

## 0.3.0 — first prerelease (2026-10-15)

First public preview of pert-ui, a Rust desktop GUI library with retained
components, general controls, typed parameters, reactive bindings, inspection,
workspace persistence and a native gallery. The public facade supports GPU
rendering and optional CPU rendering and software presentation.

The API may change before 1.0. The libraries have not been published to a
registry.
