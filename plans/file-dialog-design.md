# File dialog: design note (slice 0)

Status: approved by the maintainer on 2026-11-19, with the ten points of
section 9. This is
slice 0 of [file-dialog.md](file-dialog.md). Nothing here is implemented;
every type, method and signature is a proposal. When a slice implements a
part, its contract moves into [contracts](../docs/contracts.md) and rustdoc,
and this note stops being the reference for it.

The note fixes what slice 1 would otherwise have to guess: the public API,
the session rules, what each mode accepts, what the file browser must expose,
and how the in-window dialog is sized. An independent Codex review of the
first draft (2026-11-19, static) found four blockers and eleven major gaps;
[section 12](#12-review-record) lists them and where each is answered.

## 1. What the existing code decides

Read from the code on 2026-11-19; these facts shape the design.

- `FileBrowserController::new` requests the first root's listing during
  construction. Its request callback is called synchronously from the
  controller's methods and, by its documentation, must not call the
  controller or write signals.
- A `ListSelection`'s mode is a plain field fixed at construction and copied
  into every clone. `list_selection.rs` reads it when a gesture or a write is
  interpreted; `FileBrowserController::selected_paths` and its selection
  pruning read it through `mode()`.
- `selected_paths` reports paths in key order. Keys are `p` and a number
  given when a path is first met, compared as text, so `p10` sorts before
  `p2`; the order is unrelated to what the user sees.
- The entry list shows the current folder's listing filtered by the text
  filter. The unfiltered listing, whether the folder is loading or failed,
  and the entries in display order are private. The list's source is cached
  on the directory, the listing's allocation and the query.
- The loading, error, empty and no-match rows of the entry list are
  placeholder rows with keys. They can become the active row and be
  selected; activating one does nothing, because no entry has that key.
  Activating a directory navigates inside the browser and does not call
  `on_open`.
- Navigating to another folder clears the text filter and the entry
  selection. A listing delivered for the current folder removes selected and
  active keys the new listing no longer has; a failed listing removes them
  all. Changing the text filter does not change the selection, so a selected
  key can be hidden.
- The browser handles Backspace as "parent folder" only when the key reaches
  the browser component, that is, when focus is inside it.
- `Dialog` caps its panel at the window minus a margin. Inside, a scroll
  area with both axes measures the body with the available width and
  unbounded height, and arranges it at the larger of the viewport width and
  the body's minimum width. `FileBrowser` needs a bounded height.
- `DialogController::open` is refused with `DialogBusy` while a session is
  open, while its outcome is untaken, and after a finish until the mounted
  owner has observed the closure in an update. A finished session is not
  overwritten by a later owner removal. `Dialog` maps a modal conflict and
  lack of space to `Failed`, Escape to `Cancelled` and every other
  terminating reason to `Dismissed`.
- `Dialog::dismiss_surface` finishes its controller's session without
  checking that this mount owns the controller, and the runtime calls it for
  any removed or logically hidden component that has a surface child. A
  second mounted `Dialog` with the same controller, which stays closed and
  fails its updates, therefore ends the first one's session when it is
  removed. This is a defect in existing code, recorded in
  [TODO.md](../TODO.md); slice 1 depends on its repair (section 3, rule 7).
- `AlertDialog`'s affirmative button accepts the alert's own controller and
  runs no application code.
- Runtime shutdown removes the mounted roots before it marks the runtime
  closed. After that, signal reads return `RuntimeError::Closed`.

## 2. Public API

### 2.1 Controller

```rust
// Proposal. UI thread only; clones share the session.
pub struct FileDialogController { /* private */ }

impl FileDialogController {
    /// `roots` and `request` as for `FileBrowserController::new`.
    /// Requests no listing.
    pub fn new(
        ui: &impl StateOwner,
        roots: impl IntoIterator<Item = impl Into<PathBuf>>,
        request: impl FnMut(DirectoryRequest) + 'static,
    ) -> Result<Self, FileDialogError>;

    /// Starts a session with `request`'s configuration.
    pub fn open(&self, request: FileDialogRequest) -> Result<(), FileDialogError>;

    /// Hands a listing to the dialog's browser.
    pub fn deliver(&self, response: DirectoryResponse) -> Result<ListingDelivery, RuntimeError>;

    pub fn status(&self) -> ReadSignal<FileDialogStatus>;
    pub fn take_outcome(&self) -> Result<Option<FileDialogOutcome>, RuntimeError>;

    /// What the confirm button does; returns whether the session ended.
    pub fn accept(&self) -> Result<bool, RuntimeError>;
    /// Ends an open session as `Cancelled`; returns whether it was open.
    pub fn cancel(&self) -> Result<bool, RuntimeError>;

    /// The folder the dialog shows; the first root before the first session.
    pub fn current(&self) -> ReadSignal<PathBuf>;
}
```

- `new` allocates everything a session needs, since `open` has no state
  owner to allocate from: an idle browser controller (section 5), its entry
  selection, a private `DialogController`, and the signals the mounted
  dialog binds: title, captions, the filter list and the selected filter,
  the name text, the status message and whether the replace question is
  showing. `open` only writes them.
- The dialog does not hand out its browser controller. It installs the
  browser's single `on_open` callback and its entry predicate, and an
  application writing either would break the session. That callback holds
  the dialog controller weakly, so the browser it owns does not keep it
  alive. The dialog uses only the browser's public API, so an application
  can build the same thing by hand (section 5).
- Construction errors are the dialog's own (`FileDialogError::NoRoots`,
  `DuplicateRoot`, `Runtime`), so the browser is not part of the dialog's
  construction contract.
- `FileDialogStatus` is `Idle`, `Open` or `Finished`, a `Copy` value derived
  from the private dialog controller's status. The outcome is read with
  `take_outcome`.
- `accept` and `cancel` exist for the dialog's own buttons, for shortcuts an
  application adds and for tests. `accept` applies section 4 and never
  bypasses it. It returns `false` when the session is not open, when confirm
  is unavailable, when it refused, navigated or asked the replace question.

### 2.2 Request

```rust
// Proposal. Plain data; `Send`.
pub struct FileDialogRequest { /* private, validated */ }

impl FileDialogRequest {
    pub fn new(mode: FileDialogMode) -> Self;
    pub fn title(self, title: impl Into<String>) -> Self;
    pub fn accept_label(self, label: impl Into<String>) -> Self;
    pub fn context(self, key: impl Into<String>) -> Result<Self, FileDialogError>;
    pub fn filters(
        self,
        filters: impl IntoIterator<Item = FileFilter>,
    ) -> Result<Self, FileDialogError>;
    pub fn start_folder(self, folder: impl Into<PathBuf>) -> Self;
    pub fn suggested_name(self, name: impl Into<String>) -> Result<Self, FileDialogError>;
    pub fn append_extension(self, append: bool) -> Self;
}

#[non_exhaustive]
pub enum FileDialogMode { OpenFile, OpenFiles, ChooseFolder, SaveFile }
```

- Builders replace earlier values. Everything is fixed for the session the
  request opens.
- **Texts are fixed strings, not `TextSource`s.** This is a deliberate
  exception to the caption convention: the request is plain data that the
  operating-system dialog service will also take, and a binding cannot
  travel there. A text the application gives is used as given for the
  session. A text it does not give comes from the installed string table and
  follows a later table change, also during the session.
- `accept_label` wins over every default caption, including `ChooseFolder`'s
  caption that names the folder (section 4.5).
- `context` is the memory key (slice 2); in slice 1 it is validated and has
  no effect. A context key and a filter identity are 1 to 64 Unicode scalar
  values, with no control character and not only whitespace.
- `filters` rejects two filters with the same identity. An empty list means
  no type filter and no filter selector. The first filter is selected when
  the session starts, until slice 2 remembers the choice.
- `start_folder` is where the session starts. Precedence: the request's
  folder, then the context's remembered folder (slice 2), then the first
  root.
- `suggested_name` is the initial text of the name field in `SaveFile`,
  validated by the name rule of section 4.4; other modes ignore it. It is
  Unicode text, as everything typed into the name field is.
- `append_extension` is on by default (section 4.4).

### 2.3 Filter

```rust
// Proposal.
pub struct FileFilter { /* private, validated */ }

impl FileFilter {
    /// `id` is the filter's identity for memory and for the outcome.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        extensions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, FileDialogError>;
    /// A filter every file passes.
    pub fn all(id: impl Into<String>, label: impl Into<String>) -> Result<Self, FileDialogError>;
}
```

- An extension is given without its leading dot. It is not empty and
  contains no `/`, `\`, `*`, whitespace or control character, and does not
  start or end with a dot; `new` needs at least one. `tar.gz` is valid.
- A file passes when its name, lowercased with `str::to_lowercase`, ends
  with a dot and a lowercased extension, and has at least one character
  before that dot. A name that is not valid Unicode is compared through its
  lossy text, which keeps a valid suffix intact.
- The filter restricts files only. Entries that open as directories always
  show, so the user can navigate.

### 2.4 Outcome

```rust
// Proposal.
#[non_exhaustive]
pub enum FileDialogOutcome {
    Accepted(FileDialogSelection),
    Cancelled,
    Dismissed(SurfaceDismissal),
    Failed(FileDialogFailure),
}

pub struct FileDialogSelection { /* private */ }
impl FileDialogSelection {
    pub fn paths(&self) -> &[PathBuf];        // never empty
    pub fn into_paths(self) -> Vec<PathBuf>;
    pub fn filter(&self) -> Option<&str>;     // the selected filter's identity
}

#[non_exhaustive]
pub enum FileDialogFailure {
    /// The surface could not be shown: a modal conflict, or no space for it.
    Surface(SurfaceDismissal),
    /// The session was reserved but could not be started (section 3, rule 1).
    Start(RuntimeError),
    // Slice 3a adds the own window's refusals, for example an unsupported host.
}
```

- The selection reports the filter selected at acceptance, so a save dialog
  offering several formats tells the application which one the user chose.
- The four classes follow `Dialog`'s mapping: `Cancelled` for the cancel
  button, Escape and `cancel()`; `Failed(Surface(..))` for a modal conflict
  and for lack of space, which `Dialog` rechecks while a surface is shown, so
  it can also end a session that was visible; `Dismissed` for every other
  end without a response.
- A listing that fails is shown in the dialog. It is not an outcome.

### 2.5 Presentation

```rust
// Proposal.
pub struct FileDialog { /* private */ }

impl FileDialog {
    /// A dialog in the owner's presentation root, modal to its window.
    pub fn in_window(controller: &FileDialogController, anchor: Element) -> Self;
    /// A dialog confined to the owner's region, modal to that view.
    pub fn in_view(controller: &FileDialogController, anchor: Element) -> Self;
    pub fn size(self, size: Size) -> Result<Self, FileDialogError>;
    pub fn row_height(self, height: f32) -> Result<Self, FileDialogError>;
}
```

- The modal scope is chosen by the constructor, not by a builder, so slice
  3b's `FileDialog::new` (own window, application-modal, decision 8) cannot
  be combined with another scope. `new` uses the same controller, request
  and outcome; `FileDialogFailure` is the only type that grows.
- `anchor` is ordinary content, typically the view the dialog belongs to,
  exactly as for `Dialog::new`. The mounted `FileDialog` is the session's
  owner.
- A mounted `FileDialog` keeps its controller and its row height for life,
  as `FileBrowser` does: a new description under the same key cannot replace
  either, and an application that wants another controller gives the element
  another key. A new description's size and modal scope apply from the next
  session, including one that is open but not yet presented.
- `size` defaults to 720 × 480 logical pixels and rejects a size below
  480 × 320 with `FileDialogError::TooSmall`. `row_height` defaults to 24
  logical pixels; geometry refusals arrive as `FileDialogError::Geometry`.
- Inspection: the owner reports `SemanticRole::Dialog` named by the title.
  The confirm button, the cancel button, the name field and the filter
  selector are exposed parts of the dialog's content element, named
  `accept`, `cancel`, `name` and `filter`; a test finds the content under
  the owner's `dialog` segment and then the part, as for `AlertDialog`.

### 2.6 Errors

`FileDialogError` is a `thiserror` enum, `#[non_exhaustive]`: `Busy`,
`NoRoots`, `DuplicateRoot(PathBuf)`, `InvalidContext`, `InvalidFilter`,
`DuplicateFilter`, `InvalidName`, `TooSmall`, `Geometry(GeometryError)` and
`Runtime(RuntimeError)`. Request and description validation fails when the
value is built; `open` returns only `Busy` and `Runtime`.

## 3. Session rules

1. **Opening has one commit point.** `open` first reserves the session by
   opening the private dialog controller. That call is the busy check and the
   phase check, and when it refuses, nothing has changed. Only then does
   `open` write the session's configuration and navigate the browser to the
   starting folder, which hands the host its listing request. Those writes
   are subject to the same phase rule the reservation already passed. If one
   of them fails all the same (request generations exhausted, a reentrant
   borrow), the session ends as `Failed(Start(error))`, the error is also
   returned, and nothing is rolled back. The request callback runs inside
   `open` and, as the browser already requires, must not call the controller.
2. **One outcome.** A session ends exactly once and the first terminal
   transition wins: an accept followed by removal of the owner in the same
   input stays `Accepted`.
3. **The outcome and its payload are written and taken together.** Accepting
   stores the selection and then finishes the session; if finishing is
   refused, the stored selection is discarded, the error is returned and the
   session stays open. `take_outcome` releases the selection only after the
   private controller's outcome has been taken, so a refused take leaves both
   in place.
4. **Retention.** The outcome belongs to the controller, not to a mounted
   component, and stays until `take_outcome`. Reading it needs no callback
   and no notification, so removing the dialog cannot lose it.
5. **Busy.** `open` returns `FileDialogError::Busy` during a session, before
   its outcome is taken, and after a finish until the mounted owner has
   observed the closure in an update. With an owner that is logically
   hidden, and so not updated, that can last until it is shown or removed.
   Without a mounted owner there is nothing to wait for.
6. **No owner.** A session opened while no `FileDialog` uses the controller
   is reserved and its listing is requested; it shows nothing until an owner
   mounts or `cancel` ends it.
7. **Owners.** Removing the owner of an open session ends it as
   `Dismissed(OwnerRemoved)`, also before it reached layout. A second mounted
   owner stays closed and fails its updates with `DialogBusy` until the first
   unmounts, and removing or hiding that second owner does not touch the
   first one's session. That last clause is false today (section 1); slice 1
   adds the ownership check to `Dialog::dismiss_surface` with a regression
   test before it builds on `Dialog`.
8. **Visibility.** Logically hiding the owner or an ancestor of a presented
   session ends it as `Dismissed`. A natively hidden window (minimized,
   occluded, zero-sized) suspends the session and shows it again later.
   Native focus loss leaves it open.
9. **Admission.** A modal conflict or lack of space ends the session as
   `Failed(Surface(reason))`; the flush that discovers a conflict may return
   its error as well, as it does for `Dialog`.
10. **Shutdown.** A mounted owner is removed during shutdown, so its open
    session is dismissed, but once the runtime is closed `take_outcome` and
    reads of `status()` return `RuntimeError::Closed`. No outcome is readable
    after shutdown. An application that must act on an accept takes the
    outcome before it shuts down.
11. **Late listings.** `deliver` applies a response only while it answers
    the latest request of this controller for its directory and the browser
    still wants that directory; otherwise it is `Stale`. A response that
    arrives after its session ended, or during a later session, is ordinary
    browser data under that rule. It never ends, accepts or moves a session.
    Anything the dialog starts that must not outlive its session (the
    starting-folder search of slice 2, the new-folder continuation of slice
    4) carries the session's number and is ignored when it does not match.
12. **Listings between sessions.** The browser keeps the listings it has
    received. Every session navigates to its starting folder, which requests
    it again; a kept listing, loaded or failed, is shown until the fresh one
    arrives.

## 4. What each mode accepts

### 4.1 Terms

- *C* is the current folder. The *listing* is C's unfiltered listing:
  loading, failed or loaded.
- The *shown entries* are the loaded listing's entries that pass the type
  filter and the text filter, in display order. Placeholder rows are not
  entries.
- The *active entry* is the shown entry whose key is the entry list's active
  row, if there is one. The *selected entries* are the shown entries whose
  keys are in the selected set, in display order. A key that is hidden by a
  filter, or that belongs to a placeholder row, is ignored by both.
- An entry is a *folder* when it opens as a directory (a directory or a link
  to one), a *file* when it is a regular file or a link to one and carries
  no error, and otherwise *unusable* (a broken link, a link to something
  else, `EntryKind::Other`, an entry with an error).
- *Refuse* means: the dialog's status line says why, the session continues,
  and no outcome is written. *Unavailable* means: the confirm button is
  disabled and `accept()` does nothing.

Two events ask for acceptance. *Confirm* is the confirm button, `accept()`
or Enter in the name field. *Activation* is Enter or a double click on an
entry that does not open as a directory, reported by the browser with that
entry; activating a folder navigates inside the browser and never reaches
the dialog.

### 4.2 Confirm

Evaluated in this order; the first rule that applies decides.

1. The session is not open: nothing happens.
2. The replace question is showing: `Accepted` with its candidate (4.4).
3. The listing is loading or failed: unavailable, in every mode.
4. By mode:

| Mode | Looks at | Result |
|---|---|---|
| `OpenFile` | the active entry | None: unavailable. A file: `Accepted` with its path. A folder: navigate into it. Unusable: refuse. |
| `OpenFiles` | the selected entries | None: unavailable. All files: `Accepted` with their paths in display order. Exactly one, a folder: navigate into it. Anything else: refuse. |
| `ChooseFolder` | the active entry | A folder: `Accepted` with its path. None: `Accepted` with C. |
| `SaveFile` | the name field | Section 4.4. The active entry plays no part. |

In `OpenFiles` the active entry alone is not a selection: Control-arrow can
move it onto a row without selecting it, and confirm then acts on the
selected entries only.

### 4.3 Activation of an entry E

| Mode | Result |
|---|---|
| `OpenFile` | E is a file: `Accepted` with E's path. Unusable: refuse. |
| `OpenFiles` | E is a file: `Accepted` with E alone, whatever else is selected. Unusable: refuse. |
| `ChooseFolder` | Cannot occur; the list shows folders only (4.5). |
| `SaveFile` | E is a file: E becomes the name field's origin (4.4) and confirm runs. Unusable: refuse. |

### 4.4 `SaveFile`

The entry list shows what the type filter passes, for orientation. *N* is
the name field's text.

**Origin.** Making a file the active entry copies its name, as displayed,
into the name field and records that entry as the field's *origin*. The
origin is dropped as soon as the user edits the field or the folder changes.
While the origin stands, the candidate is that entry's own path, untouched,
so a listed file whose name is not valid Unicode, or one of two names that
render alike, is reported exactly.

**The name rule.** A typed N is a valid name when it is not empty,
`Path::new(N).components()` yields exactly one `Component::Normal`, and it
contains no NUL, no `/`, and on Windows no `\` or `:`. On Unix `\` is an
ordinary character. This refuses `..`, `.`, `a/b`, absolute paths and
Windows' `C:foo`. Device names, trailing dots and other names a filesystem
may reject are left to the application, which learns of them when it writes.

**Confirm in `SaveFile`**, after rules 1 to 3 of section 4.2:

1. N is empty: unavailable.
2. An origin stands: the candidate is the origin's path; go to step 6 with
   that entry.
3. N is not a valid name: refuse.
4. N is exactly the name of one folder in the listing: navigate into it and
   clear N.
5. Resolve the name. With `append_extension` on, a selected filter that has
   extensions, and `Path::new(N).extension()` being `None`, the name is N, a
   dot and the filter's first extension; otherwise it is N. Then look the
   name up among the unfiltered listing's entries by exact `OsStr` equality
   of their names:
   - no entry: `Accepted` with C joined with the name;
   - more than one entry (a host may list two paths with one name): refuse
     as ambiguous;
   - one entry: step 6 with that entry.
6. With the entry: a folder, navigate into it; unusable, refuse; a file, ask
   the replace question with the entry's own path as candidate.

Extension examples with the selected filter `txt`: `report` becomes
`report.txt`; `.env` becomes `.env.txt`, because Rust reports no extension
for it; `report.` stays as typed, because its extension is the empty string;
`a.tar.gz` and `photo.png` stay as typed. A name whose extension the filter
does not list is allowed.

**The replace question** is asked inside the dialog, not in a nested alert:
the footer shows the question with the candidate's name, and the buttons
become Replace and Back. The candidate is fixed when the question appears
and is not recomputed. Replace is confirm (4.2, rule 2) and ends the session
as `Accepted` in the same event as the click; Back returns to editing. While
the question shows, the browser and the name field do not take input. A
navigation or a listing delivered meanwhile cannot change the candidate.
Escape cancels the session, as everywhere.

A nested `AlertDialog` was rejected: its affirmative button accepts only the
alert's own controller, so the file session could be committed only by a
later observation that removal of the owner would cancel.

The listing can be stale either way. A file may have appeared without the
dialog knowing, or vanished after the user confirmed replacing it. The
outcome says what the user chose, not what the filesystem holds.

### 4.5 `ChooseFolder`

The entry list shows folders only, so the active entry, when there is one,
is a folder. The confirm button's default caption names the folder it will
accept, from a string-table message with a placeholder for the name: the
active folder, or C. A request's `accept_label` replaces that caption and is
shown as given.

### 4.6 What happens to the dialog's own state

| Event | Text filter and selection | Name field | Status line | Replace question |
|---|---|---|---|---|
| Navigation to another folder, by any route | Cleared (browser) | Kept; origin dropped | Cleared | Cannot occur while it shows |
| Navigation to the same folder, refresh | Kept | Kept | Kept | Kept |
| Listing delivered for C | Keys the listing lacks leave the selection (browser) | Kept; origin dropped if its entry is gone | Kept | Kept, candidate unchanged |
| Text filter or type filter changed | Selection kept; hidden keys ignored (4.1) | Kept | Cleared | Cannot occur while it shows |
| Selection changed by the user | — | `SaveFile`: an active file sets name and origin | Cleared | Cannot occur while it shows |
| Edit of the name field | Kept | Origin dropped | Cleared | Cannot occur while it shows |

Routes of navigation are the folder tree, the breadcrumb, activating a
folder, confirm on a folder, and Backspace while focus is inside the
browser. Backspace elsewhere in the dialog does not navigate.

### 4.7 Path identity

An accepted path of a listed entry is the `PathBuf` the listing delivered,
unchanged. Paths are not normalized, resolved or rebuilt from display text.
Only a new save target, which no entry names, is built by joining C and the
typed name.

## 5. Additions to the file browser and the list selection

The dialog is composed from public API only. These additions are general;
each names what it makes possible beyond the dialog.

| Addition | Purpose | Beyond the dialog |
|---|---|---|
| `FileBrowserController::new_idle(ui, selection, roots, request)` | As `new`, without requesting the first root. The first `navigate` or `refresh` starts it. | A browser built before it is shown. `new` stays the way to start listing at once. |
| `set_entry_predicate(impl Fn(&DirectoryEntry) -> bool + 'static)` and `clear_entry_predicate()` | Hides entries of the entry list that fail the predicate; the folder tree is unaffected. | A browser that shows only the files its application can open. The text filter matches names only. |
| `listing() -> ReadSignal<CurrentListing>` | The current folder's state: `Loading`, `Loaded(entries)` in display order and unfiltered, or `Failed(ListingError)`. | Status bars, entry counts, enabling actions when a folder is ready. |
| `shown_entries() -> ReadSignal<ShownEntries>` | The entries the list shows, after predicate and text filter, in display order; empty while loading or failed. | Reading a selection in the order the user sees it. `selected_paths` keeps its order and its documentation says what that order is. |
| `ListSelection::set_mode(SelectionMode) -> Result<(), RuntimeError>` | Changes the mode for every clone. | A list with a "select several" toggle. |

**Idle controller.** Until its first `navigate` or `refresh`, an idle
controller has issued no request and `listing()` reports `Loading` for the
first root. A mounted browser asks for a folder only when its tree shows that
folder expanded; a closed dialog's tree is hidden and the first root is not
expanded, so mounting an idle browser requests nothing (assumed, section 11).

**Predicate.** It is stored by the controller, as its other callbacks are;
installing or clearing one is a signal-phase write that advances a revision
the entry derivation reads, so replacing `txt` by `png` in the same folder
and query rebuilds the rows. The predicate must be pure: its answer may
depend on the entry alone, and a captured value that changes does not
refresh the list until the predicate is installed again. It is evaluated
outside the controller's internal borrows and must not call the controller.
The rows the list shows and `shown_entries` come from one evaluation.

**Signal payloads.** `CurrentListing::Loaded` and `ShownEntries` hold shared
slices and compare by allocation, as the controller's listings already do,
so a delivery for another folder neither copies nor re-compares the current
folder's entries, and the current list source and its row revision are kept.

**`ListSelection::set_mode`.** The mode moves into state shared by the
clones, and `mode()` stops being `const`.

- The same mode: nothing happens.
- `Multiple` to `Single`: the selected set becomes the active item alone, or
  empty without one, and the range anchor becomes the active item. If that
  changes the set, it is an ordinary selection write with
  `ChangeOrigin::Application`, reported by every collection sharing the
  selection at its next update.
- `Single` to `Multiple`: the set is unchanged; the range anchor is the
  active item.
- A mode change that alters no key is not a selection change and is not
  reported; the application that called it knows.
- The phase check comes first: refused during update, layout or paint with
  every field intact.
- Gestures read the mode when they are interpreted. A press that a
  collection has deferred to release, or an armed reorder, is interpreted
  under the mode current at that moment. The dialog calls `set_mode` only in
  `open`, before the session is presented, so it never meets one; the general
  rule is written down and tested in slice 1 (section 11).
- `FileBrowserController::selected_paths` and its pruning read the shared
  mode at each call.

Alternatives considered for the selection mode, since the request decides
the mode per session and the browser controller keeps one selection:

- A browser controller per session. Rejected: `FileBrowser` keeps its
  controller for life, so the browser would be remounted under a new key each
  session. The tree's expansion and the received listings belong to the
  controller and would be lost; the split position belongs to the mounted
  pane and would be lost with it.
- Two browser controllers, one per mode. Rejected: two listing caches and two
  trees for one dialog.
- Fixing single or multiple selection on `FileDialogController::new`.
  Rejected: it contradicts decision 6, where the request carries the mode.

A hand-built picker (a `Dialog` holding a `FileBrowser`, a name field, a
`Select::bound` over its filters and buttons whose handlers finish its
`DialogController`) uses the same additions. Slice 1's change rationale
names that composition as the advanced path `FileDialog` makes ordinary, and
one facade-only test builds it and consumes its outcome after the widget is
gone.

## 6. Layout and size

```
bar        breadcrumb and text filter (the browser's own)
panes      folder tree | entry list
footer     name field (SaveFile), filter selector; or the replace question
actions    status line ......................... cancel, confirm
```

- Places are added by slice 2 as the dialog's own list beside the browser.
- The filter selector is a `Select::bound` over the session's filter list,
  which the controller holds in a signal, since the list changes per session.
- The content is a component of the dialog's own with hard limits: its
  minimum and maximum size both equal the configured size. A preferred size
  would not do, because the dialog's scroll area may arrange its body
  narrower than preferred. The content arranges bar, footer and actions at
  their natural heights and gives the browser the remaining height, which is
  the bounded height it needs. The configured size is the content's;
  `Dialog` adds its body padding and margin around it.
- When the window, or for `in_view` the owner's region, is smaller than the
  content plus that padding and margin, the dialog's scroll area scrolls the
  content; it is never squeezed below its size. Slice 1 tests this case by
  keyboard and asserts the browser's allocation, the scroll extent and that
  Tab to the confirm button brings it into view.
- The in-window dialog is not resizable by the user; `Dialog` has no resize
  affordance, and adding one is not part of this plan.
- The texts the dialog adds (mode titles, captions, the name-field label,
  refusal messages, the replace question, with named placeholders for a
  folder or file name) are new `StringRole`s present in every string table,
  the gallery's second language included.

## 7. Workspace

The in-window dialog's content is mounted while the dialog is closed, and
workspace capture includes mounted views whether or not they are visible. The
browser's addressable parts therefore take part under the owner's path and
the `dialog` segment in every state of the dialog: the split between tree and
list, and what the browser's region layout and scrolling parts already
record. The dialog adds no workspace state of its own, and session data (the
current folder, the selection, the name field, the selected filter, the
status line) is never captured; from slice 2 the durable part of it belongs
to `FileDialogMemory`. Slice 1 tests capture before the first session, restore
while closed, and that the split position is in effect when the dialog then
opens.

## 8. Tests for slice 1

A module of the `surfaces` area binary in `crates/pert-ui/tests`, using the
public facade. One test per numbered rule of section 3, per rule and row of
sections 4.2 to 4.6, and in addition:

- the `Dialog` repair of rule 7: removing, and hiding, the losing owner;
- `open` refused as busy in each of its three states changes nothing and
  requests nothing; `open` whose navigation fails ends as `Failed(Start)`;
- accept and `take_outcome` with a refused write keep the payload and the
  status together;
- request, filter and description validation, each rejection;
- filter matching: case, `tar.gz`, a name that is only an extension, and on
  Unix a name that is not valid Unicode;
- the extension examples of section 4.4 and the name rule per platform;
- a save over a listed file whose name is not valid Unicode returns the
  listed path (Unix);
- a placeholder row made active or selected, then confirm;
- `OpenFiles`: display order after selecting in another order; a selected
  entry hidden by the text filter; Control-arrow onto an unusable row with
  files selected;
- a listing delivered after the session ended, and during the next one;
- redescription during a session; a description with another controller
  under the same key keeps the mounted one;
- the small-window case of section 6, for `in_window` and `in_view`;
- in the `collections` binary: `ListSelection::set_mode` in both directions
  with clones, a mounted `VirtualList`, `Tree` and `Table`, the reported
  origin, a deferred press, and browser delivery and pruning after a switch;
  the idle controller; predicate replacement in one folder; a delivery for
  another folder keeping the current source and row revision;
- the hand-built composition of section 5;
- workspace capture and restore (section 7).

## 9. Where this note narrows or corrects the plan

For the maintainer to accept or reject with the note. Items 1 and 5 reduce
what the plan promised and need an explicit answer.

1. **No standalone content element in slice 1.** The plan offered the
   picker's content as an ordinary panel. A session without a modal surface
   needs an ownership rule of its own. The browser additions make a
   hand-built panel possible; a ready-made one can follow if it is wanted.
2. **Several files are reported in display order.** The plan's first text
   said the browser's key order, which compares numbered keys as text.
3. **The name field exists only in `SaveFile` and takes a name, not a
   path.** Typing a folder's name navigates into it; typing an absolute or
   relative path is refused.
4. **`ChooseFolder` lists folders only.**
5. **The in-window dialog has a fixed size the application sets.** The
   plan's Memory section keeps a remembered size for the in-window
   presentation too. Under this note only the own window's size is
   remembered, from slice 3b; decision 2 (the size lives in
   `FileDialogMemory`) still holds for it. If the maintainer wants the
   in-window size remembered, it needs a resize affordance on `Dialog`
   first, and an explicit `FileDialog::size` would then take precedence over
   the remembered one.
6. **The outcome carries the selected filter's identity.**
7. **`ListSelection::set_mode`** changes a shared type the plan did not name.
8. **The replace question is asked inside the dialog**, not in an
   `AlertDialog` as the plan said.
9. **The modal scope is chosen by constructor** (`in_window`, `in_view`),
   where the plan had a `view_modal` builder.
10. **A repair of `Dialog`** (section 3, rule 7) joins slice 1.

## 10. Rationale for the new API

`FileDialog` offers a new way to do something already possible: an
application can compose `Dialog` and `FileBrowser` itself. It makes that
composition the advanced path and the ordinary case one call, with the
acceptance rules, the save name handling and the retained outcome written
once. It retires nothing.

## 11. Assumed, not yet tested

- That an idle browser mounted in a closed dialog requests nothing: the
  tree records a folder as wanted only while it has room to show it. Read
  from the controller's tests; not run for a closed dialog.
- That a content component with equal minimum and maximum limits is laid out
  by `Dialog`'s panel and scroll area as section 6 describes. Read from
  `DialogPanel::arrange` and the scroll area's measurement; not run.
- What a session opened under an already hidden ancestor does. `Dialog`
  dismisses on a visible-to-hidden transition; with none, the session may
  stay open and unpresented. Slice 1 pins it with a test and adds the row to
  section 3.
- How `VirtualList`, `Tree` and `Table` treat a mode change during a
  deferred press or an armed reorder. Slice 1 reads those paths and writes
  the rule in `set_mode`'s documentation.
- That every write `open` makes after the reservation is subject to exactly
  the phase rule the reservation passed.
- Names (`new_idle`, `set_entry_predicate`, `CurrentListing`,
  `ShownEntries`, `in_view`) are placeholders until slice 1's API review.
- The own window is outlined only as far as section 2.5 needs. Its protocol
  is slice 3a's design note.

## 12. Review record

Codex, 2026-11-19, `codex exec -s read-only` on the first draft; static
reading only. The blockers and the layout, cache and selection findings were
checked against the cited code before the note changed.

| Finding | Answer |
|---|---|
| Blocker: `open` promised a rollback it cannot give | Section 3, rule 1: reservation first, `Failed(Start)` after it |
| Blocker: removing a losing owner ends the winner's session | Section 1, section 3 rule 7, TODO.md; repaired in slice 1 |
| Blocker: a nested alert does not commit the file outcome | Section 4.4: the question is asked inside the dialog; rule 3 for the payload |
| Blocker: save could accept an unusable entry or change a path | Section 4.4: origin, exact lookup, ambiguity, kinds |
| Ownership, visibility and admission run together | Section 3, rules 5 to 10 |
| Controller and row height on redescription | Section 2.5: kept for life |
| Overlapping confirm rows | Sections 4.2 and 4.3: ordered rules, one table each |
| Placeholders, pruning, filters, Backspace, late deliveries | Sections 1, 3 rule 11, 4.1, 4.6 |
| Native names, separators, extension cases | Section 4.4 |
| No complete construction path; browser errors leaked | Sections 2.1, 2.6, 6 |
| Captions, bounds, minimum size | Sections 2.2, 2.5, 4.5 |
| `set_mode` changes a shared contract | Section 5 and its tests |
| Predicate without invalidation or evaluation rules | Section 5 |
| A preferred size does not fix the layout | Section 6 |
| Workspace claim too broad | Section 7 |
| Scope of settled decisions | Section 2.5 constructors, section 9 items 1 and 5 |
