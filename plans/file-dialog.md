# Plan: file dialog

Status: slices 0, 1 (0.3.123, repairs in 0.3.124) and 2 (0.3.125) done.
Slice 3 (the own window) is deferred until an application needs it
(decision 13). Slice 4 (0.3.126) is done; slice 5 is not started. Every type and method this plan
names is a proposal until a slice implements and documents it. The
[decisions](#decisions) were settled on 2026-11-19. An independent Codex
review of the first draft (2026-11-19, read-only, static) found two blockers
and eight major gaps; this revision answers them, and
[Review record](#review-record) lists what changed.

## Goal

An application can ask the user for a file or folder to open, or a path to
save to, with a dialog the library draws itself. The dialog can be presented
inside the owner window or, once slice 3 has native evidence, as its own
native window, which then becomes the default. The dialog remembers where the
user was, separately for each purpose the application names, and the
application can persist that memory.

## Background

What exists and is reused:

- `FileBrowser` and `FileBrowserController`
  (`crates/pert-ui-runtime/src/file_browser.rs`): breadcrumb, entry filter,
  lazily listed folder tree and virtualized entry list in a split pane. The
  browser never touches the filesystem: the host answers `DirectoryRequest`s,
  normally from a worker, and delivers each `DirectoryResponse` on the UI
  thread. A `DirectoryEntry` carries a native path and name, a kind and an
  optional error, no size or modification time. `on_open` reports the
  activation of any entry that does not open as a directory, which includes
  broken links, unreadable entries and `EntryKind::Other`; it holds one
  callback, and a later one replaces it. Constructing the controller takes a
  `ListSelection` and requests the first root's listing at once. The folder
  pane, entry source and layout assembly are private.
- `Dialog`, `DialogController` and `AlertDialog`
  (`crates/pert-ui-runtime/src/dialog.rs`). A dialog is window-modal by
  default; `view_modal` confines it to its owner's region and
  `application_modal` blocks every other window. All three are drawn in the
  presentation root that opened them; none has a native window of its own.
  The controller reserves one outcome slot per opening and retains the
  outcome until `take_outcome`: `Accepted`, `Cancelled`, `Dismissed(reason)`
  when ownership or presentation ended without a response (including removal
  of the mounted owner before layout) and `Failed(reason)` when admission
  failed, for example on a modal conflict. A dialog mounts its content while
  closed and wraps its body in a scroll area.
- Native windows (`crates/pert-ui-runtime/src/runtime/native_windows.rs`,
  `crates/pert-ui/src/platform/windows.rs`). Besides the main window, a
  native window is created only by detaching a mounted view:
  `Runtime::request_detach` refuses a hidden view or a subtree with an open
  modal at once, while `EventContext::detach_view` queues the request and
  reports those refusals through its `PresentationReceipt`. The commit moves
  the existing view into a detached root and keeps its return focus; closing
  a non-primary window redocks, removes the view or refuses. There is no
  request for a window owned by another window, no native modality and no
  placement. On Linux the adapter reports activation as unsupported.
- `Workspace` ([contracts](../docs/contracts.md#workspace-persistence)): a
  versioned closed document of runtime settings and the arrangement of
  mounted views, hidden ones included. The browser's split pane takes part.
- After-settling notifications
  ([contracts](../docs/contracts.md#delivery-order-and-hidden-windows)): a
  component removed before delivery reports nothing, so they cannot carry a
  result that must not be lost.

What is missing: the dialog composition around the browser (name field, type
filter, confirm and cancel, modes, outcome), a native window that is a modal
dialog of the application, and the memory.

[app-author-improvements.md](app-author-improvements.md) slice 8 lists the
operating system's own file dialog. That is a different service and stays
there; see [Out of scope](#out-of-scope).

## Design

### Appearance

```
┌ Open project ───────────────────────────────────────────── ✕ ┐
│ ▲   / home / per / projects / demo                [ filter ] │
├────────────┬──────────────┬──────────────────────────────────┤
│ PLACES     │ ▸ projects   │ 📁 assets                        │
│  Projects  │   ▸ demo     │ 📁 src                           │
│  Samples   │              │ 📄 demo.pert                     │
│ RECENT     │              │ 📄 notes.txt                     │
│  ~/demo    │              │                                  │
├────────────┴──────────────┴──────────────────────────────────┤
│ Name: [ demo.pert                  ]  [ Pert files (*.pert)▾]│
│ [New folder]                             [Cancel]  [ Open ]  │
└──────────────────────────────────────────────────────────────┘
```

- The two right-hand panes are the existing `FileBrowser`. Places and recent
  folders are a list of the dialog's own beside it, so the composition needs
  no slot inside the browser's private layout. Activating a place navigates
  the browser's controller.
- Navigation offers the parent folder and the breadcrumb, which the browser
  has. Back and forward history is not in this plan.
- Modes: open one file, open several files, choose a folder, save as. The
  confirm button's caption and the name field follow the mode. The New
  folder button arrives with slice 4.
- The type filter is a `Select` over the fixed list the opening request
  carries. It restricts which files the entry list shows; folders always
  show.
- The dialog needs a bounded height, as the browser does. Slice 0 specifies
  how it is allocated inside `Dialog`'s scroll wrapper and what a viewport
  smaller than the dialog's minimum shows.

### Session, request and outcome

`FileDialogController` follows `DialogController`'s session model instead of
a callback: opening reserves one outcome slot, `status()` is a signal the
application reads, and the terminal outcome is retained by the controller,
independent of any mounted component, until `take_outcome`.

```rust
// API sketch, not an available function.
let dialog = FileDialogController::new(&ui, roots, request)?;   // request: DirectoryRequest sink
// ... mount FileDialog::in_window(&dialog, anchor) in the tree ...
dialog.open(
    FileDialogRequest::new(FileDialogMode::OpenFile)
        .title("Open project")
        .context("open-project")?
        .filters([FileFilter::new("Pert files", ["pert"])?])?,
)?;
// later, where the application reads dialog.status():
match dialog.take_outcome()? {
    Some(FileDialogOutcome::Accepted(paths)) => { /* load */ }
    Some(FileDialogOutcome::Cancelled) => {}
    Some(FileDialogOutcome::Dismissed(reason)) => {}
    Some(FileDialogOutcome::Failed(reason)) => { /* report */ }
    None => {}
}
```

- **Configuration belongs to the opening.** Mode, title, context, filters and
  a suggested name travel in a validated `FileDialogRequest` and are fixed
  for that session. One controller serves any number of purposes, and
  redescribing the mounted `FileDialog` never changes a session. Fixed
  options are validated when the request is built, before anything is
  dispatched.
- **No work before opening.** The controller asks for no listing until
  `open`, when the starting folder is known. Slice 1 either creates the
  browser controller per opening or gives `FileBrowserController` a start
  that defers its first request; whichever it is becomes public API.
- **The dialog owns its browser controller**, so its use of `on_open` cannot
  displace an application handler. Activation and acceptance stay separate:
  activating an entry is a request to accept, judged by the mode's rules.
- **Four outcome classes**, as for `Dialog`: `Accepted(paths)`, `Cancelled`
  (the user declined), `Dismissed(reason)` (owner removed, window closed
  under it, runtime shutdown while a participant is mounted) and
  `Failed(reason)` (admission or presentation failed). A listing failure is
  shown in the dialog and is not an outcome.
- **Retention and precedence.** `open` while a session is open or its outcome
  is untaken returns a busy error. The first terminal transition wins: an
  accept followed by removal of the owner in the same input stays `Accepted`.
  A second mounted owner of one controller stays closed and fails its
  updates, as `Dialog` does. Slice 0 writes down what remains readable after
  runtime shutdown.

Acceptance is specified by a decision table in slice 0 and tested per row:

- Per mode: what Enter, a double click and the confirm button accept, given
  the highlighted entry, the selection and the name field. Choose folder
  accepts the highlighted folder when there is one, otherwise the current
  folder. Several files are reported in display order, and a selected entry
  the filter hides is not reported.
- Which entry kinds a mode accepts: links by their listed target, never an
  entry with an error, never `EntryKind::Other`.
- The name field: text naming a folder navigates; in save as, anything else
  must be exactly one normal native path component, so `..`, `.` and names
  with a separator are refused in place.
- Selected paths are reported as the `PathBuf` the listing delivered,
  untouched, so two names that render alike stay distinct.
- Save as appends the selected filter's first extension only when the typed
  name has none; the rule is documented and replaceable on the request.
- Replace confirmation: the candidate is fixed first, extension included,
  then checked against the unfiltered current listing; the `AlertDialog`
  names that candidate. While the listing is loading or failed, confirm is
  unavailable. The listing can be stale: whether the path can be read or
  written when the application acts on the outcome is the application's to
  discover.

### Presentation and modality

The modal scope is defined by the existing surface kinds, with no new one:

- `FileDialog::in_window(&controller, anchor)` presents a `Dialog` in the
  owner's presentation root: window-modal by default, `view_modal` on
  request. This is the only presentation slices 1 and 2 ship.
- The content element is available on its own for an application that wants
  the picker as an ordinary panel. Composing `Dialog` and `FileBrowser` by
  hand remains possible and becomes the advanced path; the slice's change
  rationale says so, and one facade-only test builds that composition from
  the public hooks.
- The own-window presentation (slice 3) is application-modal: it blocks every
  other presentation root and targetless commands, exactly as
  `Dialog::application_modal` does today, and differs only in being drawn in
  a native window of its own. The replace confirmation and other nested
  dialogs are admitted in the dialog window's own chain. This is the only
  modal scope the own window offers to begin with (decision 8).
- `FileDialog::new(&controller, anchor)`, defaulting to the own window, is
  introduced in slice 3 and not before, so no application's modal scope
  changes under an unchanged call.

The own window is a general capability of `Dialog` (decision 1). It cannot be
built on detach: detach moves an existing view, keeps its return focus and
closes by redocking. Slice 3 adds a distinct presentation request; its design
note must answer, before any code:

- Identity and lifetime: the logical owner (the mounted `Dialog`), the
  native root that invoked it and the session identity; what happens when
  the owner hides, detaches, redocks or unmounts while the request is
  pending and while the window is open.
- The prepare, commit and retire protocol and its receipt: revalidation of
  the owner at commit, rollback, and completion on a full request queue, a
  dropped request, a renderer failure or a stale owner. Each ends the session
  as `Failed` or `Dismissed`, never silently.
- Hosts without the capability. A headless or custom host answers the request
  as unsupported with a typed status, and the session fails; there is no
  silent fallback to the in-window presentation. An application that wants
  the fallback asks for `in_window` itself.
- Close and focus: the dialog window's title-bar close is a cancel request
  and is distinct from Escape only in its source. A close request on the
  owner or main window follows
  [closing a window](../docs/contracts.md#closing-a-window): the dialog
  session ends as `Dismissed` only when the close is accepted, not when
  admission starts, so a refused or deferred close leaves the session live.
  The note fixes the affected roots, ordering under coalesced and deferred
  decisions and error backoff, and when focus returns to a still-eligible
  opener.
- Platforms: runtime input blocking is the library's and holds everywhere.
  Owner relation, placement and activation are operating-system services
  reported per capability; where one is unavailable, as activation is on
  Linux today, the status says so and modality is still enforced by the
  runtime. Any new unsafe boundary needs the maintainer's approval first.

### Memory

`FileDialogMemory` is a state object the application owns and any number of
controllers share (`FileDialogController::with_memory`). It is separate from
the controller because it outlives any one dialog.

- Per context key: the last folder and the selected type filter. A filter is
  remembered by an identity the application gives it, not by index or label;
  a remembered identity the request no longer offers selects the first
  filter. A request without a context uses a default context.
- Shared: recent folders with a documented capacity, deduplicated with the
  newest first, and the dialog's size in logical pixels, kept separately for
  the in-window and own-window presentations.
- Places are supplied by the application as a value it can bind and are not
  stored in the memory; the library has no defaults (decision 4).
- On open, the dialog starts in the context's last folder. If that listing
  fails as not found or not a directory, it tries each ancestor in turn and
  then the first root; a permission failure stops the search and is shown.
  The search belongs to the opening and stops at the user's first
  navigation, so a late response cannot move the browser.
- Writes happen on accept only, as an atomic merge of that session's changes
  into the current memory before the outcome becomes readable. A session
  never writes back its opening snapshot, so two dialogs sharing one memory
  cannot erase each other's updates; the later accept wins for a shared
  context. A resize followed by cancel is not remembered.
- Persistence: `encode` and `decode` over its own versioned closed schema
  with documented bounds, deterministic output and duplicate rejection.
  Tests cover round trips, unsupported versions, unknown fields, invalid
  keys, over-capacity lists and one invalid entry in an otherwise valid
  document, which rejects the document. The host chooses storage.
- Paths: version 1 stores Unicode paths only. A folder whose path is not
  valid Unicode is not remembered, the dialog reports that it was not, and
  nothing lossy is stored. A lossless encoding is a later schema version.
- It is a separate document from `Workspace` because it is a user preference
  with its own owner and lifetime, not the arrangement of a view. The
  browser's split pane inside an in-window dialog is mounted while the dialog
  is closed and keeps taking part in `Workspace`; tests capture and restore
  with the picker closed, open and embedded.

## Out of scope

- The operating system's own file dialog (portal, Windows, macOS). It stays in
  [app-author-improvements.md](app-author-improvements.md) slice 8.
  `FileDialogRequest`, `FileFilter` and `FileDialogOutcome`, including its
  failure class, are shaped so that service can reuse them.
- File operations other than creating a folder: rename, delete, move, drag
  and drop, previews and thumbnails.
- Back and forward navigation history.
- Remote or virtual filesystems beyond what the host-supplied directory
  source already allows.

## Working rules

- One slice per branch, a patch version bump per slice, and rustdoc,
  examples, the gallery, the manual and the CHANGELOG updated with the
  slice's own API.
- Read [contracts](../docs/contracts.md) before each slice, in particular
  menus, pickers and dialogs, component redescription, user-write callbacks,
  host-loaded data, closing a window, runtime shutdown and workspace
  persistence.
- Gates per the [AGENTS.md](../AGENTS.md) table:

  | Slice | Gate |
  |---|---|
  | 0 | Markdown only: author review, links, consistency with existing decisions; maintainer approval of the note |
  | 1, 2, 4, 5 | Public API and, for 2 and 5, persistence: workspace gate, API listing gate, focused contract tests, semantic review |
  | 3a | Public API, scheduling and cross-package: as above |
  | 3b | Platform integration: the full dependency/platform row and native evidence recorded in [verification](../docs/verification.md) |

- An independent Codex review (`codex exec -s read-only`) before a slice is
  called merge-ready.

## Slices

### 0. Design note

Status: drafted 2026-11-19 as [file-dialog-design.md](file-dialog-design.md)
and revised after a Codex review of the draft; approved by the maintainer
on 2026-11-19. Its section 9 lists where it narrows or corrects this plan; where
the two differ, the note is the newer text. It is written and approved
before slice 1 fixes public API:

- The constructor, request and outcome API above, with the retention and
  precedence rules and shutdown behaviour.
- The acceptance decision table.
- The public additions `FileBrowser` and its controller need (deferred first
  request, an entry predicate for the type filter, listing state for the
  current folder) and the dialog's size allocation.
- The modal scope of each presentation, and the outline of the own-window
  request far enough to show slice 1's API does not have to change for it.

### 1. Dialog content and outcome, in the window

Status: implemented in 0.3.123 on `file-dialog-slice-1`, with the repair of
TODO.md DIALOG-01. [file-dialog-design.md](file-dialog-design.md) section 10a
lists where the implementation differs from the note, what it found
(TODO.md DIALOG-02, and that reacting to an outcome needs an application
component) and what is not tested.

- `FileDialogController`, `FileDialogRequest`, `FileDialogMode`,
  `FileFilter`, `FileDialogOutcome`, `FileDialog::in_window` and the plain
  content element; the browser additions from slice 0.
- Contract tests as a module of the `surfaces` area binary, one per rule:
  each row of the decision table; one outcome per opening; busy on reopen;
  accept then owner removal in one input; owner removal before first layout;
  a duplicate owner; modal conflict as `Failed`; runtime shutdown;
  redescription during a session; a stale listing after navigation; a
  viewport smaller than the dialog, driven by keyboard; the facade-only
  manual composition.
- Gallery page and a guide section with a complete example that mounts,
  opens, delivers listings and consumes the outcome.

### 2. Memory

Status: implemented in 0.3.125 on `file-dialog-slice-2`.
[file-dialog-design.md](file-dialog-design.md) section 10b lists what was
decided while implementing and what is not tested. The dialog's size is not
in the memory yet: the in-window dialog has a fixed size (note section 9,
item 5), so it arrives with the own window in slice 3b.

- `FileDialogMemory`, `with_memory`, the starting-folder search, recent
  folders and application-supplied places in the dialog.
- The schema with `encode`/`decode` and the tests named under
  [Memory](#memory), plus overlapping accepts from two dialogs, cancel after
  resize, a search overtaken by navigation and by a reopened dialog, and the
  `Workspace` captures.
- The gallery persists it beside its workspace to demonstrate the host side.

### 3a. Dialog window: runtime protocol

Status: deferred with 3b by decision 13. The in-window dialog is the
presentation the library ships. [file-dialog-window-design.md](file-dialog-window-design.md)
is the parked design for when the own window is taken up; its review found
that the detach protocol does not carry a modal dialog as it is, so the text
below understates the work.

- The design note for the own-window request, answering every point under
  [Presentation and modality](#presentation-and-modality).
- The runtime request, receipt and host-integration steps, with `Dialog`
  gaining the capability; no platform code.
- Headless tests with a fake host: commit, refusal as unsupported, failure
  between prepare and commit, a full queue, owner unmount, hide, detach and
  redock while pending and while open; a main window, two detached views, a
  nested alert and targetless commands under the application-modal scope;
  refused, deferred and accepted close of the owner.

### 3b. Dialog window: platform and default

- The platform adapter for both presenters (GPU and software), winit types
  behind the platform boundary, per-capability status for owner relation,
  placement and activation.
- `FileDialog::new` with the own window as default; the gallery shows both.
- A native check in `pert-ui-native-checks` and Linux evidence recorded with
  its platform; Windows and macOS evidence is recorded as missing until run.
  Platform claims are qualified by that evidence.

### 4. New folder

Status: implemented in 0.3.126 on `file-dialog-slice-4`;
[file-dialog-design.md](file-dialog-design.md) section 10c has what was
decided while implementing and what is not tested.

- An optional host capability: a `FileOperationRequest` ticket the host
  answers, as it answers listings and by the
  [host-loaded data](../docs/contracts.md#host-loaded-data) rules.
  Without the capability the button is absent; it is a `Permission` the
  application can bind, and revoking it cancels a pending name entry.
- The name is one normal native path component, validated before the request.
  A second submission while one is pending is refused.
- Continuations are bound to the opening and its navigation generation: a
  late success after cancel or reopen neither selects nor navigates. A
  handoff failure and a creation failure are shown in the dialog. When
  creation succeeded but the following refresh failed, retry only refreshes.
- Tests for each of those, including cancellation while the worker runs.

### 5. Entry metadata and sortable columns

- `DirectoryEntry` gains optional size in bytes and modification time, each
  absent when unknown, never zero; a partial metadata failure leaves the
  entry listed. A link reports its own metadata, not its target's, as its
  kind does. `list_directory` fills them in.
- The entry list shows name, size and modified columns and sorts by any of
  them, folders first, unknown values last and ties by name. Selection and
  the active entry survive a sort change.
- The sort choice joins the shared memory as a new schema version; a
  version-1 document is handled as the persistence rules require, with a
  test, never reinterpreted.

## Decisions

Settled by the maintainer on 2026-11-19, following the plan's proposals:

1. The dialog window of slices 3a and 3b is a capability of `Dialog` in
   general; the file dialog is its first user and gets no window kind of its
   own.
2. The dialog's size is kept in `FileDialogMemory`. Window geometry
   persistence ([app-author-improvements.md](app-author-improvements.md)
   slice 8) may take it over later as a schema change of its own.
3. Size and modification time, with sortable columns, are slice 5 of this
   plan rather than part of the first slices.
4. Places have no defaults. The library cannot know the user's home or
   documents folder without an operating-system service, so the application
   supplies them.

Confirmed by the maintainer on 2026-11-19, after the review:

5. The outcome is a retained slot read with `take_outcome`, as for
   `DialogController`. Amended by the maintainer on 2026-11-19 after slice 1
   showed that an application otherwise needs a component of its own to act
   on an ending: `FileDialog::on_outcome` hands the retained outcome over
   after settling, and a removed dialog leaves it with the controller.
6. Mode, filters and context travel in the request given to `open`, not on
   the controller or the description.
7. `FileDialog::in_window` ships first; the own window becomes the default
   only with `FileDialog::new` in slice 3b.
8. The own window has one modal scope to begin with: application-modal. It
   fails on a host that cannot present it, with no silent fallback. A scope
   that blocks only the owner's window chain is a possible later addition and
   needs a new modal scope in the runtime.
9. Memory version 1 stores Unicode paths only; a lossless encoding is a
   later schema version.
10. Back and forward history is out of scope, and places are application
    data rather than stored bookmarks.
13. The own window is deferred (maintainer, 2026-11-20). `FileDialog::in_window`
    and `in_view` are the shipped presentations, and decision 7's
    `FileDialog::new` is not introduced. Slice 3 is taken up again when an
    application needs a dialog outside its owner's window, starting from the
    parked design note and its open decisions.
12. Slice 2 scope, confirmed by the maintainer on 2026-11-20 after its
    review: an explicit `start_folder` takes precedence over the remembered
    folder and is not searched for; the dialog's size joins the memory with
    the own window in slice 3b, not before; and the gallery shows the memory
    document instead of storing it, since it has no settings storage. Where
    the [Memory](#memory) section and the slice 2 text say otherwise, this
    decision holds.
11. A `Dialog`'s documented `dialog` segment is a stable part of workspace
    paths (decided 2026-11-19; it was TODO.md DIALOG-02). The panel does not
    become structural.

## Review record

Codex, 2026-11-19, `codex exec -s read-only` on the first draft; static
reading only, no builds, tests or native sessions, and no check of
operating-system ownership APIs on Windows or macOS. Each finding was
checked against the cited code before the plan changed.

- Blocker, lost outcome: an after-settling callback reports nothing once its
  component is removed. Answered by the retained outcome slot (decision 5).
- Blocker, two definitions of modality for the own window. Answered under
  [Presentation and modality](#presentation-and-modality) and decision 8.
- Background corrected: `Dialog`'s default modality and four outcome
  classes, what `on_open` receives, `Runtime::request_detach` versus
  `EventContext::detach_view`, what `Workspace` holds and that a closed
  dialog's content stays mounted.
- Configuration ownership and work dispatched at construction: decision 6
  and "No work before opening".
- Composition through private browser parts, the missing history and the
  size allocation: places beside the browser, history out of scope, slice 0.
- Acceptance and overwrite left to guesswork: the decision table.
- Detach cannot carry a dialog window; close, focus and Wayland: slices 3a
  and 3b and the questions their note must answer.
- Memory commit rule, filter identity, non-Unicode paths, the starting-folder
  search and new-folder continuations: [Memory](#memory) and slice 4.
- Slice order and gates: slice 0 added, the default deferred to 3b, slice 3
  split, gates assigned to every slice.
