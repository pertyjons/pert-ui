# claude verification: G1

Source at HEAD (7c84f54); `git diff d4ba108 HEAD -- crates` is empty, so the
audited library source is the source checked. Throwaway probes ran against the
facade (`pert-ui`, `default-features = false`) in
`scratchpad/verify-g1-scratch/` with
`CARGO_TARGET_DIR=<repo>/target/audit-claude`; nothing in
the repository was modified.

## The other auditor's findings

| Other ID | Matched by | Verdict | Evidence | Remarks on severity, kind or fix |
|---|---|---|---|---|
| codex-C1-1 | none (my C1 was "no findings"; see dispute 1) | **confirmed** for RadioGroup, SegmentedControl, InlineChoice; Select, InlineSelect and MultiSelect are not affected (codex does not claim them) | `choice_controls.rs:939-943`: a different value binding sets `self.name = None` and `self.selected = None` in `reconcile`; `update` then returns at `validate(&choices)?` (`:1010`) before `self.name`/`self.selected` are set again (`:1038-1043`); inspection reads `self.selected` (`:1305-1308`), paint does too (`:1263`, `:1366`). Wrappers delegate (`:1508-1510`, `:1646-1648`). **Executed probe:** mount `X::bound(v1 = 2, [1 One, 2 Two])`, flush, `reconcile_children` with `X::bound(v2 = 2, [2 Two, 2 Dup])`, flush → `Err(Choice(DuplicateValue))`; for all three radio wrappers semantics go from `selected: Some(1), label: "Two", items [One, Two*]` to `selected: None, label: None, items [One, Two]`. Same probe without the binding change keeps the selection (as `component_bound_choices.rs:37-79` tests). Same probe for Select, InlineSelect and MultiSelect (with binding change) leaves semantics unchanged. | Rule correct (contracts.md:45-46 "An invalid reactive list leaves the last valid presentation in place"); kind project rule, behavior: correct. **Severity: low rather than medium.** The cleared state is visible only through inspection semantics: `Runtime::paint` refuses with `PendingUpdates` after the failed flush (observed in the probe), and the next successful update restores `selected` from the new binding. It needs a binding change and an invalid list in the same flush. Fix (keep the caches until the new reading validates; add the binding-change failure case for the three wrappers) is right and not breaking. |
| codex-C1-2 | none | **partly confirmed**: the signatures are as stated; a C1 project-rule departure is not established; the policy question holds for VirtualList only | `virtual_list.rs:635-641`, `:659-665`: `VirtualList::new`/`estimated` take `impl Into<ReadSignal<ListSource>>`. `tool_windows/mod.rs:765`, `:788-804`, `:807-809`: `ToolWindowLayout::new(center)`, `window`, `windows` add tool windows. Contracts "Choice sources" (contracts.md:40-47) speaks of *choice controls*; contracts.md:303 lists "Basic controls, choice controls and menu items" as separate families, and contracts.md:166-168 places VirtualList among controller-owned collections. | Codex already labels it a policy question; agreed for VirtualList (a selectable list whose only source is reactive, under the name `new`), but it is a scope clarification, not a departure. **ToolWindowLayout should be dropped:** its tool windows are content panels whose stripe buttons expand and collapse them (`tool_windows/mod.rs:601-605`); by that reading every keyed container (Tabs, Column) would be a choice family and `window` an "option-source builder". Severity low agreed. Fix: a one-sentence contract statement that collections and containers are outside "Choice sources" (non-breaking) is preferable to a constructor migration. |
| codex-C3-1 | claude-C3-1 | matched (all seven constructors) | Same constructors and lines: `choice_controls.rs:525` (+`:647`, `:652`), `menu.rs:299`, `:333`, `table.rs:574`, `tree.rs:272`, `file_browser/controller.rs:509`. | My report additionally raises the key-versus-state ordering of keyed items (`MenuItem::toggle`/`radio`) as a policy question and notes the sibling `InlineToggle::new(value, name)` (`inline_controls.rs:92`) already follows the rule. |
| codex-C4-1 | none | **confirmed** | `workspace.rs:541-543`: `Workspace::new` rejects an encoding over `MAX_DOCUMENT_BYTES`; `:551-554`: `set_settings` replaces settings unchecked; `encode` adds a `settings` line (`:811-823`); `decode` rejects text over the limit (`:865-868`). **Executed probe** (`src/bin/ws.rs`): one entry with 4,211 text fields padded so that `Workspace::new` succeeds with an encoding of exactly 1,048,576 bytes, which decodes; after `set_settings(WorkspaceSettings::default().with_theme_mode(Some(ThemeMode::Light)))` the encoding is 1,048,596 bytes and `Workspace::decode` returns `Err(TooLarge)`. | Kind project rule (AGENTS.md:175 and the private-invariant clause of C4), behavior: correct; `Workspace` is invariant-bearing (its constructor enforces the limit). **Severity: medium rather than high:** a document within 60 bytes of the 1 MiB limit is an edge case, though the consequence (a saved workspace the library refuses to load, so the host falls back to its default) is the silent persistence loss AGENTS.md forbids. **Fix:** besides a fallible `set_settings` (breaking), a non-breaking alternative exists: `WorkspaceSettings` has three bounded optional fields (`workspace.rs:418-422`), so the longest settings line is a small constant; `Workspace::new` (and `decode`) can check entries against `MAX_DOCUMENT_BYTES` minus that bound, so no settings replacement can overflow. Either way add the boundary round-trip test. |
| codex-C4-2 | partly: claude-C4-4 covers `ChoiceId::as_str` | **partly confirmed**: `ChoiceId::as_str` matched; `Color::channels`, `Rect::values`, `Transform2D::entries` confirmed as facts but not as departures | `core/choice.rs:48-52` `as_str`; `drawing.rs:193-201` `Rect::values`, `drawing.rs:296-304` `Color::channels`, `transform.rs:156-160` `Transform2D::entries` return the private arrays. Each compound type also has named per-component accessors (`Rect::x/y/width/height`, `Color::red/green/blue/alpha`), and the array accessors' rustdoc says they are for destructuring and backend upload. | contracts.md:1152 "Their raw value is read with `get()`" presupposes one raw value; a four- or six-component representation has none, and `get()` returning `[f32; 4]` would say less than `channels()`/`values()`. I would not record the compound accessors as findings; if the maintainer wants the policy question, it should be "the rule applies to single-value types only", and the expected answer is a wording clarification, not renames. For `ChoiceId`, C-CONV favours `as_str` for a borrowed `&str`; my C4-4 already records it as a policy question, which codex's "rename to `get`" fix pre-empts. Severity low agreed. |
| codex-C4-3 | partly: claude-C4-2 (PresentationRate, ViewState), claude-C4-3 (WorkspacePath and WorkspaceValue limits on `Workspace`; TextPreedit's limit on `TextEditError`) | **partly confirmed** for the unmatched rows: facts confirmed; departures not established for intrinsic domain bounds | Unmatched rows read: `StrokeStyle::with_miter_limit` rejects `< 1` (`geometry.rs:121-128`); `Color::rgba` and `SplitSizing::ratio` accept `0..=1` (`drawing.rs:254-262`, `split.rs:103-110`); `Capacity` rejects zero and has only `MAX` (`core/live/mod.rs:186-202`); `ContentQuery::limit`/`budget` reject zero (`content_search.rs:73-94`); `VirtualAxis::with_lanes` rejects zero (`virtual_axis.rs:187-192`); `Length`, `Size`, `Constraints`, `Rect`, `Viewport` reject negative extents (`containers.rs:19-22`, `layout.rs:138`, `:257`, `drawing.rs:55`, `:893`). | A zero, one or nonnegativity bound is the domain itself, not a chosen number a consumer needs to reference; `MIN_LANES = 1` or `Length::MIN = 0` adds nothing beyond the rustdoc. The rule's "where the type has such a limit" (rubric C4) is better read as chosen limits. The one inconsistency worth keeping is the unit interval: `NormalizedValue` exposes `MIN`/`MAX` while `Color` channels and `SplitSizing::ratio` take the same interval as raw `f32` (low, proposal). Kind: these rows are a policy question, as codex also says; the matched rows keep my severities (low). Codex omits `NumberInputError::MAX_PRECISION`, `TextError::MAX_FONT_PIXELS`, `svg::MAX_NESTING`, `VectorRasterizer`'s private edge limit and `PowerExponent`'s `f32` constants (my C4-2, C4-3). |
| codex-B3-1 | none (outside G1; codex says so) | **partly confirmed**: the fact holds; severity and fix are wrong | `menu.rs:1471-1472`: `ContextMenu::reconcile` calls `self.enabled_signal.publish_trusted(enabled)`, which replaces the value and invalidates computations and subscribers (`signal.rs:431-445`). The signal is private: created at `menu.rs:1411`, held only by the menu (`:1260`) and the private `OpenerLink` (`:1286-1293`), read with `get()` when application code calls `ContextMenuOpener::open` (`:1361`). No component, binding or application code subscribes to it. | Literal departure from AGENTS.md:132 "Never publish signals from reconciliation", which exists to keep application-observable state from changing during reconciliation; here no consumer can observe the publication. **Severity: low**, not medium. **Fix:** the public event-phase notification contract is inapt, since nobody is notified; replace the private `Signal<bool>` with a shared `Rc<Cell<bool>>` (likewise `visible_signal`, published from lifecycle hooks at `:1484`, `:1487`), which removes the departure without changing behavior. Belongs to G2's B3 verification; not a G1 item. |

## My findings disputed

1. **C1 disposition "no findings" (contradicted by codex-C1-1). Corrected.**
   The C1 disposition becomes **findings (1, project rule)**: codex-C1-1 as
   confirmed above. My C1 notes stated that "an invalid list leaves the last
   valid presentation" and that same-key redescription replaces binding and
   source together; both hold separately, but I only checked an invalid list
   under an unchanged binding (`component_bound_choices.rs:37-79`) and a
   binding change with valid lists (`:99-176`). The combination clears
   `selected`/`name` in RadioGroup's `reconcile` (`choice_controls.rs:939-943`)
   before `update` can fail (`:1010`). The probe shows Select, InlineSelect and
   MultiSelect are unaffected, so the "no departure" notes for those stand.

2. **claude-C2-1 and claude-C2-2 (codex records C2 "no findings"). C2-1
   maintained; C2-2 maintained with corrections.**
   - C2-1: codex examined the `with_` group of constructors.txt only; the
     rubric's proposal is "`from_*` and `with_*` each keep one meaning
     **across the API**". The meanings recur at the cited lines: constructor
     taking externally owned state (`menu.rs:761`, `collection_filter.rs:538`),
     capacity constructor (`navigation_history.rs:257`), property builders
     (e.g. `geometry.rs:121`, `drawing.rs:504`, core `parameter.rs:342`) and
     scoped closures (`drawing.rs:1605`, `live.rs:684`). Maintained as a
     consistency proposal.
   - C2-2: maintained as a consistency proposal, with two corrections. (a)
     Rule basis: `from_srgb` keeps the `from_` meaning, so this is not a
     breach of the C2 prefix proposal; it is a separate consistency proposal
     about the unprefixed name of the linear constructor beside the `from_`
     named sRGB ones (`drawing.rs:254-258`, `:327`, `:358`). (b) Severity
     lowered to **low**: CSS-style 0–255 channels are rejected by `rgba`
     (outside `0..=1`, `drawing.rs:259-262`), so only normalized sRGB
     fractions (such as 0.5 from a design tool) are silently read as linear.
     The claim "a consumer passing CSS values ... gets visibly lighter
     colours with no error" is withdrawn in that form.

3. **claude-C4-1 (codex classifies `TextSelection` as "documented
   control-limit snapping/clamping"). Maintained; the classification
   difference is a policy question for the checkpoint.**
   `TextSelection::new` documents clamping out-of-range offsets to the text end
   (`text_selection.rs:41-48`). AGENTS.md:175-177 permits clamping "only for
   documented control-limit behavior", meaning a control reaching its limit
   (a drag or step at a bound); an explicit byte offset passed by a caller is
   an input. Sibling offset inputs reject (`text_fields/draft.rs:7-9`,
   `:224-233`; `text_input.rs:55-80`; `text/fit.rs:230-245`). Codex cites no
   contract sentence making offset clamping a control limit. Whether the
   snapping helper should count as one is for the maintainer.

4. **claude-C4-5 (codex classifies `ProgressStatus` as composed from
   validated values, without a finding). Maintained as a policy question.**
   `ProgressStatus::of` clamps `completed > total` to a full track
   (`progress.rs:41-62`, `completed.min(total)` at `:59`), which is documented
   and reasoned but is not control-limit clamping. Codex's classification is
   about the enum's payload (`Determinate(NormalizedValue)`), which is not in
   dispute; it does not address the convenience constructor's clamp.

5. **claude-C4-2, PresentationRate's `hertz()` note. Withdrawn.** Codex
   applies the unit exception ("unless the unit itself is the point of the
   type") and I agree: the type is documented as a rate "in hertz"
   (`runtime/presentation.rs:4`, `:23-25`). The PresentationRate `MIN`/`MAX`
   part of C4-2 stands; codex-C4-3 reports it too.

## Inventory delta

Entries processed for C1–C4, by CHANGES.md row:

- `constructors.txt`: the 2 changed rows (`Constraints::bounded` moved from
  `[bound]` to `[other]`; `ApplicationError::new`'s bound path). No finding:
  `bounded` is a named constructor, not a C1 source (`layout.rs:274`).
- `state-owners.txt`: the 18 added rows. Every one leads with its state
  (`for_parameter(parameter)` ×10 including the three numeric
  `Slider`/`Knob`/`InlineSlider::new(parameter)`; `HistoryButtons::new(history,
  …)`, `ToastHost::new(queue, …)`, `LevelMeter::new`/`Trace::new(source, …)`).
  `Workspace::new(settings, entries)` is a false positive (`WorkspaceSettings`
  is a value, not bound state). These are the constructors my inventory gap 2
  had already checked by hand. No finding.
- `conversion-traits.txt`: all 152 `impl` rows re-read with their real
  targets, and the 42 "(for another type)" marks in `types.txt`. C4's
  `TryFrom` entry points are as my report used them (`ChoiceId`,
  `NormalizedValue`, `PowerExponent`, `Capacity`, `Length`, `Magnification`,
  `ViewScale`, `PresentationRate`, `UiZoom`; `OpaqueColor` from `Color`); the
  infallible `From<u16> for Length` and `From<[u16; 2]> for Size` cannot
  produce invalid values. No C4 finding. (Out of G1, for T6: only `UiZoom`
  and `PowerExponent` convert back to their raw type with `From`.)
- `free-items.txt`: all 30 rows. The free functions returning public types
  (`series_bridge`, `images::decode`/`load`, `svg::rasterize`/`rasterize_mask`,
  `svg::intrinsic_size`, `list_directory`) and constants (`svg::MAX_NESTING`,
  already in claude-C4-3; `DRAG_THRESHOLD`, a gesture threshold, not a limit
  of a type, `drag.rs:8-15`) give no new C2 or C4 finding.
- `builders.txt`: the 49 `[fallible]` rows. None takes options, a list or a
  specification (C1); all reject invalid input rather than clamp (C4);
  limits they enforce are already covered by claude-C4-3
  (`MAX_PRECISION`) or are intrinsic bounds (see codex-C4-3 above).
- Semantic lists: all 286 public field and 1254 variant lines scanned for the
  invariant-bearing types of my roles table (none exposes raw fields; the
  payloads of `PathElement`, `ProgressStatus` and `Spacing` are validated
  types), and every field or variant with a raw numeric payload (118 rows)
  read for a documented invariant. One finding below. The 25 serde trait
  entries now carrying `[configs: serde]`, plus the derived `Deserialize`
  impls in the source: invariant-bearing types deserialize through their
  checks (`PowerExponent` `serde(try_from = "f32")`, core `parameter.rs:58-62`;
  `FrameImage` `serde(try_from = "FrameImageRepr")`, `capture.rs:312-316`;
  hand-written impls for `ChoiceId`, `NormalizedValue`, `ParameterId`,
  `ParameterSpec`, `UiZoom`, `Viewport`, `ViewState`, `Workspace`,
  `WorkspacePath`, `WorkspaceValue`). No finding there.

### claude-C4-d1 `SplitRule::Ratio` carries a raw `f32` for a unit-interval value

- **Item / kind:** C4, consistency proposal (the validated type exists; the
  rule binds values consumers construct, and this one is read-only).
- **Paths, configurations:** `pert_ui_runtime::SplitRule::Ratio(f32)`
  (`split.rs:141-145`), produced by `SplitSizing::rule` (`split.rs:123-133`)
  and reported through `SplitPane::on_resize` (`split.rs:322`); all configurations.
- **Observation:** signature. The variant documents its payload as "the first
  child's share of the space after the divider, in `0..=1`", but carries a
  raw `f32`, while its sibling variants carry the validated `Length`
  (`First(Length)`, `Second(Length)`) and the API already has a validated
  unit-interval type used for the same purpose elsewhere
  (`ProgressStatus::Determinate(NormalizedValue)`, `progress.rs:30`). No public
  function takes a `SplitRule`, so no invalid value reaches the library; a
  consumer reading it cannot rely on the documented range from the type.
- **Rule:** C4 "a domain value with an invariant keeps it private";
  C-NEWTYPE. Revealed by version 2's variant payloads.
- **Severity:** low. **Confidence:** high on the facts.
- **Fix:** `SplitRule::Ratio(NormalizedValue)`. Breaking (payload type of a
  public variant), permitted while unpublished.
- **Destination:** TODO.md, or the checkpoint with codex-C4-3's unit-interval
  question.
