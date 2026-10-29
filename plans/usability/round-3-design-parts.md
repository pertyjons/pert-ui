# Round 3 checkpoint: named component parts

Status: **revision 3 approved**, 2026-10-28; implemented on the working tree,
with focused/workspace checks passed and independent implementation re-review
approved. The complete usability round remains open.
Revision 1's
use of workspace part/slot markers was rejected by fresh Claude review:
it would interrupt semantic naming and collide with application-content
workspace paths. This revision keeps those contracts independent.

The head settings-editor submission
[r3-1-head-claude](runs/r3-1-head-claude/FRICTION.md) cannot read its Select's
value through its mounted owner or Driver's address walk. The head notes
editor [r3-6-head-claude-1](runs/r3-6-head-claude-1/FRICTION.md) cannot find
a MenuBar title by its logical key through Driver::find. They need a stable
public way to reach the parts their composites expose, with no guessed
recursive key traversal and no collision with application keys.

## Separate the responsibilities

Introduce named-part metadata independent of Element::structural, part and
slot, whose existing workspace and naming meanings remain unchanged:

- `Element::expose_part(name: impl Into<String>) -> Self` declares the
  element a named interaction/inspection part of its enclosing composite.
  Repeated calls replace the name. It does not change the element's logical
  key, address marker, label, ownership, focus or callback behavior.
- `Runtime::find_exposed_part(&self, owner: &WidgetId, name: &str)` returns
  `Result<Option<WidgetId>, RuntimeError>`. It does not flush or reveal.
  A successful lookup returns the actual mounted incarnation; retain that
  WidgetId for an event, not a name or index from an older description.
- `Driver::exposed_part(&self, owner: &WidgetId, name: &str)` returns
  `Result<WidgetId, TestError>`, with RuntimeError::ForeignRuntime/Unmounted identity checks and no selected-
  window restriction, as Driver::find does today. Lookup below an owner in
  another window succeeds; a later pointer step still requires that window
  to be selected. The missing-part error is TestError::ExposedPartNotFound
  with owner (runtime-local incarnation u64), key (diagnostic logical key,
  not unique identity), name and available names. Ambiguity is
  RuntimeError::AmbiguousExposedPart { owner: u64, name: String }, never an
  arbitrary first match. Querying requires no neutral-input
  transition and does not schedule, reveal, focus or send semantic actions.
- `ComponentInspection::exposed_part: Option<String>` reports the declared
  name on the actual component record, independent of its workspace `path`.
  The record's existing id remains its target identity and parent remains
  its logical parent. A part name is a scoped selector, not another widget
  identity or an exclusive lifetime owner. No live handle enters a snapshot.

This replaces recursive logical-key guesses as the recommended route for
these composite parts. Runtime::find and workspace addressing still address
application content; Runtime::find_descendant remains generic logical
structure discovery. The existing Element::part is specifically a workspace
participant marker, not this lookup declaration; document that distinction
and cross-link both builders. No compatibility alias or second source of
selection/options is introduced.

## Lookup scope and validation

The public lookup traverses the owner's logical children in description
order. A declared exposed part is a result and a nested part scope: do not
walk into it when looking below its owner. Internal structural, workspace
part and slot wrappers are traversed; an ordinary Own-addressed child without
an exposed-part declaration is an application/component boundary and is not
traversed. Lookup on an exposed part itself starts at its children, so menus
are reached with successive scoped part lookups rather than workspace paths.

A plain application key `items` or `title/file` never matches part metadata,
and ordinary application content does not leak its descendants into a
composite's lookup. An application explicitly declaring an exposed part in
that same scope can produce ambiguity; the singular query refuses that
ambiguity. Repeated names are discoverable metadata, not invalid render
state, so no arbitrary retained subtree is destroyed to enforce lookup
uniqueness. Built-ins document and test unique names in each advertised scope.

A blank name is one that is empty after trimming. Valid names are matched exactly without trimming, path splitting or wildcard matching. A blank lookup name returns RuntimeError::EmptyExposedPartName after checking the owner identity; enumeration has no name argument. Validate a declared name before description
adoption, like an element key; an invalid declaration returns RuntimeError::EmptyExposedPartName (as element-key validation uses
RuntimeError::EmptyKey), before mutation, and preserves the last valid description and metadata. A
same-key valid redescription replaces the exposed name without remounting.
Inspection deserialization rejects a present blank name too. Ordinary
application key strings remain unconstrained by these part names.

No new scope owner, global registry, polling or cache invalidation protocol
is needed. The UI-thread lookup reads the retained tree and optional
metadata; its work is proportional to internal structure visited, stopping
at part and ordinary-component boundaries. Snapshot generation copies one
optional name per record, not every reachable part list on every ancestor.

## Built-in vocabulary

Keep all existing logical keys and workspace addressing markers:

- Select and InlineSelect expose `control` on the ComboBox anchor and
  `options` on the ListBox. Read the selected value from control, click it
  to open, then inspect/click options. ComboBox option descriptions on the
  closed anchor have no pointer rectangles because they are not drawn there.
- MenuBar exposes `title/<menu-key>` on each title. The literal slash is
  part of the name. Its application content, leading and trailing elements
  stay in their existing workspace namespaces.
- Menu, ContextMenu and each MenuBar title expose `items` for their own
  level's OptionList. A submenu panel exposes `submenu/<row-key>` in that
  level; look up its `items` from that panel for a nested level. Names follow
  stable row/menu keys, never captions, indices or translated labels.
- Menu exposes `trigger` on its button anchor. ContextMenu is opened through
  its application-provided content's pointer/context-key input; no synthetic
  semantic activation route is added.
- The shared menu construction also exposes parts for built-in text/selection
  clipboard menus. Include and test those rather than silently changing
  their observable inspection. Their existing paths stay unchanged.

The item-geometry checkpoint uses these queries to obtain menu/Select lists.
It owns title/list pointer geometry; this checkpoint owns the combined menu
and Select public examples. The tab-title example belongs to the geometry
checkpoint because titles are items of Tabs, not child components.

## Compatibility and evidence

The new optional metadata field changes the inspection schema to **17**;
reject older snapshot/query schema revisions under the existing closed-schema
policy. Record Element::expose_part, Runtime::find_exposed_part, Driver::exposed_part, Driver::exposed_parts and ExposedPart, the
new error variants, ComponentInspection::exposed_part and the schema constant
in the API listing and CHANGELOG, with replacement usage. ExposedPart is the testing module's UI-local record with public name: String
and id: WidgetId, non-exhaustive like AddressChild; it is not an owned remote
inspection DTO. Export it from both runtime/facade testing modules and the
facade's explicit testing re-export list. The new methods and enum variants
are reached through their existing public types. This is an intentional public inspection
API change in the delegated round-3 scope.

Workspace schema remains **2**: no workspace path, participant marking,
semantic naming boundary, application content address or stored preference
is changed. Verify this, including ContextMenu content keyed `items` holding
Tabs, MenuBar content keyed `title/file` holding Tabs, and leading/trailing
content. Round trips and unchanged naming/error propagation tests must pass.

Before closure, against the frozen head establish the reported missing
lookup/NoSemantics behavior; against the repair test public part lookups,
multiple titles, repeated row keys at nested levels, same-key redescription,
invalid names, ambiguous declared names, foreign/unmounted IDs and successful other-window structural queries,
and unchanged logical-key lookup. Prove custom composites can expose parts
through the same public API, with no privileged built-in path. Check hidden
parts are findable but have no bounds and querying/inspection adds no work.
Add serde round-trip, blank-name and old/unknown-schema rejection coverage.
Run workspace and API-listing gates, focused naming/addressing/persistence
and serde checks, public examples/doctests and warning-free rustdoc, then
fresh Claude implementation review. Codex authors the change; Claude reviews
without authoring fixes or delegating. Pending evidence is not called passed.

## Revision 3 decisions from review

- Use the explicit exposed-part names above instead of Driver::part, so a
  reader does not confuse lookup with Element::part's workspace vocabulary.
  Runtime::find_exposed_part and Driver::exposed_part find only names from
  Element::expose_part. Workspace parts are still found through ordinary
  Runtime::find/Driver::find.
- Add Driver::exposed_parts(owner) -> Result<Vec<ExposedPart>, TestError>.
  It lists the same immediate part scopes in logical description order,
  including duplicate declared names (a singular lookup then refuses their
  ambiguity), hidden parts and their actual retained WidgetIds. It does not
  flush, reveal or require a selected window. The private traversal is shared
  with singular lookup so enumeration/error suggestions cannot disagree.
  Skip the drag-preview subtree, like the existing address walk.
- Structural composition merges unscoped exposed names by design. An
  internal composite that needs its own namespace is itself exposed under a
  name, making it a nested scope. Built-ins isolate structurally embedded
  SearchField children of MultiSelect and QuickPick as `search`; audit the
  other new part-exposing structural compositions and document intentional
  forwarding (for example a directly delegated editor) rather than leaking
  unrelated parts. Custom-composite examples explicitly expose a nested
  Select under its own name. Test available-name lists and nested lookups,
  not only successful single-name queries.
- Clipboard menus get an exposed `clipboard` root with their own `items`
  below it. They do not add an unqualified `items` name to text editors.
  Apply the shared menu vocabulary consistently. Expose icon menus' `trigger`
  on the actual inner button, not its Tooltip wrapper, so semantics are read
  from the actionable part.
- ComponentInspection::exposed_part is descriptive metadata only. It does
  not identify a unique owning scope or provide a cross-thread lookup
  protocol; external tooling cannot reproduce the scoping walk from that
  field alone. Hosts that need lookup run the public Runtime method on the
  UI thread and capture an owned inspection of the returned incarnation.
- Give the optional inspection field a blank-name-validating serde reader;
  under schema 17 the field is required (null means no declaration). Missing
  fields, blank declarations and unsupported/unknown schemas are rejected;
  update current error examples and serde fixtures without rewriting frozen
  usability records from schema 16.
- Treat exposed-name-only redescription separately from layout metadata:
  replace it without invalidating parent layout, paint or scheduling an
  extra update. Tests check replacement and removal, same identity, and
  settled work/paint state. Existing component redescription costs are not
  claimed to disappear.

The revision-2 geometry design was found ready in the independent review,
conditional on this companion's approval. Fresh Claude recheck found revision 3 ready. Approved by Codex as author
under the maintainer's delegated goal, 2026-10-28. The reviewer authored no
fixes or tests. Its remaining nonblocking wording points are incorporated:
blank means empty after trimming; the RuntimeError exhaustive provenance
match must cover both new variants; name-only metadata bypasses the parent's
layout invalidation path. CommandPalette intentionally forwards QuickPick's
search part inside the existing addressable `surface` scope and documents that
shared vocabulary. Implementation tests confirmed that the picker itself stops
at that ordinary address boundary; the route is `find(["surface"])`, then
`exposed_part("search")`, then `exposed_part("clipboard")`. This clarification
keeps the approved traversal rule and every address marker unchanged.
