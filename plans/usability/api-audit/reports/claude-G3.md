# claude report: G3 (names and roles: N1, N2, N3, N4, N5, B4)

Commit audited: d4ba108 (library source unchanged at the working tree's 8b9c1d3; read-only, no throwaway build was needed).

## Dispositions

| item | disposition | coverage |
|---|---|---|
| N1 getters | no findings | getters.txt: 23/23 entries classified (table below). Cross-check: `rg 'fn get_\|fn get\b'` over the five packages' `src/` finds no public `get_*` and no public `get` the list misses (the four other hits are private). The one naming oddity among them (`DragPayload::get::<T>` beside `ApplicationError::downcast_ref::<T>`) is recorded under N2 as claude-N2-4. |
| N2 one word per concept | findings: claude-N2-1 … claude-N2-5 | Every public method name in the five semantic lists (1,992 method rows, 534 types), tabulated by frequency and searched for the nine rubric synonym sets plus the further sets found (reactive-read suffixes, `reorderable`/`resizable` versus `*_permission`, role-table accessors, typed downcast, `is_open`/`is_active`, `is_done`/`is_finished`, `on_select`/`on_selection_change`). Docs read for each lead; contracts "Public naming rules", "Permissions and availability", "Text and draft editing"/"Editor sessions". |
| N3 units | findings: claude-N3-1 … claude-N3-3 | numeric.txt: 242/242 entries classified (unitless count/index/id/ratio, unit in type, unit in name, or unit missing); plus the length-, time- and size-taking builders found by name in the semantic lists (`gap`, `padding`, `width`, `extent`, `*_height`, `debounce`, `peak_hold`, …) to compare with the bare-numeric ones. |
| N4 boolean parameters | findings: claude-N4-1 | bool-params.txt: 78/78 entries (21 non-builder, 57 builder; the only two-bool entry is `Component::tab_navigation`). Builders checked for non-property transformations (`TextSelection::moved`, `at_point`). Cross-check of single-line `pub fn … : bool` signatures in source found nothing missing. |
| N5 Rust conventions | findings: claude-N5-1 (C-WORD-ORDER, proposal); C-CASE, C-CONV, C-ITER, C-ITER-TY: no findings | C-CASE: every identifier in `api/*/*.txt` scanned for upper-case runs (only `BTreeMap`/`BTreeSet` from std, `NotADirectory`, `NotAParticipant`, constants). C-CONV: conversion-methods.txt 24/24 (receiver, cost, ownership checked; table below), plus a name scan of the semantic lists for `as_`/`to_`/`into_`. C-ITER/C-ITER-TY: every collection-like type in the semantic lists (methods `iter`, `entries`, `names`, `elements`, `commands`, `keys`, `segments`, `len`/`is_empty`). C-WORD-ORDER: all 46 error types and the related-name families (controllers, receipts, statuses, drag/drop, pick/palette). |
| B4 role suffixes | findings: claude-B4-1 … claude-B4-4 | All 534 public types in the five semantic lists classified (complete table in "Roles relied on"), using each type's rustdoc summary, derives and methods read from source, and types.txt's traits/auto traits; source read for every suffix lead (all 11 `*Controller`, 7 `*Receipt`, 12 `*Outcome`, 4 `*State`, the `*Status`, `*Snapshot` and `*Inspection` types, and the cloneable `!Send` handles). |

### N1 classification (getters.txt, 23 entries)

| getter | use | file:line |
|---|---|---|
| `Capacity::get`, `NormalizedValue::get`, `PowerExponent::get`, `Length::get`, `Magnification::get`, `UiZoom::get`, `ViewScale::get` | validated value's raw content | core live/mod.rs:206, normalized.rs:83, parameter.rs:85; runtime containers.rs:30, pointer.rs:127, runtime/settings.rs:50, transform.rs:302 |
| `EditSequence::get`, `EditSource::get`, `Incarnation::get`, `Revision::get`, `Sequence::get`, `Timestamp::get`, `HistoryEntryId::get` | raw content of a newtype (number/identity wrapper) | core edit.rs:42, :70, :20, live/mod.rs:55, :91, :130; runtime navigation_history.rs:21 |
| `Signal::get`, `ReadSignal::get`, `Permission::get` | cell-like read (`Permission` reads its fixed value or bound signals) | signal.rs:393, :570; permission.rs:147 |
| `ApplicationError::get` | single obvious value (the wrapped error) | error.rs:55 |
| `DragPayload::get::<T>` | single obvious value, typed (downcast) | drag.rs:53 |
| `HistorySnapshot::get(id)`, `ThemeMetrics::get(role)`, `UiStrings::get(role)`, `ViewState::get(name)` | keyed lookup | navigation_history.rs:106; runtime/theme.rs:279; strings.rs:625; workspace.rs:266 |

### N5 C-CONV classification (conversion-methods.txt, 24 entries)

All conform: `as_*` are cheap borrowed views (`ChoiceId::as_str`, `WorkspaceValue::as_flag/as_integer/as_number/as_text/as_map`, workspace.rs:140–174); `to_*` take `&self` or a `Copy` `self` and produce a new value (`Color::to_hex/to_srgb/to_srgb8`, drawing.rs:376–479; `HostValue::to_canonical`, erased.rs:418); `into_*` consume `self` (`ParameterValue::into_any`, `ColorPicker::into_popover`, `FittedText::into_layout`, `{FloatingWindow,Property,Section,Tab}::into_element`, `SendError::into_inner`, `AppShutdownError::into_parts`). `EventContext::to_local{,_delta,_rect}` and `Viewport::to_logical` transform an argument rather than convert `self`; C-CONV does not govern them and their names are accurate. `SampleWindow::iter` returns an opaque `impl DoubleEndedIterator + ExactSizeIterator` (live/mod.rs:435), so C-ITER-TY has no named iterator type to check; no public `*Iter` type exists. `Workspace::entries` and `ViewState::names` iterate types that also carry non-collection data (settings, kind), so `iter` is not required.

## Findings

### claude-N2-1 Reactive and owned reads share names; the reactive variant has four spellings

- **Item and rule kind:** N2, consistency proposal, anchored on the project rule in contracts "Public naming rules" (docs/contracts.md:1157, :1161–1162).
- **Paths and configurations:** pert-ui-runtime (all configurations; re-exported by the facade in all).
- **Observation (signature):** for the same pair of concepts, an owned read of a current value and a `ReadSignal` of it, the API uses:
  - `status()` **owned**: every `*Receipt` (`TaskReceipt::status`, runtime/tasks.rs:33; `RevealReceipt`, reveal.rs:137; `ListRevealReceipt`, virtual_list.rs:234; `ToastReceipt`, notifications.rs:215; `PresentationReceipt`, native_windows.rs:957; `CommandReceipt`, commands.rs:1612; `AuthorityReceipt`, remote.rs:100), `PendingJump::status` (content_search.rs:422);
  - `status()` **owned through `Result`**: `CommandPaletteController::status` (command_palette.rs:330), `QuickPickController::status` (quick_pick.rs:518);
  - `status()` **returning `ReadSignal`**: `DialogController::status` (dialog.rs:86), `WindowController::status` (windows.rs:64), `FieldHandle::status` (field_status.rs:362);
  - `snapshot()` **returning `ReadSignal`**: `Parameter::snapshot` (parameter_binding.rs:316); **owned**: its erased form `AnyParameter::snapshot` (erased.rs:99), `DeliveryMonitor::snapshot` (delivery.rs:279), `ParameterState::snapshot` (core edit.rs:338);
  - reactive variant spelled `*_binding()`: receipts' `status_binding`, `PendingJump::status_binding`, `Runtime::command_enabled_binding` (commands.rs:689); spelled `*_signal()`: `UiSettings::zoom_signal` (settings.rs:330) and its four siblings, `ThemeManager::density_signal`/`mode_signal` (theme.rs:1069, :1147); spelled `observe()`: `DeliveryMonitor::observe` (delivery.rs:284); spelled bare: `Parameter::delivery_status`, `FormStatus::summary`, `ZoomController::view`, `TableController::sort`, and the others listed above.
  - `_signal` also names the opposite direction: the contract makes it the suffix of a builder that **takes** a `ReadSignal<bool>` (`View::visible_signal`, `MenuItem::checked_signal`, …), and `Runtime::command_enabled_signal` (commands.rs:1694) takes one, while `UiSettings::*_signal` **return** one. On `Runtime` the getter is `command_enabled_binding` and the setter `command_enabled_signal`.
- **Rule:** "A `*Receipt` reports one queued request through an owned `status()` and a `status_binding()` for reactive UI" (contracts.md:1161–1162); "the binding variant adds `_signal` and takes a `ReadSignal<bool>`" (contracts.md:1157). The rubric's N2 finding: "one word for two concepts". Here `status`/`snapshot` each mean both an owned value and a binding, and `_signal` means both "takes" and "returns" a signal. No family exception covers controllers' or settings handles' read names.
- **Severity:** medium (a consumer cannot predict whether `x.status()` gives a value or a binding, nor which suffix names the reactive form; `Parameter::snapshot` and `AnyParameter::snapshot` differ in the same family; the compiler catches misuse, but each family has to be learned separately).
- **Confidence:** high.
- **Suggested fix:** adopt the receipt rule generally: `x()` owned, `x_binding()` the `ReadSignal`; rename `UiSettings::*_signal`/`ThemeManager::*_signal` to `*_binding`, `DialogController::status`/`WindowController::status`/`FieldHandle::status`/`Parameter::snapshot` to `*_binding` (adding owned `status()`/`snapshot()` where useful), `DeliveryMonitor::observe` to `snapshot_binding`; keep `_signal` for inputs only. Breaking (renames). Record the convention in "Public naming rules" (policy decision).
- **Destination:** TODO.md.

### claude-N2-2 `reorderable` and `resizable` mean a capability on some types and a restriction on others

- **Item and rule kind:** N2, consistency proposal.
- **Paths and configurations:** pert-ui-runtime and pert-ui (`App::resizable`), all configurations where they exist (`App` in the desktop configurations).
- **Observation (signature):**
  - `reorderable` installs the reorder **callback** (creating the capability) on `VirtualList::reorderable` (virtual_list.rs:970), `Tree::reorderable` (tree.rs:1016) and `Tabs::reorderable` (tabs.rs:535), whose restriction is then `reorder_permission(impl Into<Permission>)` (virtual_list.rs:986, tree.rs:1036, table.rs:1432); on `ToolWindow::reorderable` (tool_windows/mod.rs:403) and `ToolWindowLayout::reorderable` (:846) `reorderable` **is** the Permission restriction.
  - `resizable(impl Into<Permission>)` is the restriction on `SplitPane` (split.rs:354), `RegionLayout` (region_layout.rs:209), `PropertyGrid` (property_grid.rs:356), `FloatingWindow` (windows.rs:412), `ToolWindow`, `ToolWindowLayout` (:813); `Table`'s restriction is `resize_permission` (table.rs:1421); `TableColumn::resizable(bool)` (table.rs:115) and `App::resizable(bool)` (app.rs:375) declare the capability.
- **Rule:** N2 "one word for two concepts". Contracts "Permissions and availability" explains why lists and tables keep a callback or column flag beside the restriction ("column flags and list callbacks remain required", contracts.md:296–298), but no contract sentence explains the different names for the same restriction.
- **Severity:** low (types differ, so misuse fails to compile; finding the restriction on a list or table costs a search).
- **Confidence:** high.
- **Suggested fix:** name every Permission restriction the same way, either `*_permission` everywhere or the adjective form everywhere, and move the list callbacks to an `on_reorder`-style name (`Tabs::reorderable` already names its closure `on_reorder`). Breaking.
- **Destination:** TODO.md.

### claude-N2-3 The three role-keyed theme tables use three accessor vocabularies

- **Item and rule kind:** N2, consistency proposal.
- **Paths and configurations:** `Palette`, `ThemeMetrics`, `UiStrings` (pert-ui-runtime, all).
- **Observation (signature):** each is a complete validated table keyed by a role enum, read and replaced the same way, but `Palette::color(role)` / `Palette::with_color(role, color)` (theme.rs:856, :875), `ThemeMetrics::get(role)` / `ThemeMetrics::with_metric(role, value)` (theme.rs:279, :290), `UiStrings::get(role)` / `UiStrings::with(role, text)` (strings.rs:625, :601).
- **Rule:** N2 "two words for one concept".
- **Severity:** low. **Confidence:** high.
- **Suggested fix:** one pair for all three (for example `get(role)` and `with(role, value)`, keeping `Painter::color`/`Painter::metric` as the paint-time readers). Breaking.
- **Destination:** TODO.md.

### claude-N2-4 Typed downcast is `get::<T>` on `DragPayload` and `downcast_ref::<T>` on `ApplicationError`

- **Item and rule kind:** N2, consistency proposal (N1 classifies both `get`s as legitimate).
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature):** `DragPayload::get::<T>() -> Option<&T>` downcasts (drag.rs:53, body `self.value.downcast_ref()`); `ApplicationError::downcast_ref::<T>() -> Option<&T>` does the same (error.rs:60), while `ApplicationError::get()` returns the untyped wrapped error (error.rs:55). `get` thus means "typed downcast" on one type and "untyped inner value" on the other, and the typed downcast has two names.
- **Rule:** N2.
- **Severity:** low. **Confidence:** high.
- **Suggested fix:** rename `DragPayload::get` to `downcast_ref`, the std name (`Any`, `Error`). Breaking.
- **Destination:** TODO.md.

### claude-N2-5 Minor synonym pairs for one concept

- **Item and rule kind:** N2, consistency proposal.
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature):**
  - an edit begun and not yet ended: `EditorSession::is_open` (editor_session.rs:188) and `DraftEditorController::is_active` (text_fields/draft.rs:375); both families use `begin`/`end`;
  - work that has ended: `ContentSearch::is_done` ("has visited everything", content_search.rs:310) and `ProgressStatus::is_finished` ("has ended", progress.rs:74);
  - a selection change notification: `Tabs::on_select` taking `SelectionChange` (tabs.rs:607) and `VirtualList`/`Tree`/`Table::on_selection_change` taking `ListSelectionChange` (virtual_list.rs:872, tree.rs:1054, table.rs:1556);
  - one boolean property under two names: `ToolWindowLayout::show_names(bool)` (tool_windows/mod.rs:916) and `ToolWindowLayout::names(impl Into<Signal<bool>>)` (:924), with `ToolWindowController::names`/`set_names` (:2510, :2491). (Whether the pair should be `names`/`names_signal` is B1's boolean-builder rule, contracts.md:1156–1157.)
- **Rule:** N2 "two words for one concept". AGENTS.md "Do not change authority, callback timing or cancellation merely for spelling symmetry" limits the fix to names; the `on_select`/`on_selection_change` timings are contract-governed (component-owned versus application-owned selection), so only the name is in question there.
- **Severity:** low. **Confidence:** medium (each pair is arguably distinguishable in context).
- **Suggested fix:** pick one word per pair (`is_open` or `is_active`; `is_finished`; `on_selection_change`; `names`/`names_signal`). Breaking.
- **Destination:** TODO.md.

### claude-N3-1 `pert_ui_wgpu::Renderer` takes physical pixels as two bare `u32`s although `PhysicalExtent` exists

- **Item and rule kind:** N3, project rule (AGENTS.md "Rust and data invariants": "Give domain quantities and identities appropriate validated types").
- **Paths and configurations:** `pert_ui_wgpu::Renderer::new`, `new_surface`, `resize` (pert-ui-wgpu, its only configuration).
- **Observation (signature):** `new(target, width: u32, height: u32, notify)` (pert-ui-wgpu/src/lib.rs:391), `new_surface(&self, target, width: u32, height: u32)` (:1271) and `resize(&mut self, width: u32, height: u32)` (:485) take the surface size as bare integers, while `Renderer::extent()` returns `Option<PhysicalExtent>` (:468) and the runtime has `PhysicalExtent` ("Integer extent of a native window's drawable area in physical pixels", pert-ui-runtime/src/drawing.rs:855–867). Only `resize`'s rustdoc names the unit ("a physical-pixel extent", lib.rs:475); `new`'s (lib.rs:357–390) and `new_surface`'s (:1250–1270) do not.
- **Rule:** N3: "A numeric parameter or return value with a unit says so, through its type … or its name."
- **Severity:** medium (a host that passes a logical size presents a wrongly sized surface on a scaled display; nothing in the two signatures or their rustdoc says which pixel space is meant). **Confidence:** high.
- **Suggested fix:** take `PhysicalExtent` in all three (matching `extent()`), or at least name the unit in the rustdoc of `new` and `new_surface`. Breaking for the signature change.
- **Destination:** TODO.md.

### claude-N3-2 Logical-pixel lengths are `f32` in some builders and `Length` in their siblings

- **Item and rule kind:** N3, project rule (AGENTS.md "Give domain quantities … appropriate validated types"); consistency proposal for the choice of type.
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature):**
  - bare `f32` lengths, validated inside and reported as `GeometryError`: `CommandPalette::width(width: f32)` (command_palette.rs:611), `QuickPick::width(width: f32)` (quick_pick.rs:975), `VirtualList::new(items, row_height: f32)` and `estimated(items, estimate: f32)`, `Table::new(controller, row_height: f32)` (table.rs:1441), `Tree::new(controller, row_height: f32)`, `FileBrowser::new(controller, row_height: f32)`; names give the dimension, only rustdoc gives the unit ("logical pixels", e.g. quick_pick.rs:966; "logical row height", virtual_list.rs:630);
  - the same quantity typed `Length` ("Validated finite nonnegative extent in logical pixels", containers.rs:12): `PropertyGrid::label_width(Length)`, `TableColumn::width(Length)`, `TableController::set_column_width(.., Length)`, `RegionLayout::{top,bottom}_height/{left,right}_width(Length)`, `ToolWindow::extent(Length)`, `TextSpan::size(Length)`, `ClickCounter::with_distance(Length)`;
  - within one type: `ThemeMetrics::with_metric(role, value: Length)` (theme.rs:290) but `ThemeMetrics::get(role) -> f32` (theme.rs:279); `Painter::metric`/`MeasureContext::metric` also return `f32`.
- **Rule:** N3 (as above). Positivity (width and row height must be greater than zero, which `Length` alone does not enforce) explains a check, not the bare type.
- **Severity:** low. **Confidence:** medium (the library's documented default unit is the logical pixel, so the risk is inconsistency more than misuse).
- **Suggested fix:** take `Length` (returning `GeometryError::Empty` for zero where needed) and return `Length` from `ThemeMetrics::get`, or record "bare `f32` geometry is in logical pixels" as a documented convention in contracts and accept the difference. Breaking for the signature change; the alternative is a policy decision.
- **Destination:** TODO.md.

### claude-N3-3 `AxisMapping` mixes domain values and pixels as `f64`, and three pixel parameters do not say so

- **Item and rule kind:** N3, project rule.
- **Paths and configurations:** `pert_ui_runtime::AxisMapping` (all).
- **Observation (signature):** every value of the type is `f64`, half in domain units and half in pixels. Most pixel parameters say so (`pixel(value)`, `value(pixel)`, `panned(pixels)`, `tick_step(min_pixels)`, `new(start, pixels_per_unit)`), but `visible(extent)` (axis_mapping.rs:88, "`extent` pixels"), `ticks(extent, min_pixels)` (:146, "`extent` pixels") and `zoomed_at(pixels_per_unit, anchor)` (:101, "the pixel `anchor`") name pixel quantities only in rustdoc.
- **Rule:** N3.
- **Severity:** low (passing a domain value as `anchor` or `extent` compiles and silently gives a wrong range or zoom centre). **Confidence:** high.
- **Suggested fix:** rename to `extent_pixels` and `anchor_pixel` (non-breaking for callers, since Rust arguments are positional; breaking only for documentation links).
- **Destination:** TODO.md.

### claude-N4-1 Boolean parameters whose meaning the call site does not show

- **Item and rule kind:** N4, guideline C-CUSTOM-TYPE.
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature):**
  - `Runtime::focus_next(&mut self, reverse: bool)` (runtime/pointer.rs:153): its call sites need comments to be read, e.g. `ui.focus_next(false)?; // the document: titles are not Tab stops` (menu_bar.rs:608), and the guide (pert-ui/src/guide/surfaces.md:151) and runtime README (:475, :1151) show `focus_next(false)`;
  - `Component::tab_navigation(&mut self, cx, reverse: bool, repeat: bool)` (runtime.rs:1383), the only two-bool function, carries the same direction plus an OS-repeat flag although `KeyTransition::Repeated` already models repeats (input.rs:220–227);
  - `EventContext::request_reveal(&self, target, focus: bool)` (runtime/reveal.rs:260; its own example calls `cx.request_reveal(&target, true)`), while the `Runtime` counterpart splits the same choice into `Runtime::reveal` and `Runtime::reveal_and_focus` (reveal.rs:347, :394);
  - `TextSelection::moved(self, layout, movement, extend: bool)` and `at_point(self, layout, point, extend: bool)` (text_selection.rs:62, :87) are listed as builders (self → Self) but compute a new selection rather than set a property, so the builder exemption does not fit; `extend` is not visible at a call such as `moved(&layout, TextMovement::Left, true)`;
  - `ToolWindowChange::new(key, expanded: bool, origin)` (tool_windows/mod.rs:247), a notification constructed by tests and hosts.
  The remaining 17 non-builder entries are setters or state callbacks whose name states the property (`set_visible`, `set_modified`, `set_profiling`, `allow_diagnostic_commands`, `Component::hover_changed`, …) or constructors whose bool is the value (`ParameterSpec::boolean`, `WorkspaceValue::flag`); `Runtime::command_in_modals(id, allowed)` and `command_repeats(id, repeat)` read as questions but name their property and are not counted.
- **Rule:** C-CUSTOM-TYPE ("Arguments convey meaning through types, not `bool` or `Option`"); rubric N4: "A non-builder `bool` parameter whose meaning the call site cannot show is an enum."
- **Severity:** low. **Confidence:** high for `focus_next`, `tab_navigation` and `request_reveal`; medium for the rest.
- **Suggested fix:** a `FocusDirection { Forward, Backward }` for `focus_next` and `tab_navigation` (with `KeyTransition` or a dedicated enum for `repeat`); `EventContext::request_reveal` plus `request_reveal_and_focus`, mirroring `Runtime`; a `SelectionExtent { Move, Extend }` (or `moved`/`extended` pairs) for `TextSelection`. Breaking.
- **Destination:** TODO.md.

### claude-N5-1 Related drag and picker names drop their family prefix

- **Item and rule kind:** N5, guideline C-WORD-ORDER (consistent naming of related types); consistency proposal.
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature):**
  - The reorderable-collection drag family is `ListDrop`, `ListDropMark`, `ListDropPlacement`, `ListDropPoint`, `ListDragItems` (list_drag.rs:20, :64; virtual_list.rs:524–592), but `DropPosition` (list_drag.rs:47, "Where dropped items go, expressed against the target's current keys"), used only by `ListDrop::position` and `ListDropPlacement::position`, has the prefix of the generic drag types `DropOperation`, `DropOperations`, `DropResponse` (drag.rs:74–307). Slice 3's F3 records that testers had to find `ListDrop` and `DropPosition`.
  - The command palette's status is `CommandPaletteStatus` (command_palette.rs:120) while the quick pick's is `PickStatus` (quick_pick.rs:281), with `QuickPick`/`QuickPickController` and `CommandPalette`/`CommandPaletteController` otherwise parallel.
- **Rule:** C-WORD-ORDER asks related names to be formed consistently; the error-type order (subject or operation, then `Error`) is consistent across all 46 error types and is not a finding.
- **Severity:** low. **Confidence:** medium.
- **Suggested fix:** `ListDropPosition`; `QuickPickStatus` (or `PaletteStatus`). Breaking.
- **Destination:** TODO.md (or app-author slice 9 if the maintainer treats it as discovery).

### claude-B4-1 Cloneable UI-thread handles that drive a widget under suffixes other than `*Controller`

- **Item and rule kind:** B4, project rule (contracts "Public naming rules").
- **Paths and configurations:** pert-ui-runtime, all.
- **Observation (signature and behavior):**
  - `ContextMenuOpener` (menu.rs:1295–1305): `#[derive(Clone)]`, `!Send` (types.txt), "Opens a mounted [`ContextMenu`] from application code … Clones share the menu"; obtained from `ContextMenu::opener` (menu.rs:1380). It is exactly the role of `DialogController`, `QuickPickController` and `CommandPaletteController`, which open their surfaces.
  - `ToastQueue` (notifications.rs:253–265): `#[derive(Clone)]`, `!Send`, "Cloneable UI-local notification sender … One [`ToastHost`] owns presentation … posting invalidates only that host"; the host takes it in `ToastHost::new(queue, content)` (notifications.rs:407). Pushing drives that one widget.
  - `FormStatus` (field_status.rs:173–216) and `FieldHandle` (:298–304): both `#[derive(Clone)]`, `!Send`; a `FieldHandle` is given to `Field::form` (:520) "so the field follows the status and the form can focus its control", and `FormStatus::focus_first_error` moves focus to a field's control (:284). `FormStatus` is also the only `*Status` that is a handle; every other `*Status` (`DialogStatus`, `ToastStatus`, `FieldStatus`, `TaskStatus`, …) is an owned value enum. `FieldHandle` is the only `*Handle`.
  - Considered and not counted: `ListSelection` (application-owned selection, which the contract places outside the suffix rule: "Collection selection remains application-owned and addressed by stable key", contracts.md:1160–1161); `NavigationHistory`, `ListFilter`, `ListSource`/`TableSource`/`TreeSource` (models or derived data a widget binds to, addressing no widget instance); `EditorSession` (one edit at a time across whichever editor attaches, governed by contracts "Editor sessions"); `Notifier`, `ThemeManager`, `UiSettings`, `FontManager`, `IconManager` (runtime-wide or scheduling handles).
- **Rule:** "Cloneable UI-thread handles that drive a widget are `*Controller`" (contracts.md:1159). No family exception names these types.
- **Severity:** low. **Confidence:** high for `ContextMenuOpener`; medium for `ToastQueue`, `FormStatus` and `FieldHandle` (each could be read as application-owned state like `ListSelection`, which would need the contract to say so).
- **Suggested fix:** `ContextMenuController`; `ToastController` (or record the queue as an application-owned model in the contract); `FormController` and `FieldController` (or `FormStatus` → `Form…` and a contract sentence placing form status beside collection selection). Breaking; the alternative is a policy decision on the contract sentence.
- **Destination:** TODO.md.

### claude-B4-2 `PendingJump` is a receipt under another suffix

- **Item and rule kind:** B4, project rule.
- **Paths and configurations:** `pert_ui_runtime::PendingJump`, all.
- **Observation (signature):** `PendingJump` (content_search.rs:409–431) "holds the collection's reveal receipt" and exposes exactly the receipt interface, an owned `status() -> ListRevealStatus` and `status_binding() -> ReadSignal<ListRevealStatus>`, for one queued row reveal; it is also the continuation passed to `Runtime::resume_content_jump`.
- **Rule:** "A `*Receipt` reports one queued request through an owned `status()` and a `status_binding()`" (contracts.md:1161–1162): a type in the receipt role under another suffix.
- **Severity:** low. **Confidence:** medium (its second role as a continuation token justifies a distinct name).
- **Suggested fix:** keep the name for the continuation and replace the two forwarding methods with `receipt(&self) -> &ListRevealReceipt`, so the receipt role carries the receipt name. Breaking.
- **Destination:** TODO.md.

### claude-B4-3 `*State` names an active state machine, and passive captured data mostly use other suffixes (policy question)

- **Item and rule kind:** B4, project rule; policy question.
- **Paths and configurations:** pert-ui-core `ParameterState` (all core configurations, re-exported); pert-ui-runtime snapshot and inspection types (all).
- **Observation (signature):**
  - `pert_ui_core::ParameterState` (core edit.rs:272) is the "Synchronous edit state machine for one parameter" with mutating `begin`, `update`, `commit`, `cancel`, `write_base`, `replace_spec`, `remove` (edit.rs:367, :450, :486 …): not passive captured data.
  - Passive captured data are named `*State` only in `ViewState`, `KeyboardState` and `TextInputState`; the same role is `*Snapshot` (`InspectionSnapshot`, `ProfileSnapshot`, `HistorySnapshot`, `DeliverySnapshot`, `ParameterSnapshot`, `AnyParameterSnapshot`), `*Inspection` (`ComponentInspection`, `WindowInspection`, `DragInspection`, `SourceInspection`, `ParameterInspection`, `BoxModelInspection`, `BorderInspection`), `*Report`, `ChildInfo` ("Owned UI-local snapshot"), `WindowPlacement`. `NavigationHistory::state()` returns a `HistorySnapshot` binding.
- **Rule:** "passive captured data are `*State`" (contracts.md:1159–1160). Read literally, 20-odd types depart; read as the counterpart of `*Controller` (a widget's captured state), only `ParameterState` clearly does, by carrying the suffix in another role.
- **Severity:** low. **Confidence:** medium (the sentence's scope is the question).
- **Suggested fix:** the maintainer decides the scope: either amend the sentence to name `*Snapshot` and `*Inspection` as the suffixes of owned captures and reserve `*State` for a widget's captured state, or rename. Independently, rename `ParameterState` (for example `ParameterEditor` or `ParameterMachine`), breaking.
- **Destination:** TODO.md (rename); the contract wording is the maintainer's decision.

### claude-B4-4 `*Outcome` is used for handling flags and operation results, not only interaction endings (policy question)

- **Item and rule kind:** B4, project rule; policy question.
- **Paths and configurations:** pert-ui-core and pert-ui-runtime, all.
- **Observation (signature):** interaction endings named `*Outcome`: `EditOutcome`, `DialogOutcome`, `DragOutcome`, `EditorOutcome`, `RevealOutcome`. Under the same suffix but not an interaction's end: `InputOutcome` ("Whether an input callback consumed an event", `Ignored`/`Handled`, pointer.rs:216), `TextKeyOutcome` (commands.rs:327), `TextCommandOutcome` (text_input.rs:143), `PreviewOutcome` ("Result of a preview message", `Applied`/`Stale`, core edit.rs:139), `PublishOutcome` ("The result of one publish attempt", core live/mod.rs:143), `FilterOutcome` ("What applying a filter changed", a struct of counts, collection_filter.rs:115), and arguably `CommandOutcome` (admission result, commands.rs:392). Conversely, results documented as outcomes carry other suffixes: `ContentJump` ("The outcome of `Runtime::jump_to_content`", content_search.rs:389), `ChildReveal`, `ListingDelivery`, `ResponseDisposition`, `pert_ui_wgpu::Frame` ("Surface acquisition/presentation outcome", lib.rs:92).
- **Rule:** "A `*Outcome` names how an interaction ended, distinct from `Result` as an error carrier" (contracts.md:1162–1163).
- **Severity:** low. **Confidence:** medium (whether "interaction" covers a dispatch or an operation is the question).
- **Suggested fix:** amend the sentence to "the non-error result of an interaction or operation" (no code change), or rename the handling flags (`InputOutcome` → `InputHandling` …). The latter is breaking and wide; the maintainer decides.
- **Destination:** TODO.md if renames are chosen; otherwise a contract edit.

## Inventory gaps

1. **`From` implementations attributed to their source type (mis-rendered list).** candidates/conversion-traits.txt and the `traits:` column of candidates/types.txt list `impl From<X> for Y` also under `X`, rendered as if `X` implemented `From<X>`, without naming `Y`. Examples: `pert_ui_runtime::TableController	impl From<TableController>	…collection_filter.rs:367` (the source is `impl From<TableController> for FilterTarget`, collection_filter.rs:367); likewise `TreeController` (:372), `ListFilter` (:362), `pert_ui_core::ChoiceId	impl From<ChoiceId>	…erased.rs:156` (an impl for `AnyValue`), and every error type listed with `From<ItsOwnName>` at workspace.rs:654 or capture.rs:270–291 (impls into `RuntimeError`, `WorkspaceError` or `CaptureError`). A filter on rows whose `From<…>` argument names the row's own type finds 46 such rows in conversion-traits.txt. A T6 or T4 auditor reading these rows cannot see the conversion's target. Fix: render the target (`impl From<X> for Y`) or list such impls only under `Y`.
2. **Builder classification in bool-params.txt.** `builder=yes` is assigned to every `self → Self` method, which includes value transformations that are not property builders (`TextSelection::moved`, `TextSelection::at_point`, text_selection.rs:62, :87). N4 needs them; I processed them as non-builders (claude-N4-1). Fix: note in the header that `builder=yes` is syntactic, or mark methods on non-builder types.
No public `get_*` getter, `bool` parameter, `as_`/`to_`/`into_` method or type was found missing from the lists used.

## Outside the assigned items (for the other auditors' attention, not counted)

- `Semantics::read_only(self, read_only: bool)` (and its siblings `disabled`, `invalid`, …) take a `bool`, while the naming rule makes `read_only()` and `disabled()` no-argument shortcuts (contracts.md:1158). `Semantics` is an inspection data builder, so the rule may not be meant for it; B1's auditor should decide.

## Roles relied on

Every public type of the five semantic lists, classified (534 rows; B4 relies on all of them, N2-1 on the controller, receipt and handle rows, N4 on the builder rows). Role vocabulary: controller; controller or receipt role under another suffix (B4 findings); handle (other cloneable or owning UI-thread or `Send` handles that address no single widget); receipt; state (passive data, snapshot or status value); outcome; value with an invariant (validated or opaque value type); plain data; error; trait; widget/builder (components and the descriptions widgets consume); other (contexts, runtime, services, renderers, the edit state machine). Source reference: the definition's file:line from the semantic list, where I read the rustdoc summary, derives and methods; the note quotes or paraphrases what decided the role. Facade rows inherit the semantic list's configuration marks: `pert_ui::App`, `AppError`, `AppShutdownError`, `ErrorAction`, `NativeCloseAction` exist in cpu, default, image-formats, images, no-default-desktop-cpu, serde and svg; `pert_ui::Svg` and `pert_ui::svg::SvgError` in no-default-svg and svg; `pert_ui::images::*` in image-formats, images and no-default-png. No row is unresolved.

| path | role | source reference |
|---|---|---|
| `pert_ui_core::AnyParameterSnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-core/src/erased.rs:179 (family-tagged owned view (snapshot)) |
| `pert_ui_core::AnySpec` | plain data | crates/pert-ui-core/src/erased.rs:24 (family-tagged spec value) |
| `pert_ui_core::AnyValue` | plain data | crates/pert-ui-core/src/erased.rs:112 (family-tagged canonical value) |
| `pert_ui_core::Capacity` | value with an invariant | crates/pert-ui-core/src/live/mod.rs:183 (validated item count, fallible new) |
| `pert_ui_core::ChoiceId` | value with an invariant | crates/pert-ui-core/src/choice.rs:32 (validated option identity, fallible new) |
| `pert_ui_core::ChoiceOption` | plain data | crates/pert-ui-core/src/choice.rs:97 (option id+label) |
| `pert_ui_core::DrainReport` | plain data | crates/pert-ui-core/src/live/mod.rs:312 (counters, public fields) |
| `pert_ui_core::EditError` | error | crates/pert-ui-core/src/edit.rs:189 (rustdoc summary) |
| `pert_ui_core::EditOutcome` | outcome | crates/pert-ui-core/src/edit.rs:163 (how an admitted edit ended) |
| `pert_ui_core::EditSequence` | value with an invariant | crates/pert-ui-core/src/edit.rs:31 (ordered message number newtype) |
| `pert_ui_core::EditSource` | value with an invariant | crates/pert-ui-core/src/edit.rs:59 (opaque origin newtype) |
| `pert_ui_core::EditToken` | value with an invariant | crates/pert-ui-core/src/edit.rs:82 (opaque admitted-edit token) |
| `pert_ui_core::HostValue` | trait | crates/pert-ui-core/src/erased.rs:408 (rustdoc summary) |
| `pert_ui_core::Incarnation` | value with an invariant | crates/pert-ui-core/src/live/mod.rs:47 (restart generation newtype) |
| `pert_ui_core::LiveError` | error | crates/pert-ui-core/src/live/mod.rs:163 (rustdoc summary) |
| `pert_ui_core::LiveItem` | trait | crates/pert-ui-core/src/live/item.rs:36 (rustdoc summary) |
| `pert_ui_core::NormalizedValue` | value with an invariant | crates/pert-ui-core/src/normalized.rs:47 (validated 0..=1) |
| `pert_ui_core::NormalizedValueError` | error | crates/pert-ui-core/src/normalized.rs:114 (rustdoc summary) |
| `pert_ui_core::ParameterError` | error | crates/pert-ui-core/src/parameter.rs:541 (rustdoc summary) |
| `pert_ui_core::ParameterFamily` | plain data | crates/pert-ui-core/src/family.rs:17 (closed enum) |
| `pert_ui_core::ParameterId` | value with an invariant | crates/pert-ui-core/src/parameter_id.rs:44 (identity, fallible new) |
| `pert_ui_core::ParameterIdError` | error | crates/pert-ui-core/src/parameter_id.rs:132 (rustdoc summary) |
| `pert_ui_core::ParameterMapping` | plain data | crates/pert-ui-core/src/parameter.rs:14 (enum) |
| `pert_ui_core::ParameterSnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-core/src/edit.rs:116 (coherent local view (snapshot)) |
| `pert_ui_core::ParameterSpec` | value with an invariant | crates/pert-ui-core/src/parameter.rs:151 (immutable validated metadata, fallible ctors) |
| `pert_ui_core::ParameterState` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-core/src/edit.rs:272 (synchronous edit state machine (active, mutating methods)) |
| `pert_ui_core::ParameterValue` | trait | crates/pert-ui-core/src/family.rs:47 (rustdoc summary) |
| `pert_ui_core::PowerExponent` | value with an invariant | crates/pert-ui-core/src/parameter.rs:63 (validated exponent) |
| `pert_ui_core::PreviewOutcome` | outcome | crates/pert-ui-core/src/edit.rs:139 (result of a preview message (Applied/Stale)) |
| `pert_ui_core::PublishOutcome` | outcome | crates/pert-ui-core/src/live/mod.rs:143 (result of a publish attempt) |
| `pert_ui_core::Revision` | value with an invariant | crates/pert-ui-core/src/edit.rs:12 (revision newtype) |
| `pert_ui_core::Sample` | plain data | crates/pert-ui-core/src/live/mod.rs:286 (received item) |
| `pert_ui_core::SampleWindow` | state (passive data, snapshot or status value) | crates/pert-ui-core/src/live/mod.rs:382 (retained bounded window (collection data)) |
| `pert_ui_core::SampleWindowEntry` | plain data | crates/pert-ui-core/src/live/mod.rs:367 (window entry) |
| `pert_ui_core::Sequence` | value with an invariant | crates/pert-ui-core/src/live/mod.rs:83 (publish number, Option new) |
| `pert_ui_core::SeriesConfig` | value with an invariant | crates/pert-ui-core/src/live/mod.rs:244 (validated config) |
| `pert_ui_core::SeriesConsumer` | handle | crates/pert-ui-core/src/live/series.rs:292 (consumer half of a bridge) |
| `pert_ui_core::SeriesEndpoint` | handle | crates/pert-ui-core/src/live/series.rs:138 (producer half of a bridge) |
| `pert_ui_core::Timestamp` | value with an invariant | crates/pert-ui-core/src/live/mod.rs:120 (producer time newtype) |
| `pert_ui_core::WakeFlag` | handle | crates/pert-ui-core/src/live/wake.rs:36 (coalescing wake flag) |
| `pert_ui_cpu::CpuRenderError` | error | crates/pert-ui-cpu/src/lib.rs:16 (rustdoc summary) |
| `pert_ui_cpu::CpuRenderer` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-cpu/src/lib.rs:108 (renderer) |
| `pert_ui_runtime::Accordion` | widget/builder | crates/pert-ui-runtime/src/collapsible.rs:947 (rustdoc summary) |
| `pert_ui_runtime::AlertDialog` | widget/builder | crates/pert-ui-runtime/src/dialog.rs:493 (rustdoc summary) |
| `pert_ui_runtime::Align` | plain data | crates/pert-ui-runtime/src/containers.rs:421 (rustdoc summary) |
| `pert_ui_runtime::AnyDeliveryMonitor` | handle | crates/pert-ui-runtime/src/parameter_binding/erased.rs:223 (erased DeliveryMonitor) |
| `pert_ui_runtime::AnyParameter` | handle | crates/pert-ui-runtime/src/parameter_binding/erased.rs:58 (erased parameter binding) |
| `pert_ui_runtime::AnyParameterHost` | handle | crates/pert-ui-runtime/src/parameter_binding/erased.rs:255 (erased authoritative owner) |
| `pert_ui_runtime::AnyPreviewEvent` | plain data | crates/pert-ui-runtime/src/parameter_binding/erased.rs:202 (event record) |
| `pert_ui_runtime::AnyTerminalEvent` | plain data | crates/pert-ui-runtime/src/parameter_binding/erased.rs:114 (event record) |
| `pert_ui_runtime::ApplicationError` | error | crates/pert-ui-runtime/src/error.rs:46 (application failure carrier) |
| `pert_ui_runtime::ArcOrigin` | plain data | crates/pert-ui-runtime/src/numeric_controls.rs:36 (rustdoc summary) |
| `pert_ui_runtime::ArrangeContext` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/layout.rs:968 (layout context) |
| `pert_ui_runtime::AuthorityReceipt` | receipt | crates/pert-ui-runtime/src/parameter_binding/remote.rs:80 (status+status_binding) |
| `pert_ui_runtime::AuthorityResponse` | plain data | crates/pert-ui-runtime/src/parameter_binding/remote.rs:40 (external decision) |
| `pert_ui_runtime::AuthorityStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/parameter_binding/remote.rs:49 (status value) |
| `pert_ui_runtime::AxisMapping` | value with an invariant | crates/pert-ui-runtime/src/axis_mapping.rs:37 (validated mapping) |
| `pert_ui_runtime::BackgroundTask` | handle | crates/pert-ui-runtime/src/runtime/application.rs:68 (task handle (cancel), drives no widget) |
| `pert_ui_runtime::Banner` | widget/builder | crates/pert-ui-runtime/src/notifications.rs:25 (rustdoc summary) |
| `pert_ui_runtime::BindingError` | error | crates/pert-ui-runtime/src/parameter_binding.rs:62 (rustdoc summary) |
| `pert_ui_runtime::BorderInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:142 (captured observation) |
| `pert_ui_runtime::BoxModelInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:96 (captured observation) |
| `pert_ui_runtime::Breadcrumb` | widget/builder | crates/pert-ui-runtime/src/breadcrumb.rs:143 (rustdoc summary) |
| `pert_ui_runtime::BreadcrumbSegment` | widget/builder | crates/pert-ui-runtime/src/breadcrumb.rs:28 (description consumed by Breadcrumb) |
| `pert_ui_runtime::Button` | widget/builder | crates/pert-ui-runtime/src/controls.rs:445 (rustdoc summary) |
| `pert_ui_runtime::Capture` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/capture.rs:487 (finished screenshot) |
| `pert_ui_runtime::CaptureError` | error | crates/pert-ui-runtime/src/capture.rs:164 (rustdoc summary) |
| `pert_ui_runtime::CaptureJob` | plain data | crates/pert-ui-runtime/src/capture.rs:532 (host work item) |
| `pert_ui_runtime::CaptureRequest` | plain data | crates/pert-ui-runtime/src/capture.rs:121 (request description) |
| `pert_ui_runtime::CaptureTarget` | plain data | crates/pert-ui-runtime/src/capture.rs:38 (rustdoc summary) |
| `pert_ui_runtime::CellEdit` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/table/editing.rs:29 (editor-callback context) |
| `pert_ui_runtime::ChangeOrigin` | plain data | crates/pert-ui-runtime/src/runtime/notify.rs:152 (rustdoc summary) |
| `pert_ui_runtime::Checkbox` | widget/builder | crates/pert-ui-runtime/src/choice_controls.rs:647 (rustdoc summary) |
| `pert_ui_runtime::ChildInfo` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime.rs:2482 (owned snapshot of a child) |
| `pert_ui_runtime::ChildReveal` | outcome | crates/pert-ui-runtime/src/runtime/reveal.rs:11 (what a container did when asked to reveal (outcome under another suffix)) |
| `pert_ui_runtime::Chip` | widget/builder | crates/pert-ui-runtime/src/chip.rs:72 (rustdoc summary) |
| `pert_ui_runtime::Choice` | plain data | crates/pert-ui-runtime/src/choice_controls.rs:16 (typed option description) |
| `pert_ui_runtime::ChoiceError` | error | crates/pert-ui-runtime/src/choice_controls.rs:113 (rustdoc summary) |
| `pert_ui_runtime::ClickCounter` | plain data | crates/pert-ui-runtime/src/click.rs:66 (stateful counter value (Clone, Send, PartialEq)) |
| `pert_ui_runtime::Clipboard` | trait | crates/pert-ui-runtime/src/text_input.rs:192 (rustdoc summary) |
| `pert_ui_runtime::ClipboardError` | error | crates/pert-ui-runtime/src/text_input.rs:158 (rustdoc summary) |
| `pert_ui_runtime::Collapsible` | widget/builder | crates/pert-ui-runtime/src/collapsible.rs:464 (rustdoc summary) |
| `pert_ui_runtime::CollectionFilter` | widget/builder | crates/pert-ui-runtime/src/collection_filter.rs:499 (rustdoc summary) |
| `pert_ui_runtime::Color` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:210 (validated linear RGBA) |
| `pert_ui_runtime::ColorPicker` | widget/builder | crates/pert-ui-runtime/src/color_picker.rs:386 (rustdoc summary) |
| `pert_ui_runtime::ColorRole` | plain data | crates/pert-ui-runtime/src/runtime/theme.rs:357 (role key) |
| `pert_ui_runtime::ColorSwatch` | widget/builder | crates/pert-ui-runtime/src/color_swatch.rs:96 (rustdoc summary) |
| `pert_ui_runtime::Column` | widget/builder | crates/pert-ui-runtime/src/containers.rs:898 (rustdoc summary) |
| `pert_ui_runtime::CommandContext` | handle | crates/pert-ui-runtime/src/runtime/commands.rs:365 (owned retained command context) |
| `pert_ui_runtime::CommandEntry` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime/commands.rs:419 (owned row captured from a context) |
| `pert_ui_runtime::CommandId` | value with an invariant | crates/pert-ui-runtime/src/runtime/commands.rs:380 (runtime identity) |
| `pert_ui_runtime::CommandInfo` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:404 (name + enabled, public fields) |
| `pert_ui_runtime::CommandKey` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:7 (rustdoc summary) |
| `pert_ui_runtime::CommandKeyInput` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:269 (owned native key translation) |
| `pert_ui_runtime::CommandMatcher` | plain data | crates/pert-ui-runtime/src/command_palette.rs:40 (replaceable matching policy (closure value)) |
| `pert_ui_runtime::CommandOutcome` | outcome | crates/pert-ui-runtime/src/runtime/commands.rs:392 (command admission result) |
| `pert_ui_runtime::CommandPalette` | widget/builder | crates/pert-ui-runtime/src/command_palette.rs:505 (rustdoc summary) |
| `pert_ui_runtime::CommandPaletteController` | controller | crates/pert-ui-runtime/src/command_palette.rs:208 (rustdoc summary) |
| `pert_ui_runtime::CommandPaletteStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/command_palette.rs:120 (status value) |
| `pert_ui_runtime::CommandReceipt` | receipt | crates/pert-ui-runtime/src/runtime/commands.rs:1607 (status+status_binding) |
| `pert_ui_runtime::CommandScope` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:340 (rustdoc summary) |
| `pert_ui_runtime::CommandSource` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:352 (rustdoc summary) |
| `pert_ui_runtime::CommandStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime/commands.rs:1585 (status value) |
| `pert_ui_runtime::Component` | trait | crates/pert-ui-runtime/src/runtime.rs:1019 (rustdoc summary) |
| `pert_ui_runtime::ComponentDetails` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:646 (inspection contribution) |
| `pert_ui_runtime::ComponentInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:667 (captured observation) |
| `pert_ui_runtime::Constraints` | value with an invariant | crates/pert-ui-runtime/src/layout.rs:169 (measurement space) |
| `pert_ui_runtime::Container` | widget/builder | crates/pert-ui-runtime/src/containers.rs:63 (rustdoc summary) |
| `pert_ui_runtime::ContentField` | plain data | crates/pert-ui-runtime/src/content_search.rs:164 (rustdoc summary) |
| `pert_ui_runtime::ContentHit` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/content_search.rs:176 (search hit) |
| `pert_ui_runtime::ContentJump` | outcome | crates/pert-ui-runtime/src/content_search.rs:392 (outcome of jump_to_content (under another suffix)) |
| `pert_ui_runtime::ContentQuery` | value with an invariant | crates/pert-ui-runtime/src/content_search.rs:22 (validated query) |
| `pert_ui_runtime::ContentRows` | trait | crates/pert-ui-runtime/src/content_search.rs:350 (rustdoc summary) |
| `pert_ui_runtime::ContentSearch` | handle | crates/pert-ui-runtime/src/content_search.rs:238 (resumable search session) |
| `pert_ui_runtime::ContentSearchError` | error | crates/pert-ui-runtime/src/content_search.rs:436 (rustdoc summary) |
| `pert_ui_runtime::ContentStep` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/content_search.rs:204 (what one step found) |
| `pert_ui_runtime::ContentTarget` | plain data | crates/pert-ui-runtime/src/content_search.rs:148 (rustdoc summary) |
| `pert_ui_runtime::ContextMenu` | widget/builder | crates/pert-ui-runtime/src/menu.rs:1250 (rustdoc summary) |
| `pert_ui_runtime::ContextMenuOpener` | controller role under another suffix | crates/pert-ui-runtime/src/menu.rs:1303 (cloneable UI-thread handle that opens a mounted ContextMenu) |
| `pert_ui_runtime::CursorIcon` | plain data | crates/pert-ui-runtime/src/pointer.rs:230 (rustdoc summary) |
| `pert_ui_runtime::DebugMode` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:14 (rustdoc summary) |
| `pert_ui_runtime::DebugOverlay` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:41 (rustdoc summary) |
| `pert_ui_runtime::DecimalInput` | widget/builder | crates/pert-ui-runtime/src/text_fields/number_inputs.rs:203 (rustdoc summary) |
| `pert_ui_runtime::Delivery` | plain data | crates/pert-ui-runtime/src/live.rs:23 (rustdoc summary) |
| `pert_ui_runtime::DeliveryError` | error | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:11 (rustdoc summary) |
| `pert_ui_runtime::DeliveryMonitor` | handle | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:247 (registration report handle) |
| `pert_ui_runtime::DeliveryResolution` | plain data | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:188 (rustdoc summary) |
| `pert_ui_runtime::DeliverySnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:206 (owned delivery report) |
| `pert_ui_runtime::DeliveryStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:176 (status value) |
| `pert_ui_runtime::Density` | plain data | crates/pert-ui-runtime/src/runtime/theme.rs:25 (rustdoc summary) |
| `pert_ui_runtime::Depth` | plain data | crates/pert-ui-runtime/src/inspection.rs:144 (rustdoc summary) |
| `pert_ui_runtime::DescribeContext` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/described.rs:26 (description-callback context) |
| `pert_ui_runtime::DetachedClosePolicy` | plain data | crates/pert-ui-runtime/src/runtime/native_windows.rs:171 (rustdoc summary) |
| `pert_ui_runtime::Detail` | plain data | crates/pert-ui-runtime/src/inspection.rs:162 (rustdoc summary) |
| `pert_ui_runtime::DiagnosticCommand` | plain data | crates/pert-ui-runtime/src/inspection.rs:397 (rustdoc summary) |
| `pert_ui_runtime::Dialog` | widget/builder | crates/pert-ui-runtime/src/dialog.rs:177 (rustdoc summary) |
| `pert_ui_runtime::DialogController` | controller | crates/pert-ui-runtime/src/dialog.rs:47 (rustdoc summary) |
| `pert_ui_runtime::DialogOutcome` | outcome | crates/pert-ui-runtime/src/dialog.rs:11 (terminal dialog response) |
| `pert_ui_runtime::DialogStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/dialog.rs:23 (status value) |
| `pert_ui_runtime::DirectoryEntry` | plain data | crates/pert-ui-runtime/src/file_browser.rs:119 (listing entry) |
| `pert_ui_runtime::DirectoryRequest` | plain data | crates/pert-ui-runtime/src/file_browser.rs:196 (host request (respond)) |
| `pert_ui_runtime::DirectoryResponse` | plain data | crates/pert-ui-runtime/src/file_browser.rs:228 (host answer) |
| `pert_ui_runtime::DisplayList` | plain data | crates/pert-ui-runtime/src/drawing.rs:1200 (owned drawing commands) |
| `pert_ui_runtime::Divider` | widget/builder | crates/pert-ui-runtime/src/views.rs:906 (rustdoc summary) |
| `pert_ui_runtime::DraftAlign` | plain data | crates/pert-ui-runtime/src/text_fields/draft.rs:26 (rustdoc summary) |
| `pert_ui_runtime::DraftEditor` | widget/builder | crates/pert-ui-runtime/src/text_fields/draft.rs:582 (rustdoc summary) |
| `pert_ui_runtime::DraftEditorController` | controller | crates/pert-ui-runtime/src/text_fields/draft.rs:153 (rustdoc summary) |
| `pert_ui_runtime::DraftSelection` | plain data | crates/pert-ui-runtime/src/text_fields/draft.rs:12 (rustdoc summary) |
| `pert_ui_runtime::DragEvent` | plain data | crates/pert-ui-runtime/src/drag.rs:254 (one delivery to a component) |
| `pert_ui_runtime::DragInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/drag.rs:355 (captured observation) |
| `pert_ui_runtime::DragOutcome` | outcome | crates/pert-ui-runtime/src/drag.rs:330 (how a drag session ended) |
| `pert_ui_runtime::DragPayload` | handle | crates/pert-ui-runtime/src/drag.rs:38 (type-erased shared payload) |
| `pert_ui_runtime::DragPhase` | plain data | crates/pert-ui-runtime/src/drag.rs:222 (rustdoc summary) |
| `pert_ui_runtime::DragPreview` | widget/builder | crates/pert-ui-runtime/src/drag.rs:191 (description of the drag preview) |
| `pert_ui_runtime::DragRole` | plain data | crates/pert-ui-runtime/src/drag.rs:243 (rustdoc summary) |
| `pert_ui_runtime::DrawCommand` | plain data | crates/pert-ui-runtime/src/drawing.rs:1026 (rustdoc summary) |
| `pert_ui_runtime::DrawError` | error | crates/pert-ui-runtime/src/drawing.rs:24 (rustdoc summary) |
| `pert_ui_runtime::DropOperation` | plain data | crates/pert-ui-runtime/src/drag.rs:74 (rustdoc summary) |
| `pert_ui_runtime::DropOperations` | plain data | crates/pert-ui-runtime/src/drag.rs:112 (bit set) |
| `pert_ui_runtime::DropPosition` | plain data | crates/pert-ui-runtime/src/list_drag.rs:47 (rustdoc summary) |
| `pert_ui_runtime::DropResponse` | plain data | crates/pert-ui-runtime/src/drag.rs:307 (component answer) |
| `pert_ui_runtime::Dynamic` | widget/builder | crates/pert-ui-runtime/src/described.rs:178 (rustdoc summary) |
| `pert_ui_runtime::Each` | widget/builder | crates/pert-ui-runtime/src/described.rs:277 (rustdoc summary) |
| `pert_ui_runtime::EdgeMode` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:109 (rustdoc summary) |
| `pert_ui_runtime::EditSession` | handle | crates/pert-ui-runtime/src/parameter_binding.rs:706 (one local edit gesture) |
| `pert_ui_runtime::EditorAttachment` | handle | crates/pert-ui-runtime/src/editor_session.rs:219 (editor-side binding to an EditorSession) |
| `pert_ui_runtime::EditorCompletion` | outcome | crates/pert-ui-runtime/src/editor_session.rs:74 (how one edit ended (record of trigger + EditorOutcome)) |
| `pert_ui_runtime::EditorOutcome` | outcome | crates/pert-ui-runtime/src/editor_session.rs:50 (result of an edit's completion) |
| `pert_ui_runtime::EditorSession` | handle | crates/pert-ui-runtime/src/editor_session.rs:152 (host-side session (Editor sessions contract)) |
| `pert_ui_runtime::EditorStart` | plain data | crates/pert-ui-runtime/src/editor_session.rs:14 (rustdoc summary) |
| `pert_ui_runtime::EditorTrigger` | plain data | crates/pert-ui-runtime/src/editor_session.rs:29 (rustdoc summary) |
| `pert_ui_runtime::Element` | widget/builder | crates/pert-ui-runtime/src/layout.rs:312 (keyed description) |
| `pert_ui_runtime::EntryKind` | plain data | crates/pert-ui-runtime/src/file_browser.rs:26 (rustdoc summary) |
| `pert_ui_runtime::EventContext` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/runtime.rs:2897 (event context) |
| `pert_ui_runtime::EventHistory` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/inspection.rs:339 (retained events after a cursor) |
| `pert_ui_runtime::EventKind` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:735 (rustdoc summary) |
| `pert_ui_runtime::EventRecord` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:820 (recorded event) |
| `pert_ui_runtime::EventStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:795 (outcome of a recorded operation) |
| `pert_ui_runtime::ExpandMode` | plain data | crates/pert-ui-runtime/src/collapsible.rs:794 (rustdoc summary) |
| `pert_ui_runtime::FailurePhase` | plain data | crates/pert-ui-runtime/src/runtime.rs:77 (rustdoc summary) |
| `pert_ui_runtime::FailureSite` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime.rs:55 (last failure location) |
| `pert_ui_runtime::Field` | widget/builder | crates/pert-ui-runtime/src/field_status.rs:438 (rustdoc summary) |
| `pert_ui_runtime::FieldHandle` | controller role under another suffix | crates/pert-ui-runtime/src/field_status.rs:304 (cloneable UI-thread write handle a Field follows) |
| `pert_ui_runtime::FieldStatus` | plain data | crates/pert-ui-runtime/src/field_status.rs:27 (application-reported status value) |
| `pert_ui_runtime::FileBrowser` | widget/builder | crates/pert-ui-runtime/src/file_browser/view.rs:70 (rustdoc summary) |
| `pert_ui_runtime::FileBrowserController` | controller | crates/pert-ui-runtime/src/file_browser/controller.rs:480 (rustdoc summary) |
| `pert_ui_runtime::FileBrowserError` | error | crates/pert-ui-runtime/src/file_browser.rs:261 (rustdoc summary) |
| `pert_ui_runtime::FillRule` | plain data | crates/pert-ui-runtime/src/geometry.rs:33 (rustdoc summary) |
| `pert_ui_runtime::FilterCounts` | plain data | crates/pert-ui-runtime/src/collection_filter.rs:100 (counts, public fields) |
| `pert_ui_runtime::FilterMatcher` | plain data | crates/pert-ui-runtime/src/collection_filter.rs:39 (replaceable matching policy) |
| `pert_ui_runtime::FilterOutcome` | outcome | crates/pert-ui-runtime/src/collection_filter.rs:115 (what applying a filter changed (operation result)) |
| `pert_ui_runtime::FilterTarget` | plain data | crates/pert-ui-runtime/src/collection_filter.rs:353 (collection selector) |
| `pert_ui_runtime::FittedText` | plain data | crates/pert-ui-runtime/src/text/fit.rs:58 (prepared text) |
| `pert_ui_runtime::FloatingWindow` | widget/builder | crates/pert-ui-runtime/src/windows.rs:195 (description inside WindowLayer) |
| `pert_ui_runtime::FlushReport` | plain data | crates/pert-ui-runtime/src/runtime.rs:3485 (work counts) |
| `pert_ui_runtime::FontData` | plain data | crates/pert-ui-runtime/src/text/fonts.rs:44 (font bytes) |
| `pert_ui_runtime::FontError` | error | crates/pert-ui-runtime/src/text/fonts.rs:121 (rustdoc summary) |
| `pert_ui_runtime::FontId` | value with an invariant | crates/pert-ui-runtime/src/text/fonts.rs:86 (runtime identity) |
| `pert_ui_runtime::FontManager` | handle | crates/pert-ui-runtime/src/runtime/fonts.rs:38 (runtime-wide font registry handle) |
| `pert_ui_runtime::FontRole` | plain data | crates/pert-ui-runtime/src/text/fonts.rs:99 (rustdoc summary) |
| `pert_ui_runtime::Form` | widget/builder | crates/pert-ui-runtime/src/form.rs:53 (rustdoc summary) |
| `pert_ui_runtime::FormStatus` | controller role under another suffix | crates/pert-ui-runtime/src/field_status.rs:216 (cloneable UI-thread form coordinator (drives Field status and focus)) |
| `pert_ui_runtime::FormSummary` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/field_status.rs:101 (counts captured from a form) |
| `pert_ui_runtime::FrameError` | error | crates/pert-ui-runtime/src/capture.rs:224 (rustdoc summary) |
| `pert_ui_runtime::FrameImage` | value with an invariant | crates/pert-ui-runtime/src/capture.rs:317 (validated pixel buffer) |
| `pert_ui_runtime::Freshness` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/live.rs:47 (status value) |
| `pert_ui_runtime::GeometryError` | error | crates/pert-ui-runtime/src/layout.rs:80 (rustdoc summary) |
| `pert_ui_runtime::Grid` | widget/builder | crates/pert-ui-runtime/src/containers.rs:1027 (rustdoc summary) |
| `pert_ui_runtime::Group` | widget/builder | crates/pert-ui-runtime/src/views.rs:136 (rustdoc summary) |
| `pert_ui_runtime::HeldCancellation` | outcome | crates/pert-ui-runtime/src/runtime/held.rs:15 (terminal cancellation of a held press) |
| `pert_ui_runtime::HeldInputId` | value with an invariant | crates/pert-ui-runtime/src/runtime/held.rs:8 (runtime identity) |
| `pert_ui_runtime::HexColorError` | error | crates/pert-ui-runtime/src/drawing.rs:755 (rustdoc summary) |
| `pert_ui_runtime::HighlightError` | error | crates/pert-ui-runtime/src/text/fit.rs:210 (rustdoc summary) |
| `pert_ui_runtime::HistoryButtons` | widget/builder | crates/pert-ui-runtime/src/history_navigator.rs:486 (rustdoc summary) |
| `pert_ui_runtime::HistoryEntry` | plain data | crates/pert-ui-runtime/src/navigation_history.rs:29 (visited place) |
| `pert_ui_runtime::HistoryEntryId` | value with an invariant | crates/pert-ui-runtime/src/navigation_history.rs:15 (runtime identity) |
| `pert_ui_runtime::HistoryNavigator` | widget/builder | crates/pert-ui-runtime/src/history_navigator.rs:111 (rustdoc summary) |
| `pert_ui_runtime::HistorySnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/navigation_history.rs:57 (owned view of a history) |
| `pert_ui_runtime::Icon` | plain data | crates/pert-ui-runtime/src/text/icons.rs:9 (role key) |
| `pert_ui_runtime::IconGlyph` | value with an invariant | crates/pert-ui-runtime/src/text/fonts.rs:149 (validated glyph) |
| `pert_ui_runtime::IconManager` | handle | crates/pert-ui-runtime/src/runtime/fonts.rs:162 (runtime-wide icon overrides handle) |
| `pert_ui_runtime::IconSource` | plain data | crates/pert-ui-runtime/src/text/icons.rs:237 (rustdoc summary) |
| `pert_ui_runtime::IgnoredEntry` | plain data | crates/pert-ui-runtime/src/workspace.rs:719 (reason) |
| `pert_ui_runtime::Image` | widget/builder | crates/pert-ui-runtime/src/image.rs:76 (rustdoc summary) |
| `pert_ui_runtime::ImageFit` | plain data | crates/pert-ui-runtime/src/image.rs:17 (rustdoc summary) |
| `pert_ui_runtime::ImageId` | value with an invariant | crates/pert-ui-runtime/src/resources.rs:85 (scene-local index) |
| `pert_ui_runtime::ImageSampling` | plain data | crates/pert-ui-runtime/src/resources.rs:106 (rustdoc summary) |
| `pert_ui_runtime::InlineChoice` | widget/builder | crates/pert-ui-runtime/src/choice_controls.rs:1585 (rustdoc summary) |
| `pert_ui_runtime::InlineDecimal` | widget/builder | crates/pert-ui-runtime/src/text_fields/number_inputs.rs:274 (rustdoc summary) |
| `pert_ui_runtime::InlineInteger` | widget/builder | crates/pert-ui-runtime/src/text_fields/number_inputs.rs:247 (rustdoc summary) |
| `pert_ui_runtime::InlineReadout` | widget/builder | crates/pert-ui-runtime/src/inline_controls.rs:579 (rustdoc summary) |
| `pert_ui_runtime::InlineSelect` | widget/builder | crates/pert-ui-runtime/src/select.rs:1740 (rustdoc summary) |
| `pert_ui_runtime::InlineSlider` | widget/builder | crates/pert-ui-runtime/src/numeric_controls.rs:1592 (rustdoc summary) |
| `pert_ui_runtime::InlineText` | widget/builder | crates/pert-ui-runtime/src/text_fields/inline_text.rs:66 (rustdoc summary) |
| `pert_ui_runtime::InlineToggle` | widget/builder | crates/pert-ui-runtime/src/inline_controls.rs:54 (rustdoc summary) |
| `pert_ui_runtime::InputOutcome` | outcome | crates/pert-ui-runtime/src/pointer.rs:216 (whether an input callback consumed an event (handled flag)) |
| `pert_ui_runtime::InputRejection` | error | crates/pert-ui-runtime/src/text_fields/normalize.rs:24 (normalizer refusal (implements Error)) |
| `pert_ui_runtime::InspectValue` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:71 (rustdoc summary) |
| `pert_ui_runtime::InspectionClient` | handle | crates/pert-ui-runtime/src/inspection.rs:721 (Send request handle) |
| `pert_ui_runtime::InspectionQuery` | plain data | crates/pert-ui-runtime/src/inspection.rs:200 (request description) |
| `pert_ui_runtime::InspectionReply` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/inspection.rs:448 (one answer) |
| `pert_ui_runtime::InspectionService` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/inspection.rs:1053 (UI-thread service end) |
| `pert_ui_runtime::InspectionSnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:849 (versioned owned snapshot) |
| `pert_ui_runtime::IntegerInput` | widget/builder | crates/pert-ui-runtime/src/text_fields/number_inputs.rs:166 (rustdoc summary) |
| `pert_ui_runtime::IntervalTimer` | handle | crates/pert-ui-runtime/src/runtime/application.rs:84 (timer handle (cancel), drives no widget) |
| `pert_ui_runtime::IntoElement` | trait | crates/pert-ui-runtime/src/layout.rs:365 (rustdoc summary) |
| `pert_ui_runtime::Invalidation` | plain data | crates/pert-ui-runtime/src/runtime.rs:41 (rustdoc summary) |
| `pert_ui_runtime::Justify` | plain data | crates/pert-ui-runtime/src/containers.rs:393 (rustdoc summary) |
| `pert_ui_runtime::KeyAction` | plain data | crates/pert-ui-runtime/src/input.rs:206 (rustdoc summary) |
| `pert_ui_runtime::KeyChord` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:27 (key + modifiers) |
| `pert_ui_runtime::KeyCode` | plain data | crates/pert-ui-runtime/src/input.rs:11 (rustdoc summary) |
| `pert_ui_runtime::KeyDispatch` | plain data | crates/pert-ui-runtime/src/runtime/commands.rs:452 (routing result) |
| `pert_ui_runtime::KeyEvent` | plain data | crates/pert-ui-runtime/src/input.rs:241 (physical key event) |
| `pert_ui_runtime::KeyTransition` | plain data | crates/pert-ui-runtime/src/input.rs:220 (rustdoc summary) |
| `pert_ui_runtime::KeyboardState` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/input.rs:352 (pressed-key state captured from the runtime) |
| `pert_ui_runtime::Knob` | widget/builder | crates/pert-ui-runtime/src/numeric_controls.rs:1598 (rustdoc summary) |
| `pert_ui_runtime::Label` | widget/builder | crates/pert-ui-runtime/src/controls.rs:58 (rustdoc summary) |
| `pert_ui_runtime::LayoutError` | error | crates/pert-ui-runtime/src/layout.rs:10 (rustdoc summary) |
| `pert_ui_runtime::Length` | value with an invariant | crates/pert-ui-runtime/src/containers.rs:12 (validated logical-pixel extent) |
| `pert_ui_runtime::LevelMeter` | widget/builder | crates/pert-ui-runtime/src/live_widgets.rs:131 (rustdoc summary) |
| `pert_ui_runtime::LimitError` | error | crates/pert-ui-runtime/src/render_limits.rs:46 (rustdoc summary) |
| `pert_ui_runtime::LineCap` | plain data | crates/pert-ui-runtime/src/geometry.rs:47 (rustdoc summary) |
| `pert_ui_runtime::LineJoin` | plain data | crates/pert-ui-runtime/src/geometry.rs:63 (rustdoc summary) |
| `pert_ui_runtime::Link` | widget/builder | crates/pert-ui-runtime/src/link.rs:76 (rustdoc summary) |
| `pert_ui_runtime::LinkTarget` | plain data | crates/pert-ui-runtime/src/file_browser.rs:42 (rustdoc summary) |
| `pert_ui_runtime::LinkUnderline` | plain data | crates/pert-ui-runtime/src/link.rs:11 (rustdoc summary) |
| `pert_ui_runtime::ListController` | controller | crates/pert-ui-runtime/src/virtual_list.rs:178 (rustdoc summary) |
| `pert_ui_runtime::ListDragItems` | plain data | crates/pert-ui-runtime/src/list_drag.rs:20 (dragged keys) |
| `pert_ui_runtime::ListDrop` | plain data | crates/pert-ui-runtime/src/list_drag.rs:64 (drop handed to a callback) |
| `pert_ui_runtime::ListDropMark` | plain data | crates/pert-ui-runtime/src/virtual_list.rs:592 (rustdoc summary) |
| `pert_ui_runtime::ListDropPlacement` | plain data | crates/pert-ui-runtime/src/virtual_list.rs:562 (rustdoc summary) |
| `pert_ui_runtime::ListDropPoint` | plain data | crates/pert-ui-runtime/src/virtual_list.rs:524 (rustdoc summary) |
| `pert_ui_runtime::ListFilter` | handle | crates/pert-ui-runtime/src/collection_filter.rs:255 (derived filtered row source (no widget addressed)) |
| `pert_ui_runtime::ListItems` | trait | crates/pert-ui-runtime/src/virtual_list.rs:39 (rustdoc summary) |
| `pert_ui_runtime::ListRevealReceipt` | receipt | crates/pert-ui-runtime/src/virtual_list.rs:229 (status+status_binding) |
| `pert_ui_runtime::ListRevealStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/virtual_list.rs:200 (status value) |
| `pert_ui_runtime::ListSelection` | handle | crates/pert-ui-runtime/src/list_selection.rs:62 (application-owned selection state (contract: selection stays application-owned, by key)) |
| `pert_ui_runtime::ListSelectionChange` | plain data | crates/pert-ui-runtime/src/list_selection.rs:86 (notification) |
| `pert_ui_runtime::ListSource` | handle | crates/pert-ui-runtime/src/virtual_list.rs:79 (shared row-set handle) |
| `pert_ui_runtime::ListingDelivery` | outcome | crates/pert-ui-runtime/src/file_browser.rs:249 (what deliver did with a response (under another suffix)) |
| `pert_ui_runtime::ListingError` | error | crates/pert-ui-runtime/src/file_browser.rs:77 (rustdoc summary) |
| `pert_ui_runtime::ListingErrorKind` | plain data | crates/pert-ui-runtime/src/file_browser.rs:58 (rustdoc summary) |
| `pert_ui_runtime::LiveWaker` | handle | crates/pert-ui-runtime/src/live.rs:144 (Send wake handle) |
| `pert_ui_runtime::Magnification` | value with an invariant | crates/pert-ui-runtime/src/pointer.rs:106 (validated factor) |
| `pert_ui_runtime::Mailbox` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/mailbox.rs:249 (UI-thread channel end) |
| `pert_ui_runtime::MaskId` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:1096 (scene-local index) |
| `pert_ui_runtime::MaskImage` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:1115 (validated coverage buffer) |
| `pert_ui_runtime::MeasureContext` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/layout.rs:706 (layout context) |
| `pert_ui_runtime::Menu` | widget/builder | crates/pert-ui-runtime/src/menu.rs:675 (rustdoc summary) |
| `pert_ui_runtime::MenuBar` | widget/builder | crates/pert-ui-runtime/src/menu_bar.rs:624 (rustdoc summary) |
| `pert_ui_runtime::MenuItem` | widget/builder | crates/pert-ui-runtime/src/menu.rs:222 (row description) |
| `pert_ui_runtime::MeterAxis` | plain data | crates/pert-ui-runtime/src/live_widgets.rs:18 (rustdoc summary) |
| `pert_ui_runtime::MetricRole` | plain data | crates/pert-ui-runtime/src/runtime/theme.rs:74 (role key) |
| `pert_ui_runtime::Modifiers` | plain data | crates/pert-ui-runtime/src/input.rs:285 (modifier flags, public fields) |
| `pert_ui_runtime::MotionPreference` | plain data | crates/pert-ui-runtime/src/runtime/settings.rs:120 (rustdoc summary) |
| `pert_ui_runtime::MultiSelect` | widget/builder | crates/pert-ui-runtime/src/multi_select.rs:1209 (rustdoc summary) |
| `pert_ui_runtime::MultiSelectCommit` | plain data | crates/pert-ui-runtime/src/multi_select.rs:20 (rustdoc summary) |
| `pert_ui_runtime::NativeWindowId` | value with an invariant | crates/pert-ui-runtime/src/runtime/native_windows.rs:131 (runtime identity) |
| `pert_ui_runtime::NativeWindowOptions` | value with an invariant | crates/pert-ui-runtime/src/runtime/native_windows.rs:186 (validated options) |
| `pert_ui_runtime::NavigationHistory` | handle | crates/pert-ui-runtime/src/navigation_history.rs:194 (application-owned history model shared by history widgets) |
| `pert_ui_runtime::Notifier` | handle | crates/pert-ui-runtime/src/runtime/notify.rs:242 (scheduling handle for a component's own delivery) |
| `pert_ui_runtime::NumberFormat` | plain data | crates/pert-ui-runtime/src/number_format.rs:38 (presentation/parsing policy (closures)) |
| `pert_ui_runtime::NumberInputError` | error | crates/pert-ui-runtime/src/text_fields/number_inputs.rs:14 (rustdoc summary) |
| `pert_ui_runtime::NumberLocale` | value with an invariant | crates/pert-ui-runtime/src/number_format.rs:191 (validated separators) |
| `pert_ui_runtime::NumberLocaleError` | error | crates/pert-ui-runtime/src/number_format.rs:147 (rustdoc summary) |
| `pert_ui_runtime::OpaqueColor` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:667 (alpha exactly one) |
| `pert_ui_runtime::Orientation` | plain data | crates/pert-ui-runtime/src/numeric_controls.rs:20 (rustdoc summary) |
| `pert_ui_runtime::OverlapPolicy` | plain data | crates/pert-ui-runtime/src/validation.rs:11 (rustdoc summary) |
| `pert_ui_runtime::Painter` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/drawing.rs:1307 (paint context) |
| `pert_ui_runtime::Palette` | value with an invariant | crates/pert-ui-runtime/src/runtime/theme.rs:742 (validated palette) |
| `pert_ui_runtime::Parameter` | handle | crates/pert-ui-runtime/src/parameter_binding.rs:283 (UI-local parameter binding) |
| `pert_ui_runtime::ParameterHost` | handle | crates/pert-ui-runtime/src/parameter_binding.rs:495 (authoritative local owner) |
| `pert_ui_runtime::ParameterInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:718 (captured observation) |
| `pert_ui_runtime::Path` | value with an invariant | crates/pert-ui-runtime/src/geometry.rs:199 (immutable validated outline) |
| `pert_ui_runtime::PathBuilder` | widget/builder | crates/pert-ui-runtime/src/geometry.rs:382 (path builder) |
| `pert_ui_runtime::PathElement` | plain data | crates/pert-ui-runtime/src/geometry.rs:158 (validated path operation (enum)) |
| `pert_ui_runtime::PathError` | error | crates/pert-ui-runtime/src/geometry.rs:12 (rustdoc summary) |
| `pert_ui_runtime::Pending` | handle | crates/pert-ui-runtime/src/inspection.rs:565 (cross-thread pending reply (wait/try_reply)) |
| `pert_ui_runtime::PendingJump` | receipt role under another suffix | crates/pert-ui-runtime/src/content_search.rs:416 (status()+status_binding() of a queued row reveal under another suffix) |
| `pert_ui_runtime::Permission` | plain data | crates/pert-ui-runtime/src/permission.rs:43 (capability value (fixed or bound)) |
| `pert_ui_runtime::PhysicalExtent` | plain data | crates/pert-ui-runtime/src/drawing.rs:860 (physical pixel extent) |
| `pert_ui_runtime::PickChoice` | plain data | crates/pert-ui-runtime/src/quick_pick.rs:361 (application decision) |
| `pert_ui_runtime::PickItem` | plain data | crates/pert-ui-runtime/src/quick_pick.rs:33 (row description) |
| `pert_ui_runtime::PickMatcher` | plain data | crates/pert-ui-runtime/src/quick_pick.rs:116 (replaceable matching policy) |
| `pert_ui_runtime::PickPending` | handle | crates/pert-ui-runtime/src/quick_pick.rs:319 (application-finished result adapter) |
| `pert_ui_runtime::PickStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/quick_pick.rs:281 (status value) |
| `pert_ui_runtime::Point` | value with an invariant | crates/pert-ui-runtime/src/pointer.rs:4 (finite coordinates) |
| `pert_ui_runtime::PointerButton` | plain data | crates/pert-ui-runtime/src/pointer.rs:40 (rustdoc summary) |
| `pert_ui_runtime::PointerEvent` | plain data | crates/pert-ui-runtime/src/pointer.rs:140 (owned pointer event) |
| `pert_ui_runtime::PointerKind` | plain data | crates/pert-ui-runtime/src/pointer.rs:62 (rustdoc summary) |
| `pert_ui_runtime::Popover` | widget/builder | crates/pert-ui-runtime/src/popover.rs:193 (rustdoc summary) |
| `pert_ui_runtime::PreparedDetach` | handle | crates/pert-ui-runtime/src/runtime/native_windows.rs:245 (validated pending movement) |
| `pert_ui_runtime::PresentationAction` | plain data | crates/pert-ui-runtime/src/runtime/native_windows.rs:984 (rustdoc summary) |
| `pert_ui_runtime::PresentationRate` | value with an invariant | crates/pert-ui-runtime/src/runtime/presentation.rs:10 (validated hertz) |
| `pert_ui_runtime::PresentationReceipt` | receipt | crates/pert-ui-runtime/src/runtime/native_windows.rs:950 (status+status_binding) |
| `pert_ui_runtime::PresentationRequest` | handle | crates/pert-ui-runtime/src/runtime/native_windows.rs:1015 (queued host operation (drop cancels)) |
| `pert_ui_runtime::PresentationStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime/native_windows.rs:919 (status value) |
| `pert_ui_runtime::PreviewEvent` | plain data | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:156 (accepted preview record) |
| `pert_ui_runtime::ProfileOperation` | plain data | crates/pert-ui-runtime/src/profiling.rs:14 (rustdoc summary) |
| `pert_ui_runtime::ProfileSample` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/profiling.rs:114 (captured timing) |
| `pert_ui_runtime::ProfileSnapshot` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/profiling.rs:137 (owned profiling data) |
| `pert_ui_runtime::ProfileStats` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/profiling.rs:89 (captured aggregate) |
| `pert_ui_runtime::ProgressBar` | widget/builder | crates/pert-ui-runtime/src/progress.rs:143 (rustdoc summary) |
| `pert_ui_runtime::ProgressStatus` | plain data | crates/pert-ui-runtime/src/progress.rs:22 (application-reported value) |
| `pert_ui_runtime::Property` | widget/builder | crates/pert-ui-runtime/src/property_grid.rs:35 (row description) |
| `pert_ui_runtime::PropertyGrid` | widget/builder | crates/pert-ui-runtime/src/property_grid.rs:222 (rustdoc summary) |
| `pert_ui_runtime::QuarterTurn` | plain data | crates/pert-ui-runtime/src/transform.rs:172 (rustdoc summary) |
| `pert_ui_runtime::QueryResult` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/inspection.rs:364 (query answer) |
| `pert_ui_runtime::QuickPick` | widget/builder | crates/pert-ui-runtime/src/quick_pick.rs:866 (rustdoc summary) |
| `pert_ui_runtime::QuickPickController` | controller | crates/pert-ui-runtime/src/quick_pick.rs:425 (rustdoc summary) |
| `pert_ui_runtime::RadioGroup` | widget/builder | crates/pert-ui-runtime/src/choice_controls.rs:697 (rustdoc summary) |
| `pert_ui_runtime::RasterImage` | value with an invariant | crates/pert-ui-runtime/src/resources.rs:147 (validated decoded image) |
| `pert_ui_runtime::ReadSignal` | handle | crates/pert-ui-runtime/src/signal.rs:192 (read-only signal view) |
| `pert_ui_runtime::Rect` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:53 (finite rectangle) |
| `pert_ui_runtime::Redescribed` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/conformance.rs:76 (conformance result) |
| `pert_ui_runtime::RedescriptionError` | error | crates/pert-ui-runtime/src/conformance.rs:14 (rustdoc summary) |
| `pert_ui_runtime::RegionEdge` | plain data | crates/pert-ui-runtime/src/region_layout.rs:64 (rustdoc summary) |
| `pert_ui_runtime::RegionLayout` | widget/builder | crates/pert-ui-runtime/src/region_layout.rs:43 (rustdoc summary) |
| `pert_ui_runtime::RemoteCommit` | plain data | crates/pert-ui-runtime/src/parameter_binding/remote.rs:12 (immutable proposal) |
| `pert_ui_runtime::RemoteHost` | handle | crates/pert-ui-runtime/src/parameter_binding/remote.rs:421 (external-authority adapter owner) |
| `pert_ui_runtime::RequestError` | error | crates/pert-ui-runtime/src/inspection.rs:413 (rustdoc summary) |
| `pert_ui_runtime::RequestTicket` | plain data | crates/pert-ui-runtime/src/async_source.rs:28 (issued ticket) |
| `pert_ui_runtime::RequestTracker` | handle | crates/pert-ui-runtime/src/async_source.rs:108 (issues tickets (not Clone)) |
| `pert_ui_runtime::ResourceKind` | plain data | crates/pert-ui-runtime/src/render_limits.rs:7 (rustdoc summary) |
| `pert_ui_runtime::ResponseDisposition` | outcome | crates/pert-ui-runtime/src/parameter_binding/remote.rs:114 (whether a response changed state (under another suffix)) |
| `pert_ui_runtime::RestoreRejection` | error | crates/pert-ui-runtime/src/workspace.rs:677 (participant refusal (implements Error)) |
| `pert_ui_runtime::RevealOutcome` | outcome | crates/pert-ui-runtime/src/runtime/reveal.rs:64 (terminal result of a reveal) |
| `pert_ui_runtime::RevealReceipt` | receipt | crates/pert-ui-runtime/src/runtime/reveal.rs:132 (status+status_binding) |
| `pert_ui_runtime::RevealRefusal` | plain data | crates/pert-ui-runtime/src/runtime/reveal.rs:33 (reason) |
| `pert_ui_runtime::RevealStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime/reveal.rs:110 (status value) |
| `pert_ui_runtime::RichText` | plain data | crates/pert-ui-runtime/src/text/rich.rs:87 (styled text) |
| `pert_ui_runtime::Row` | widget/builder | crates/pert-ui-runtime/src/containers.rs:893 (rustdoc summary) |
| `pert_ui_runtime::Runtime` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/runtime.rs:3508 (runtime) |
| `pert_ui_runtime::RuntimeError` | error | crates/pert-ui-runtime/src/runtime.rs:184 (rustdoc summary) |
| `pert_ui_runtime::ScaleLimits` | value with an invariant | crates/pert-ui-runtime/src/zoom_view.rs:14 (validated limits) |
| `pert_ui_runtime::ScrollArea` | widget/builder | crates/pert-ui-runtime/src/views.rs:329 (rustdoc summary) |
| `pert_ui_runtime::ScrollAxes` | plain data | crates/pert-ui-runtime/src/views.rs:294 (rustdoc summary) |
| `pert_ui_runtime::SearchField` | widget/builder | crates/pert-ui-runtime/src/text_fields/search.rs:107 (rustdoc summary) |
| `pert_ui_runtime::SearchKey` | plain data | crates/pert-ui-runtime/src/text_fields/search.rs:16 (rustdoc summary) |
| `pert_ui_runtime::Section` | widget/builder | crates/pert-ui-runtime/src/collapsible.rs:809 (description consumed by Accordion) |
| `pert_ui_runtime::SegmentedControl` | widget/builder | crates/pert-ui-runtime/src/choice_controls.rs:1441 (rustdoc summary) |
| `pert_ui_runtime::Select` | widget/builder | crates/pert-ui-runtime/src/select.rs:1362 (rustdoc summary) |
| `pert_ui_runtime::SelectableText` | widget/builder | crates/pert-ui-runtime/src/selectable_text.rs:108 (rustdoc summary) |
| `pert_ui_runtime::SelectionChange` | plain data | crates/pert-ui-runtime/src/runtime/notify.rs:181 (notification) |
| `pert_ui_runtime::SelectionMode` | plain data | crates/pert-ui-runtime/src/list_selection.rs:11 (rustdoc summary) |
| `pert_ui_runtime::Selector` | plain data | crates/pert-ui-runtime/src/inspection.rs:60 (query) |
| `pert_ui_runtime::SemanticAction` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:299 (rustdoc summary) |
| `pert_ui_runtime::SemanticItem` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:350 (inspection row builder) |
| `pert_ui_runtime::SemanticRole` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:162 (rustdoc summary) |
| `pert_ui_runtime::SemanticValue` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:256 (rustdoc summary) |
| `pert_ui_runtime::Semantics` | plain data | crates/pert-ui-runtime/src/diagnostics.rs:464 (inspection data builder) |
| `pert_ui_runtime::SendError` | error | crates/pert-ui-runtime/src/mailbox.rs:25 (rustdoc summary) |
| `pert_ui_runtime::SeriesSource` | handle | crates/pert-ui-runtime/src/live.rs:627 (UI-thread series source) |
| `pert_ui_runtime::Shape` | plain data | crates/pert-ui-runtime/src/shapes.rs:17 (rustdoc summary) |
| `pert_ui_runtime::ShortcutBinding` | plain data | crates/pert-ui-runtime/src/validation.rs:44 (declaration) |
| `pert_ui_runtime::ShortcutScope` | plain data | crates/pert-ui-runtime/src/validation.rs:23 (rustdoc summary) |
| `pert_ui_runtime::Show` | widget/builder | crates/pert-ui-runtime/src/described.rs:223 (rustdoc summary) |
| `pert_ui_runtime::Signal` | handle | crates/pert-ui-runtime/src/signal.rs:177 (mutable UI-local state cell) |
| `pert_ui_runtime::SinkError` | error | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:89 (rustdoc summary) |
| `pert_ui_runtime::SinkErrorKind` | plain data | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:53 (rustdoc summary) |
| `pert_ui_runtime::Size` | value with an invariant | crates/pert-ui-runtime/src/layout.rs:119 (finite nonnegative size) |
| `pert_ui_runtime::SizeLimits` | value with an invariant | crates/pert-ui-runtime/src/layout.rs:177 (validated limits) |
| `pert_ui_runtime::Slider` | widget/builder | crates/pert-ui-runtime/src/numeric_controls.rs:1586 (rustdoc summary) |
| `pert_ui_runtime::SortDirection` | plain data | crates/pert-ui-runtime/src/table.rs:212 (rustdoc summary) |
| `pert_ui_runtime::SourceInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/live.rs:281 (captured observation) |
| `pert_ui_runtime::Spacing` | plain data | crates/pert-ui-runtime/src/runtime/theme.rs:313 (rustdoc summary) |
| `pert_ui_runtime::Spinner` | widget/builder | crates/pert-ui-runtime/src/progress.rs:348 (rustdoc summary) |
| `pert_ui_runtime::SplitAxis` | plain data | crates/pert-ui-runtime/src/split.rs:8 (rustdoc summary) |
| `pert_ui_runtime::SplitPane` | widget/builder | crates/pert-ui-runtime/src/split.rs:198 (rustdoc summary) |
| `pert_ui_runtime::SplitRule` | plain data | crates/pert-ui-runtime/src/split.rs:141 (rustdoc summary) |
| `pert_ui_runtime::SplitSizing` | value with an invariant | crates/pert-ui-runtime/src/split.rs:76 (validated sizing) |
| `pert_ui_runtime::StateOwner` | trait | crates/pert-ui-runtime/src/signal.rs:64 (rustdoc summary) |
| `pert_ui_runtime::StringRole` | plain data | crates/pert-ui-runtime/src/strings.rs:20 (role key) |
| `pert_ui_runtime::StripeGroup` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:84 (rustdoc summary) |
| `pert_ui_runtime::StrokeStyle` | value with an invariant | crates/pert-ui-runtime/src/geometry.rs:94 (validated stroke) |
| `pert_ui_runtime::SubmissionDelivery` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/parameter_binding/remote.rs:132 (status value) |
| `pert_ui_runtime::SurfaceDismissal` | plain data | crates/pert-ui-runtime/src/popover.rs:50 (reason) |
| `pert_ui_runtime::SurfaceKind` | plain data | crates/pert-ui-runtime/src/popover.rs:8 (rustdoc summary) |
| `pert_ui_runtime::Tab` | widget/builder | crates/pert-ui-runtime/src/tabs.rs:12 (description consumed by Tabs) |
| `pert_ui_runtime::Table` | widget/builder | crates/pert-ui-runtime/src/table.rs:1329 (rustdoc summary) |
| `pert_ui_runtime::TableColumn` | widget/builder | crates/pert-ui-runtime/src/table.rs:34 (column description) |
| `pert_ui_runtime::TableController` | controller | crates/pert-ui-runtime/src/table.rs:561 (rustdoc summary) |
| `pert_ui_runtime::TableItems` | trait | crates/pert-ui-runtime/src/table.rs:236 (rustdoc summary) |
| `pert_ui_runtime::TableSort` | plain data | crates/pert-ui-runtime/src/table.rs:221 (rustdoc summary) |
| `pert_ui_runtime::TableSource` | handle | crates/pert-ui-runtime/src/table.rs:278 (shared row-set handle) |
| `pert_ui_runtime::Tabs` | widget/builder | crates/pert-ui-runtime/src/tabs.rs:263 (rustdoc summary) |
| `pert_ui_runtime::TaskReceipt` | receipt | crates/pert-ui-runtime/src/runtime/tasks.rs:28 (status+status_binding) |
| `pert_ui_runtime::TaskStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/runtime/tasks.rs:6 (status value) |
| `pert_ui_runtime::Telemetry` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/live.rs:79 (what a component reads from a source) |
| `pert_ui_runtime::TerminalEvent` | plain data | crates/pert-ui-runtime/src/parameter_binding/delivery.rs:130 (terminal record) |
| `pert_ui_runtime::TextArea` | widget/builder | crates/pert-ui-runtime/src/text_fields.rs:203 (rustdoc summary) |
| `pert_ui_runtime::TextColor` | plain data | crates/pert-ui-runtime/src/text/rich.rs:6 (rustdoc summary) |
| `pert_ui_runtime::TextCommand` | plain data | crates/pert-ui-runtime/src/text_input.rs:111 (rustdoc summary) |
| `pert_ui_runtime::TextCommandOutcome` | outcome | crates/pert-ui-runtime/src/text_input.rs:143 (outcome of a delivered TextCommand (handled flag)) |
| `pert_ui_runtime::TextEditError` | error | crates/pert-ui-runtime/src/text_input.rs:11 (rustdoc summary) |
| `pert_ui_runtime::TextError` | error | crates/pert-ui-runtime/src/text.rs:54 (rustdoc summary) |
| `pert_ui_runtime::TextField` | widget/builder | crates/pert-ui-runtime/src/text_fields.rs:154 (rustdoc summary) |
| `pert_ui_runtime::TextInputState` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/text_input.rs:217 (IME anchor data) |
| `pert_ui_runtime::TextKeyOutcome` | outcome | crates/pert-ui-runtime/src/runtime/commands.rs:327 (text-editing phase result (handled flag)) |
| `pert_ui_runtime::TextLayout` | plain data | crates/pert-ui-runtime/src/text.rs:176 (measured text) |
| `pert_ui_runtime::TextLimit` | plain data | crates/pert-ui-runtime/src/text.rs:94 (rustdoc summary) |
| `pert_ui_runtime::TextMovement` | plain data | crates/pert-ui-runtime/src/text_selection.rs:8 (rustdoc summary) |
| `pert_ui_runtime::TextOverflow` | plain data | crates/pert-ui-runtime/src/text/fit.rs:39 (rustdoc summary) |
| `pert_ui_runtime::TextPreedit` | value with an invariant | crates/pert-ui-runtime/src/text_input.rs:50 (validated preedit) |
| `pert_ui_runtime::TextRenderingMode` | plain data | crates/pert-ui-runtime/src/runtime/settings.rs:133 (rustdoc summary) |
| `pert_ui_runtime::TextSelection` | value with an invariant | crates/pert-ui-runtime/src/text_selection.rs:39 (selection offsets) |
| `pert_ui_runtime::TextSource` | plain data | crates/pert-ui-runtime/src/text_source.rs:60 (text value (fixed, bound or role)) |
| `pert_ui_runtime::TextSpan` | plain data | crates/pert-ui-runtime/src/text/rich.rs:25 (styled segment) |
| `pert_ui_runtime::TextStyle` | plain data | crates/pert-ui-runtime/src/text.rs:123 (rustdoc summary) |
| `pert_ui_runtime::ThemeManager` | handle | crates/pert-ui-runtime/src/runtime/theme.rs:1014 (runtime-wide theme handle) |
| `pert_ui_runtime::ThemeMetrics` | value with an invariant | crates/pert-ui-runtime/src/runtime/theme.rs:244 (validated measurements) |
| `pert_ui_runtime::ThemeMode` | plain data | crates/pert-ui-runtime/src/runtime/theme.rs:14 (rustdoc summary) |
| `pert_ui_runtime::TicketStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/async_source.rs:58 (status value) |
| `pert_ui_runtime::Toast` | widget/builder | crates/pert-ui-runtime/src/notifications.rs:144 (toast description) |
| `pert_ui_runtime::ToastHost` | widget/builder | crates/pert-ui-runtime/src/notifications.rs:398 (rustdoc summary) |
| `pert_ui_runtime::ToastQueue` | controller role under another suffix | crates/pert-ui-runtime/src/notifications.rs:262 (cloneable UI-local sender whose single ToastHost presents it) |
| `pert_ui_runtime::ToastReceipt` | receipt | crates/pert-ui-runtime/src/notifications.rs:196 (status+status_binding) |
| `pert_ui_runtime::ToastStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/notifications.rs:169 (status value) |
| `pert_ui_runtime::Toggle` | widget/builder | crates/pert-ui-runtime/src/choice_controls.rs:652 (rustdoc summary) |
| `pert_ui_runtime::ToolEdge` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:20 (rustdoc summary) |
| `pert_ui_runtime::ToolGroup` | widget/builder | crates/pert-ui-runtime/src/toolbar.rs:47 (group description) |
| `pert_ui_runtime::ToolWindow` | widget/builder | crates/pert-ui-runtime/src/tool_windows/mod.rs:303 (tool window description) |
| `pert_ui_runtime::ToolWindowChange` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:218 (notification) |
| `pert_ui_runtime::ToolWindowChangeKind` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:191 (rustdoc summary) |
| `pert_ui_runtime::ToolWindowController` | controller | crates/pert-ui-runtime/src/tool_windows/mod.rs:2279 (rustdoc summary) |
| `pert_ui_runtime::ToolWindowLayout` | widget/builder | crates/pert-ui-runtime/src/tool_windows/mod.rs:749 (rustdoc summary) |
| `pert_ui_runtime::ToolWindowMode` | plain data | crates/pert-ui-runtime/src/tool_windows/mod.rs:158 (rustdoc summary) |
| `pert_ui_runtime::Toolbar` | widget/builder | crates/pert-ui-runtime/src/toolbar.rs:327 (rustdoc summary) |
| `pert_ui_runtime::ToolbarOverflow` | plain data | crates/pert-ui-runtime/src/toolbar.rs:15 (rustdoc summary) |
| `pert_ui_runtime::Tooltip` | widget/builder | crates/pert-ui-runtime/src/popover.rs:404 (rustdoc summary) |
| `pert_ui_runtime::Trace` | widget/builder | crates/pert-ui-runtime/src/live_widgets.rs:481 (rustdoc summary) |
| `pert_ui_runtime::Track` | value with an invariant | crates/pert-ui-runtime/src/containers.rs:944 (validated sizing rule) |
| `pert_ui_runtime::Transform2D` | value with an invariant | crates/pert-ui-runtime/src/transform.rs:31 (validated invertible map) |
| `pert_ui_runtime::TranslucentColorError` | error | crates/pert-ui-runtime/src/drawing.rs:742 (rustdoc summary) |
| `pert_ui_runtime::Tree` | widget/builder | crates/pert-ui-runtime/src/tree.rs:901 (rustdoc summary) |
| `pert_ui_runtime::TreeController` | controller | crates/pert-ui-runtime/src/tree.rs:258 (rustdoc summary) |
| `pert_ui_runtime::TreeItems` | trait | crates/pert-ui-runtime/src/tree.rs:25 (rustdoc summary) |
| `pert_ui_runtime::TreeSource` | handle | crates/pert-ui-runtime/src/tree.rs:81 (shared hierarchy handle) |
| `pert_ui_runtime::UiMessage` | plain data | crates/pert-ui-runtime/src/strings.rs:663 (rustdoc summary) |
| `pert_ui_runtime::UiSender` | handle | crates/pert-ui-runtime/src/mailbox.rs:88 (Send channel handle) |
| `pert_ui_runtime::UiSettings` | handle | crates/pert-ui-runtime/src/runtime/settings.rs:213 (runtime-wide settings handle) |
| `pert_ui_runtime::UiStringError` | error | crates/pert-ui-runtime/src/strings.rs:510 (rustdoc summary) |
| `pert_ui_runtime::UiStrings` | value with an invariant | crates/pert-ui-runtime/src/strings.rs:558 (validated string table) |
| `pert_ui_runtime::UiZoom` | value with an invariant | crates/pert-ui-runtime/src/runtime/settings.rs:22 (validated zoom) |
| `pert_ui_runtime::UpdateContext` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/runtime.rs:2551 (update context) |
| `pert_ui_runtime::ValidationCode` | plain data | crates/pert-ui-runtime/src/validation.rs:114 (rustdoc summary) |
| `pert_ui_runtime::ValidationHints` | plain data | crates/pert-ui-runtime/src/validation.rs:89 (rustdoc summary) |
| `pert_ui_runtime::ValidationIssue` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/validation.rs:174 (captured issue) |
| `pert_ui_runtime::ValidationMarker` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/validation.rs:191 (captured marker) |
| `pert_ui_runtime::ValidationReport` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/validation.rs:220 (latest results) |
| `pert_ui_runtime::VectorGeometry` | plain data | crates/pert-ui-runtime/src/vector_scene.rs:24 (shared vector data) |
| `pert_ui_runtime::VectorId` | value with an invariant | crates/pert-ui-runtime/src/vector_scene.rs:7 (scene-local index) |
| `pert_ui_runtime::VectorRasterizer` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-runtime/src/vector_scene.rs:112 (CPU vector fallback) |
| `pert_ui_runtime::VectorStyle` | plain data | crates/pert-ui-runtime/src/vector_scene.rs:16 (rustdoc summary) |
| `pert_ui_runtime::View` | widget/builder | crates/pert-ui-runtime/src/views.rs:14 (rustdoc summary) |
| `pert_ui_runtime::ViewScale` | value with an invariant | crates/pert-ui-runtime/src/transform.rs:267 (validated scale) |
| `pert_ui_runtime::ViewState` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/workspace.rs:226 (state one participant captured) |
| `pert_ui_runtime::ViewTransform` | value with an invariant | crates/pert-ui-runtime/src/transform.rs:334 (scale + offset) |
| `pert_ui_runtime::Viewport` | value with an invariant | crates/pert-ui-runtime/src/drawing.rs:830 (validated viewport) |
| `pert_ui_runtime::VirtualAxis` | value with an invariant | crates/pert-ui-runtime/src/virtual_axis.rs:123 (layout model value with validated constructors) |
| `pert_ui_runtime::VirtualList` | widget/builder | crates/pert-ui-runtime/src/virtual_list.rs:439 (rustdoc summary) |
| `pert_ui_runtime::WaitError` | error | crates/pert-ui-runtime/src/inspection.rs:431 (rustdoc summary) |
| `pert_ui_runtime::WakeError` | error | crates/pert-ui-runtime/src/live.rs:105 (rustdoc summary) |
| `pert_ui_runtime::WheelGesture` | plain data | crates/pert-ui-runtime/src/zoom_view.rs:67 (rustdoc summary) |
| `pert_ui_runtime::WidgetId` | value with an invariant | crates/pert-ui-runtime/src/runtime.rs:121 (runtime identity) |
| `pert_ui_runtime::WindowCloseAction` | plain data | crates/pert-ui-runtime/src/windows.rs:156 (close decision) |
| `pert_ui_runtime::WindowController` | controller | crates/pert-ui-runtime/src/windows.rs:41 (rustdoc summary) |
| `pert_ui_runtime::WindowError` | error | crates/pert-ui-runtime/src/runtime/native_windows.rs:870 (rustdoc summary) |
| `pert_ui_runtime::WindowInspection` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/diagnostics.rs:961 (captured observation) |
| `pert_ui_runtime::WindowLayer` | widget/builder | crates/pert-ui-runtime/src/windows.rs:650 (rustdoc summary) |
| `pert_ui_runtime::WindowPlacement` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/windows.rs:25 (committed placement as controller/workspace keep it) |
| `pert_ui_runtime::WindowStatus` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/windows.rs:11 (status value) |
| `pert_ui_runtime::Workspace` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/workspace.rs:488 (captured workspace document) |
| `pert_ui_runtime::WorkspaceError` | error | crates/pert-ui-runtime/src/workspace.rs:579 (rustdoc summary) |
| `pert_ui_runtime::WorkspacePath` | value with an invariant | crates/pert-ui-runtime/src/workspace.rs:355 (validated address) |
| `pert_ui_runtime::WorkspaceReport` | state (passive data, snapshot or status value) | crates/pert-ui-runtime/src/workspace.rs:746 (what a restore did) |
| `pert_ui_runtime::WorkspaceSettings` | plain data | crates/pert-ui-runtime/src/workspace.rs:418 (presentation preferences) |
| `pert_ui_runtime::WorkspaceValue` | plain data | crates/pert-ui-runtime/src/workspace.rs:39 (workspace value) |
| `pert_ui_runtime::Wrap` | widget/builder | crates/pert-ui-runtime/src/containers.rs:936 (rustdoc summary) |
| `pert_ui_runtime::ZoomController` | controller | crates/pert-ui-runtime/src/zoom_view.rs:126 (rustdoc summary) |
| `pert_ui_runtime::ZoomView` | widget/builder | crates/pert-ui-runtime/src/zoom_view.rs:306 (rustdoc summary) |
| `pert_ui_wgpu::Frame` | outcome | crates/pert-ui-wgpu/src/lib.rs:92 (surface acquisition/presentation outcome (under another suffix)) |
| `pert_ui_wgpu::Notice` | plain data | crates/pert-ui-wgpu/src/lib.rs:80 (asynchronous notification) |
| `pert_ui_wgpu::RenderError` | error | crates/pert-ui-wgpu/src/lib.rs:46 (rustdoc summary) |
| `pert_ui_wgpu::Renderer` | other (context, runtime, service, renderer or state machine) | crates/pert-ui-wgpu/src/lib.rs:234 (renderer) |
| `pert_ui::App` | widget/builder | crates/pert-ui/src/app.rs:285 (application builder) |
| `pert_ui::AppError` | error | crates/pert-ui/src/app.rs:30 (rustdoc summary) |
| `pert_ui::AppShutdownError` | error | crates/pert-ui/src/app.rs:108 (rustdoc summary) |
| `pert_ui::ErrorAction` | plain data | crates/pert-ui/src/app.rs:184 (host decision) |
| `pert_ui::NativeCloseAction` | plain data | crates/pert-ui/src/app.rs:202 (host decision) |
| `pert_ui::Svg` | widget/builder | crates/pert-ui/src/svg/component.rs:42 (rustdoc summary) |
| `pert_ui::images::ImageError` | error | crates/pert-ui/src/images.rs:215 (rustdoc summary) |
| `pert_ui::images::ImageFormat` | plain data | crates/pert-ui/src/images.rs:87 (rustdoc summary) |
| `pert_ui::svg::SvgError` | error | crates/pert-ui/src/svg.rs:52 (rustdoc summary) |
