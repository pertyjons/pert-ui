//! The in-app file browser: requests with generations and stale results,
//! loading and error rows, identity by path (including paths that are not
//! UTF-8), symbolic links, activation and navigation, and the optional
//! `std::fs` helper.
use crate::support;
use pert_ui::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};
use support::{press, settle, widget};

struct Stage {
    ui: Runtime,
    controller: FileBrowserController,
    requests: Rc<RefCell<Vec<DirectoryRequest>>>,
    opened: Rc<RefCell<Vec<PathBuf>>>,
    browser: WidgetId,
}

fn dir(path: &str) -> DirectoryEntry {
    DirectoryEntry::new(path, EntryKind::Directory)
}
fn file(path: &str) -> DirectoryEntry {
    DirectoryEntry::new(path, EntryKind::File)
}

fn stage(mode: SelectionMode) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let controller =
        FileBrowserController::new(&ui, ["/r"], ListSelection::new(&ui, mode), move |request| {
            queue.borrow_mut().push(request)
        })
        .unwrap();
    let opened = Rc::new(RefCell::new(Vec::new()));
    let sink = opened.clone();
    controller.on_open(move |_, entry| {
        sink.borrow_mut().push(entry.path().to_owned());
        Ok(())
    });
    let browser = ui
        .mount_tree(Element::new(
            "browser",
            FileBrowser::new(&controller, 24.0).unwrap(),
        ))
        .unwrap();
    settle(&mut ui);
    Stage {
        ui,
        controller,
        requests,
        opened,
        browser,
    }
}

impl Stage {
    /// Removes and returns the latest request for `directory`.
    #[track_caller]
    fn take(&self, directory: &str) -> DirectoryRequest {
        let mut requests = self.requests.borrow_mut();
        let index = requests
            .iter()
            .rposition(|request| request.directory() == Path::new(directory))
            .unwrap_or_else(|| panic!("no request for {directory}: {requests:?}"));
        requests.remove(index)
    }
    fn requested(&self) -> Vec<PathBuf> {
        self.requests
            .borrow()
            .iter()
            .map(|request| request.directory().to_owned())
            .collect()
    }
    #[track_caller]
    fn answer(&mut self, directory: &str, entries: Vec<DirectoryEntry>) -> ListingDelivery {
        let request = self.take(directory);
        let outcome = self
            .controller
            .deliver(request.respond(Ok(entries)))
            .unwrap();
        settle(&mut self.ui);
        outcome
    }
    fn keys(&self, parent: &str) -> Vec<String> {
        let parent = widget(&self.ui, &self.browser, parent);
        self.ui
            .children(&parent)
            .unwrap()
            .iter()
            .map(|child| self.ui.key(child).unwrap().to_owned())
            .collect()
    }
    /// The entry list's rows as paths, or the placeholder key for a row
    /// without one.
    fn entries(&self) -> Vec<String> {
        self.keys("entries")
            .into_iter()
            .map(|key| match self.controller.path_of(&key) {
                Some(path) => path.to_string_lossy().into_owned(),
                None => key,
            })
            .collect()
    }
    fn folders(&self) -> Vec<String> {
        self.keys("folders")
            .into_iter()
            .map(|key| match self.controller.path_of(&key) {
                Some(path) => path.to_string_lossy().into_owned(),
                None => key.split(':').next().unwrap_or_default().to_owned(),
            })
            .collect()
    }
    fn current(&self) -> PathBuf {
        self.controller.current().get().unwrap()
    }
    /// The widget of the row showing `path` in the list or tree `parent`.
    fn row(&self, parent: &str, path: &str) -> WidgetId {
        let parent = widget(&self.ui, &self.browser, parent);
        self.ui
            .children(&parent)
            .unwrap()
            .into_iter()
            .find(|child| {
                self.controller.path_of(self.ui.key(child).unwrap()) == Some(PathBuf::from(path))
            })
            .unwrap_or_else(|| panic!("no row for {path}"))
    }
    fn click(&mut self, id: &WidgetId) {
        let bounds = self.ui.bounds(id).unwrap().unwrap();
        let point = Point::new(bounds.x() + 40.0, bounds.y() + bounds.height() / 2.0).unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(point, kind))
                .unwrap();
        }
        settle(&mut self.ui);
    }
    fn advance(&mut self, by: Duration) {
        let now = self.ui.now() + by;
        self.ui.flush(now).unwrap();
    }
    fn property(&self, key: &str, name: &str) -> Option<InspectValue> {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get(name).cloned())
    }
}

#[test]
fn the_browser_requests_its_folder_and_shows_loading_until_the_listing_arrives() {
    let mut stage = stage(SelectionMode::Single);
    assert_eq!(stage.requested(), [PathBuf::from("/r")]);
    assert_eq!(stage.entries(), ["loading"]);
    let outcome = stage.answer(
        "/r",
        vec![
            file("/r/b.wav"),
            dir("/r/Zeta"),
            file("/r/A.wav"),
            dir("/r/alpha"),
        ],
    );
    assert_eq!(outcome, ListingDelivery::Applied);
    // Directories first, then by name ignoring case.
    assert_eq!(
        stage.entries(),
        ["/r/alpha", "/r/Zeta", "/r/A.wav", "/r/b.wav"]
    );
    let semantics = stage
        .ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "browser")
        .and_then(|component| component.details.semantics)
        .unwrap();
    assert_eq!(semantics.role, SemanticRole::Group);
    assert_eq!(semantics.value, Some(SemanticValue::Text("/r".into())));
    assert_eq!(
        stage.ui.next_deadline(),
        None,
        "the browser schedules nothing"
    );
}

#[test]
fn a_result_for_a_folder_the_user_left_is_dropped_as_stale() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a"), dir("/r/b")]);
    stage.controller.navigate("/r/a").unwrap();
    settle(&mut stage.ui);
    let late = stage.take("/r/a");
    stage.controller.navigate("/r/b").unwrap();
    settle(&mut stage.ui);
    let outcome = stage
        .controller
        .deliver(late.respond(Ok(vec![file("/r/a/old.wav")])))
        .unwrap();
    assert_eq!(outcome, ListingDelivery::Stale);
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["loading"], "/r/b still loads");
    assert_eq!(
        stage.answer("/r/b", vec![file("/r/b/new.wav")]),
        ListingDelivery::Applied
    );
    assert_eq!(stage.entries(), ["/r/b/new.wav"]);
}

#[test]
fn a_newer_request_supersedes_an_older_one() {
    let mut stage = stage(SelectionMode::Single);
    let first = stage.take("/r");
    stage.controller.refresh().unwrap();
    let second = stage.take("/r");
    assert!(second.generation() > first.generation());
    let older = stage
        .controller
        .deliver(first.respond(Ok(vec![file("/r/old")])))
        .unwrap();
    assert_eq!(older, ListingDelivery::Stale);
    let newer = stage
        .controller
        .deliver(second.clone().respond(Ok(vec![file("/r/new")])))
        .unwrap();
    assert_eq!(newer, ListingDelivery::Applied);
    // Delivering the same answer twice applies it once.
    let again = stage
        .controller
        .deliver(second.respond(Ok(vec![file("/r/other")])))
        .unwrap();
    assert_eq!(again, ListingDelivery::Stale);
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["/r/new"]);
}

#[test]
fn a_failed_listing_is_an_error_row_not_an_empty_folder() {
    let mut stage = stage(SelectionMode::Single);
    let request = stage.take("/r");
    let error = ListingError::new(ListingErrorKind::PermissionDenied, "permission denied");
    stage
        .controller
        .deliver(request.respond(Err(error)))
        .unwrap();
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["error"]);
    let label = stage
        .ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "error")
        .and_then(|component| component.details.semantics)
        .unwrap();
    assert_eq!(label.name.as_deref(), Some("permission denied"));
    // An empty listing is a distinct, explained state.
    stage.controller.refresh().unwrap();
    stage.answer("/r", vec![]);
    assert_eq!(stage.entries(), ["empty"]);
}

#[test]
fn expanding_an_unlisted_folder_shows_a_loading_row_and_requests_it() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a"), file("/r/f.wav")]);
    // The tree's active node is the root; expand it and then its folder.
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.folders(), ["/r", "/r/a"], "files are not folders");
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.folders(), ["/r", "/r/a", "loading"]);
    assert_eq!(stage.requested(), [PathBuf::from("/r/a")]);
    stage.answer("/r/a", vec![dir("/r/a/x"), dir("/r/a/y")]);
    assert_eq!(stage.folders(), ["/r", "/r/a", "/r/a/x", "/r/a/y"]);
    // A folder known to have no subfolders cannot expand; one not yet listed
    // can, without listing it first.
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.requested(), [PathBuf::from("/r/a/x")]);
    stage.answer("/r/a/x", vec![file("/r/a/x/only.wav")]);
    assert_eq!(stage.folders(), ["/r", "/r/a", "/r/a/x", "/r/a/y"]);
}

#[test]
fn collapsing_a_loading_folder_makes_its_result_stale() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a")]);
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    let request = stage.take("/r/a");
    press(&mut stage.ui, KeyCode::ArrowLeft);
    let outcome = stage
        .controller
        .deliver(request.respond(Ok(vec![dir("/r/a/x")])))
        .unwrap();
    assert_eq!(outcome, ListingDelivery::Stale);
    // Expanding again asks again.
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.requested(), [PathBuf::from("/r/a")]);
}

#[test]
fn a_failed_folder_listing_is_an_error_row_under_its_node() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/locked")]);
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    let request = stage.take("/r/locked");
    stage
        .controller
        .deliver(request.respond(Err(ListingError::new(
            ListingErrorKind::PermissionDenied,
            "denied",
        ))))
        .unwrap();
    settle(&mut stage.ui);
    assert_eq!(stage.folders(), ["/r", "/r/locked", "error"]);
}

#[test]
fn selection_is_kept_by_path_when_a_listing_changes() {
    let mut stage = stage(SelectionMode::Multiple);
    stage.answer("/r", vec![file("/r/b"), file("/r/c")]);
    let keys: Vec<String> = stage.keys("entries");
    stage.controller.selection().select(keys.clone()).unwrap();
    stage.controller.refresh().unwrap();
    stage.answer("/r", vec![file("/r/a"), file("/r/c"), file("/r/b")]);
    assert_eq!(stage.entries(), ["/r/a", "/r/b", "/r/c"]);
    let mut selected = stage.controller.selected_paths().unwrap();
    selected.sort();
    assert_eq!(selected, [PathBuf::from("/r/b"), PathBuf::from("/r/c")]);
    // The rows that kept their path kept their widgets.
    assert_eq!(
        stage.keys("entries")[1..],
        [keys[0].clone(), keys[1].clone()]
    );
}

#[cfg(unix)]
#[test]
fn paths_that_are_not_utf8_keep_their_own_identity() {
    use std::os::unix::ffi::OsStrExt;
    let mut stage = stage(SelectionMode::Multiple);
    let first = Path::new("/r").join(std::ffi::OsStr::from_bytes(b"take\xff"));
    let second = Path::new("/r").join(std::ffi::OsStr::from_bytes(b"take\xfe"));
    assert_eq!(first.to_string_lossy(), second.to_string_lossy());
    let request = stage.take("/r");
    stage
        .controller
        .deliver(request.respond(Ok(vec![
            DirectoryEntry::new(&first, EntryKind::File),
            DirectoryEntry::new(&second, EntryKind::File),
        ])))
        .unwrap();
    settle(&mut stage.ui);
    let keys = stage.keys("entries");
    assert_eq!(keys.len(), 2, "two rows, though their names look the same");
    stage.controller.selection().select(keys).unwrap();
    let mut selected = stage.controller.selected_paths().unwrap();
    selected.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(selected, expected, "paths come back exactly");
}

#[test]
fn enter_opens_directories_and_links_to_them_and_hands_files_to_the_application() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer(
        "/r",
        vec![
            dir("/r/a"),
            DirectoryEntry::new("/r/shortcut", EntryKind::Symlink(LinkTarget::Directory)),
            file("/r/song.wav"),
        ],
    );
    // Links are not folders in the tree, so it never recurses through one.
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.folders(), ["/r", "/r/a"]);

    let list = widget(&stage.ui, &stage.browser, "entries");
    stage.ui.focus(&list).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::End);
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(*stage.opened.borrow(), [PathBuf::from("/r/song.wav")]);
    assert_eq!(stage.current(), Path::new("/r"));
    press(&mut stage.ui, KeyCode::ArrowUp);
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(stage.current(), Path::new("/r/shortcut"));
    assert_eq!(stage.entries(), ["loading"]);
}

#[test]
fn a_double_click_activates_an_entry_and_a_single_click_only_selects() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a"), file("/r/song.wav")]);
    let song = stage.row("entries", "/r/song.wav");
    stage.click(&song);
    assert!(stage.opened.borrow().is_empty());
    assert_eq!(
        stage.controller.selected_paths().unwrap(),
        [PathBuf::from("/r/song.wav")]
    );
    // Too slow for a double click.
    stage.advance(Duration::from_secs(1));
    stage.click(&song);
    assert!(stage.opened.borrow().is_empty());
    stage.click(&song);
    assert_eq!(*stage.opened.borrow(), [PathBuf::from("/r/song.wav")]);

    let folder = stage.row("entries", "/r/a");
    stage.advance(Duration::from_secs(1));
    stage.click(&folder);
    stage.click(&folder);
    assert_eq!(stage.current(), Path::new("/r/a"));
    assert!(
        stage.controller.selected_paths().unwrap().is_empty(),
        "navigating clears the selection"
    );
}

#[test]
fn four_quick_presses_open_twice_and_a_distant_second_press_does_not() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![file("/r/song.wav")]);
    let song = stage.row("entries", "/r/song.wav");
    for _ in 0..4 {
        stage.click(&song);
    }
    assert_eq!(stage.opened.borrow().len(), 2, "two double clicks");
    // A second press five pixels away is another single click.
    stage.advance(Duration::from_secs(1));
    let bounds = stage.ui.bounds(&song).unwrap().unwrap();
    for x in [40.0, 45.0] {
        let point = Point::new(bounds.x() + x, bounds.y() + bounds.height() / 2.0).unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            stage
                .ui
                .pointer_input(PointerEvent::new(point, kind))
                .unwrap();
        }
    }
    assert_eq!(stage.opened.borrow().len(), 2);
}

#[test]
fn clicking_a_folder_in_the_tree_opens_it() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a"), dir("/r/b")]);
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    let b = stage.row("folders", "/r/b");
    stage.click(&b);
    assert_eq!(stage.current(), Path::new("/r/b"));
    // Enter on the active folder opens it too.
    press(&mut stage.ui, KeyCode::ArrowUp);
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(stage.current(), Path::new("/r/a"));
}

#[test]
fn navigating_expands_the_tree_down_to_the_folder_and_the_breadcrumb_follows() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a")]);
    stage.controller.navigate("/r/a/b").unwrap();
    settle(&mut stage.ui);
    // The tree asks for /r/a, which it now shows expanded, and the entry list
    // for /r/a/b.
    let mut requested = stage.requested();
    requested.sort();
    assert_eq!(requested, [PathBuf::from("/r/a"), PathBuf::from("/r/a/b")]);
    stage.answer("/r/a", vec![dir("/r/a/b")]);
    assert_eq!(stage.folders(), ["/r", "/r/a", "/r/a/b"]);
    let crumbs: Vec<String> = stage
        .keys("crumbs")
        .into_iter()
        .filter_map(|key| stage.controller.path_of(&key))
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    assert_eq!(crumbs, ["/r", "/r/a", "/r/a/b"]);
    // Activating a breadcrumb segment opens that folder.
    let crumbs = widget(&stage.ui, &stage.browser, "crumbs");
    let segment = stage
        .ui
        .children(&crumbs)
        .unwrap()
        .into_iter()
        .find(|child| {
            stage.controller.path_of(stage.ui.key(child).unwrap()) == Some(PathBuf::from("/r"))
        })
        .unwrap();
    stage.ui.focus(&segment).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(stage.current(), Path::new("/r"));
}

#[test]
fn backspace_in_the_list_opens_the_parent_folder() {
    let mut stage = stage(SelectionMode::Single);
    stage.controller.navigate("/r/a").unwrap();
    settle(&mut stage.ui);
    stage.answer("/r/a", vec![file("/r/a/x")]);
    let list = widget(&stage.ui, &stage.browser, "entries");
    stage.ui.focus(&list).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::Backspace);
    assert_eq!(stage.current(), Path::new("/r"));
    // Typed into the filter, Backspace edits instead.
    let filter = widget(&stage.ui, &stage.browser, "filter");
    stage.ui.focus(&filter).unwrap();
    settle(&mut stage.ui);
    stage.ui.text_input("ab").unwrap();
    press(&mut stage.ui, KeyCode::Backspace);
    assert_eq!(stage.current(), Path::new("/r"));
    assert_eq!(stage.controller.filter().get().unwrap(), "a");
}

#[test]
fn the_filter_narrows_the_entries_and_navigating_clears_it() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer(
        "/r",
        vec![dir("/r/Drums"), file("/r/kick.wav"), file("/r/snare.wav")],
    );
    stage.controller.filter().set("  DRU ".into()).unwrap();
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["/r/Drums"]);
    stage.controller.filter().set("zzz".into()).unwrap();
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["empty"]);
    stage.controller.navigate("/r/Drums").unwrap();
    assert_eq!(stage.controller.filter().get().unwrap(), "");
}

#[test]
fn unreadable_entries_are_listed_with_their_error() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer(
        "/r",
        vec![
            DirectoryEntry::unreadable(
                "/r/secret",
                ListingError::new(ListingErrorKind::PermissionDenied, "denied"),
            ),
            file("/r/ok"),
        ],
    );
    assert_eq!(stage.entries(), ["/r/ok", "/r/secret"]);
    let key = stage.keys("entries")[1].clone();
    assert_eq!(
        stage.property(&key, "kind"),
        Some(InspectValue::Text("other".into()))
    );
}

#[test]
fn a_response_for_another_controller_or_a_root_given_twice_is_refused() {
    let mut stage = stage(SelectionMode::Single);
    let ui = Runtime::new();
    let other_requests = Rc::new(RefCell::new(Vec::new()));
    let queue = other_requests.clone();
    let _other = FileBrowserController::new(
        &ui,
        ["/elsewhere"],
        ListSelection::new(&ui, SelectionMode::Single),
        move |request| queue.borrow_mut().push(request),
    )
    .unwrap();
    let foreign: DirectoryRequest = other_requests.borrow_mut().remove(0);
    assert_eq!(
        stage
            .controller
            .deliver(foreign.respond(Ok(vec![])))
            .unwrap(),
        ListingDelivery::Stale
    );
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["loading"]);

    let none: [&str; 0] = [];
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    assert_eq!(
        FileBrowserController::new(&ui, none, selection.clone(), |_| ()).unwrap_err(),
        FileBrowserError::NoRoots
    );
    assert_eq!(
        FileBrowserController::new(&ui, ["/a", "/a"], selection, |_| ()).unwrap_err(),
        FileBrowserError::DuplicateRoot(PathBuf::from("/a"))
    );
}

#[test]
fn a_foreign_response_with_a_matching_directory_and_generation_is_stale() {
    let mut stage = stage(SelectionMode::Single);
    // A second controller over the same root makes its first request for /r
    // with the same generation as the stage's own pending request.
    let other_requests = Rc::new(RefCell::new(Vec::new()));
    let queue = other_requests.clone();
    let _other = FileBrowserController::new(
        &stage.ui,
        ["/r"],
        ListSelection::new(&stage.ui, SelectionMode::Single),
        move |request| queue.borrow_mut().push(request),
    )
    .unwrap();
    let foreign: DirectoryRequest = other_requests.borrow_mut().remove(0);
    let own = stage.take("/r");
    assert_eq!(
        (foreign.directory(), foreign.generation()),
        (own.directory(), own.generation())
    );
    assert_ne!(foreign, own, "requests of different controllers differ");
    assert_eq!(
        stage
            .controller
            .deliver(foreign.respond(Ok(vec![file("/r/intruder")])))
            .unwrap(),
        ListingDelivery::Stale
    );
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["loading"]);
    assert_eq!(
        stage
            .controller
            .deliver(own.respond(Ok(vec![file("/r/mine")])))
            .unwrap(),
        ListingDelivery::Applied
    );
    settle(&mut stage.ui);
    assert_eq!(stage.entries(), ["/r/mine"]);
}

#[test]
fn collapsing_an_ancestor_makes_a_hidden_folders_result_stale() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a")]);
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    // Expand /r, then /r/a, which starts loading.
    press(&mut stage.ui, KeyCode::ArrowRight);
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    let request = stage.take("/r/a");
    // Collapse /r: /r/a stays expanded in the tree's set but is hidden.
    press(&mut stage.ui, KeyCode::ArrowUp);
    press(&mut stage.ui, KeyCode::ArrowLeft);
    assert_eq!(stage.folders(), ["/r"]);
    let outcome = stage
        .controller
        .deliver(request.respond(Ok(vec![dir("/r/a/x")])))
        .unwrap();
    assert_eq!(outcome, ListingDelivery::Stale);
    // Showing /r again asks for /r/a again.
    press(&mut stage.ui, KeyCode::ArrowRight);
    assert_eq!(stage.requested(), [PathBuf::from("/r/a")]);
}

#[test]
fn a_refresh_drops_selected_entries_that_are_gone() {
    let mut stage = stage(SelectionMode::Multiple);
    stage.answer("/r", vec![file("/r/old.wav"), file("/r/kept.wav")]);
    let keys: Vec<String> = stage.keys("entries");
    stage.controller.selection().select(keys).unwrap();
    stage.controller.refresh().unwrap();
    stage.answer("/r", vec![file("/r/kept.wav"), file("/r/new.wav")]);
    assert_eq!(
        stage.controller.selected_paths().unwrap(),
        [PathBuf::from("/r/kept.wav")]
    );

    let mut single = stage_single_with_active("/r/old.wav");
    single.controller.refresh().unwrap();
    single.answer("/r", vec![file("/r/new.wav")]);
    assert!(single.controller.selected_paths().unwrap().is_empty());
    assert_eq!(
        single.controller.selection().active().get().unwrap(),
        None,
        "the active row is gone too"
    );
}

/// A single-selection stage whose active entry is `path` in `/r`.
fn stage_single_with_active(path: &str) -> Stage {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![file(path)]);
    let key = stage.keys("entries")[0].clone();
    stage.controller.selection().set_active(Some(key)).unwrap();
    settle(&mut stage.ui);
    stage
}

#[test]
fn a_retained_entry_row_shows_its_replacement_listing() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![file("/r/item"), file("/r/other")]);
    let row = stage.row("entries", "/r/item");
    // A first press arms a double click on the row.
    stage.click(&row);
    stage.controller.refresh().unwrap();
    stage.answer(
        "/r",
        vec![
            DirectoryEntry::unreadable(
                "/r/item",
                ListingError::new(ListingErrorKind::PermissionDenied, "denied"),
            ),
            file("/r/other"),
        ],
    );
    let row_now = stage.row("entries", "/r/item");
    assert_eq!(row_now, row, "the row keeps its widget");
    let key = stage.ui.key(&row).unwrap().to_owned();
    assert_eq!(
        stage.property(&key, "kind"),
        Some(InspectValue::Text("other".into()))
    );
    let name = stage
        .ui
        .inspect()
        .components
        .into_iter()
        .filter(|component| component.key == "name")
        .filter_map(|component| component.details.semantics.and_then(|s| s.name))
        .find(|name| name.starts_with("item"));
    assert_eq!(name.as_deref(), Some("item — denied"));
    // The press armed for the old entry does not complete a double click on
    // the replacement.
    stage.click(&row);
    assert!(stage.opened.borrow().is_empty());
}

/// Delivers a held response from its update, where signal writes are refused.
struct DeliverInUpdate {
    controller: FileBrowserController,
    trigger: Signal<u32>,
    response: Rc<RefCell<Option<DirectoryResponse>>>,
    outcome: Rc<RefCell<Option<Result<ListingDelivery, RuntimeError>>>>,
}
impl Component for DeliverInUpdate {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read(&self.trigger)?;
        if let Some(response) = self.response.borrow_mut().take() {
            *self.outcome.borrow_mut() = Some(self.controller.deliver(response));
        }
        Ok(())
    }
}

#[test]
fn a_delivery_that_fails_keeps_its_request_pending_for_a_retry() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let controller = FileBrowserController::new(
        &ui,
        ["/r"],
        ListSelection::new(&ui, SelectionMode::Single),
        move |request: DirectoryRequest| queue.borrow_mut().push(request),
    )
    .unwrap();
    let trigger = ui.signal(0_u32);
    let held = Rc::new(RefCell::new(None));
    let outcome = Rc::new(RefCell::new(None));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("browser", FileBrowser::new(&controller, 24.0).unwrap()),
                Element::new(
                    "deliverer",
                    DeliverInUpdate {
                        controller: controller.clone(),
                        trigger: trigger.clone(),
                        response: held.clone(),
                        outcome: outcome.clone(),
                    },
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let request: DirectoryRequest = requests.borrow_mut().remove(0);
    let response = request.respond(Ok(vec![file("/r/a.wav")]));
    *held.borrow_mut() = Some(response.clone());
    trigger.set(1).unwrap();
    settle(&mut ui);
    // The write was refused during the update, and nothing was applied.
    let refused = outcome.borrow_mut().take();
    assert!(
        matches!(refused, Some(Err(RuntimeError::WriteDuringUpdate))),
        "{refused:?}"
    );
    let entries = widget(&ui, &root, "entries");
    let shown: Vec<String> = ui
        .children(&entries)
        .unwrap()
        .iter()
        .map(|child| ui.key(child).unwrap().to_owned())
        .collect();
    assert_eq!(shown, ["loading"]);
    // The same response delivered from application code then applies once.
    assert_eq!(
        controller.deliver(response.clone()).unwrap(),
        ListingDelivery::Applied
    );
    assert_eq!(
        controller.deliver(response).unwrap(),
        ListingDelivery::Stale
    );
}

#[test]
fn unmounting_the_browser_leaves_the_controller_working() {
    let mut stage = stage(SelectionMode::Single);
    stage.ui.unmount(&stage.browser).unwrap();
    settle(&mut stage.ui);
    let request = stage.take("/r");
    assert_eq!(
        stage
            .controller
            .deliver(request.respond(Ok(vec![file("/r/x")])))
            .unwrap(),
        ListingDelivery::Applied
    );
    settle(&mut stage.ui);
    assert_eq!(stage.ui.next_deadline(), None);
}

#[test]
fn list_directory_reports_every_entry_and_links() {
    let base = std::env::temp_dir().join(format!("pert-ui-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("sub")).unwrap();
    std::fs::write(base.join("file.txt"), b"x").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(base.join("sub"), base.join("to-sub")).unwrap();
        std::os::unix::fs::symlink(base.join("missing"), base.join("broken")).unwrap();
        // A link back to its own directory is reported, never followed.
        std::os::unix::fs::symlink(&base, base.join("loop")).unwrap();
    }
    let mut entries = list_directory(&base).unwrap();
    entries.sort_by(|a, b| a.name().cmp(b.name()));
    let kinds: Vec<(String, EntryKind)> = entries
        .iter()
        .map(|entry| (entry.name().to_string_lossy().into_owned(), entry.kind()))
        .collect();
    let mut expected = vec![
        ("file.txt".to_owned(), EntryKind::File),
        ("sub".to_owned(), EntryKind::Directory),
    ];
    #[cfg(unix)]
    expected.extend([
        ("broken".to_owned(), EntryKind::Symlink(LinkTarget::Broken)),
        ("loop".to_owned(), EntryKind::Symlink(LinkTarget::Directory)),
        (
            "to-sub".to_owned(),
            EntryKind::Symlink(LinkTarget::Directory),
        ),
    ]);
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    let mut kinds = kinds;
    kinds.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(kinds, expected);
    assert!(
        entries
            .iter()
            .all(|entry| entry.path().parent() == Some(base.as_path()))
    );
    let missing = list_directory(&base.join("nope")).unwrap_err();
    assert_eq!(missing.kind(), ListingErrorKind::NotFound);
    std::fs::remove_dir_all(&base).unwrap();
}

struct NavigateInUpdate {
    controller: FileBrowserController,
    directory: PathBuf,
}
impl Component for NavigateInUpdate {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        assert_eq!(
            self.controller.navigate(&self.directory),
            Err(RuntimeError::WriteDuringUpdate)
        );
        Ok(())
    }
}
#[test]
fn refused_navigation_neither_queues_nor_dispatches_a_request() {
    for directory in ["/r", "/r/other"] {
        let mut stage = stage(SelectionMode::Single);
        stage.answer("/r", vec![file("/r/a")]);
        stage.controller.filter().set("a".into()).unwrap();
        settle(&mut stage.ui);
        let before = stage.controller.current().get().unwrap();
        let probe = stage
            .ui
            .mount(
                "probe",
                NavigateInUpdate {
                    controller: stage.controller.clone(),
                    directory: PathBuf::from(directory),
                },
            )
            .unwrap();
        settle(&mut stage.ui);
        assert_eq!(stage.controller.current().get().unwrap(), before);
        assert_eq!(stage.controller.filter().get().unwrap(), "a");
        assert!(stage.requests.borrow().is_empty());
        stage.ui.unmount(&probe).unwrap();
        stage.controller.navigate("/r").unwrap();
        assert_eq!(stage.requested(), [PathBuf::from("/r")]);
    }
}

#[test]
fn redescribed_browser_parts_keep_the_mounted_controller() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let original = FileBrowserController::new(
        &ui,
        ["/r"],
        ListSelection::new(&ui, SelectionMode::Single),
        move |request| queue.borrow_mut().push(request),
    )
    .unwrap();
    let other = FileBrowserController::new(
        &ui,
        ["/other"],
        ListSelection::new(&ui, SelectionMode::Single),
        |_| {},
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Container::new(Element::new(
                "browser",
                FileBrowser::new(&original, 24.0).unwrap(),
            )),
        ))
        .unwrap();
    let request = requests.borrow_mut().pop().unwrap();
    original
        .deliver(request.respond(Ok(vec![file("/r/alpha"), file("/r/beta")])))
        .unwrap();
    settle(&mut ui);
    let field = widget(&ui, &root, "filter");
    let entries = widget(&ui, &root, "entries");
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "browser",
            FileBrowser::new(&other, 42.0).unwrap(),
        )],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(widget(&ui, &root, "filter"), field);
    assert_eq!(widget(&ui, &root, "entries"), entries);
    ui.focus(&field).unwrap();
    ui.text_input("alp").unwrap();
    settle(&mut ui);
    assert_eq!(original.filter().get().unwrap(), "alp");
    assert_eq!(other.filter().get().unwrap(), "");
    let paths: Vec<_> = ui
        .children(&entries)
        .unwrap()
        .iter()
        .filter_map(|id| original.path_of(ui.key(id).unwrap()))
        .collect();
    assert_eq!(paths, [PathBuf::from("/r/alpha")]);
    let localized = UiStrings::english()
        .with(StringRole::FileBrowserFilter, "Find entries")
        .unwrap();
    ui.ui_settings().set_strings(localized).unwrap();
    settle(&mut ui);
    assert_eq!(widget(&ui, &root, "filter"), field);
    assert_eq!(original.filter().get().unwrap(), "alp");
    assert_eq!(other.filter().get().unwrap(), "");
}

#[test]
fn another_folder_response_updates_the_tree_without_rebuilding_current_entries() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer("/r", vec![dir("/r/a"), file("/r/alpha"), file("/r/beta")]);
    let tree = widget(&stage.ui, &stage.browser, "folders");
    let entries = widget(&stage.ui, &stage.browser, "entries");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowRight);
    let before = stage.ui.children(&entries).unwrap();
    stage.ui.paint().unwrap();
    stage.ui.set_profiling(true).unwrap();
    assert_eq!(
        stage.answer("/r/a", vec![file("/r/a/other")]),
        ListingDelivery::Applied
    );
    stage.ui.paint().unwrap();
    assert_eq!(stage.current(), PathBuf::from("/r"));
    assert_eq!(stage.ui.children(&entries).unwrap(), before);
    assert_eq!(stage.folders(), ["/r", "/r/a"]);
    let work: Vec<_> = stage
        .ui
        .profile()
        .stats
        .into_iter()
        .filter(|stat| {
            stat.key == "entries"
                && matches!(
                    stat.operation,
                    ProfileOperation::Update
                        | ProfileOperation::Measure
                        | ProfileOperation::Arrange
                        | ProfileOperation::Paint
                )
        })
        .collect();
    assert!(work.is_empty(), "{work:?}");
}

#[test]
fn hidden_browser_initializes_from_its_retained_controller_and_current_strings() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let original = FileBrowserController::new(
        &ui,
        ["/r"],
        ListSelection::new(&ui, SelectionMode::Single),
        move |request| queue.borrow_mut().push(request),
    )
    .unwrap();
    let other = FileBrowserController::new(
        &ui,
        ["/other"],
        ListSelection::new(&ui, SelectionMode::Single),
        |_| {},
    )
    .unwrap();
    ui.ui_settings()
        .set_strings(
            UiStrings::english()
                .with(StringRole::FileBrowserFilter, "Initial entries")
                .unwrap()
                .with(StringRole::FileBrowserName, "Initial browser")
                .unwrap(),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Container::new(Element::new(
                "browser",
                FileBrowser::new(&original, 24.0).unwrap(),
            )),
        ))
        .unwrap();
    let browser = widget(&ui, &root, "browser");
    ui.set_visible(&browser, false).unwrap();
    let request = requests.borrow_mut().pop().unwrap();
    original
        .deliver(request.respond(Ok(vec![file("/r/alpha")])))
        .unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "browser",
            FileBrowser::new(&other, 42.0).unwrap(),
        )],
    )
    .unwrap();
    settle(&mut ui);
    let step = ui
        .content_search(ContentQuery::new("Initial entries").unwrap())
        .step(&ui)
        .unwrap();
    assert!(step.hits.iter().any(|hit| hit.text == "Initial entries"));
    let inspected = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "browser")
        .unwrap();
    let semantics = inspected.details.semantics.unwrap();
    assert_eq!(semantics.name.as_deref(), Some("Initial browser"));
    assert_eq!(semantics.value, Some(SemanticValue::Text("/r".into())));
    ui.ui_settings()
        .set_strings(
            UiStrings::english()
                .with(StringRole::FileBrowserFilter, "Find entries")
                .unwrap(),
        )
        .unwrap();
    settle(&mut ui);
    assert!(!ui.children(&browser).unwrap().is_empty());
    let hidden_workspace = ui.capture_workspace().unwrap();
    assert!(
        hidden_workspace
            .entries()
            .any(|(path, _)| path.segments().last() == Some("panes"))
    );
    let report = ui.restore_workspace(&hidden_workspace).unwrap();
    assert!(report.rejected().next().is_none());
    assert_eq!(ui.capture_workspace().unwrap(), hidden_workspace);
    let query = ContentQuery::new("alpha").unwrap();
    let mut cursor = ui.content_search(query);
    let mut found = false;
    loop {
        let page = cursor.step(&ui).unwrap();
        found |= !page.hits.is_empty();
        if page.done {
            break;
        }
    }
    assert!(found, "never-shown browser must expose current source rows");
    ui.set_visible(&browser, true).unwrap();
    settle(&mut ui);
    let layout = widget(&ui, &browser, "layout");
    let field = widget(&ui, &browser, "filter");
    assert!(ui.inspect().components.iter().any(|component| {
        component.details.properties.get("placeholder")
            == Some(&InspectValue::Text("Find entries".into()))
    }));
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "browser",
            FileBrowser::new(&other, 42.0).unwrap(),
        )],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(widget(&ui, &browser, "layout"), layout);
    assert_eq!(widget(&ui, &browser, "filter"), field);
    ui.focus(&field).unwrap();
    ui.text_input("alp").unwrap();
    settle(&mut ui);
    assert_eq!(original.filter().get().unwrap(), "alp");
    assert_eq!(other.filter().get().unwrap(), "");
}

#[test]
fn whitespace_folder_names_remain_navigable_with_visible_breadcrumb_labels() {
    let mut stage = stage(SelectionMode::Single);
    for path in ["/r/ ", "/r/ /\u{2003}"] {
        stage.controller.navigate(path).unwrap();
        stage.ui.flush(stage.ui.now()).unwrap();
        assert_eq!(stage.current(), PathBuf::from(path));
        let crumbs = widget(&stage.ui, &stage.browser, "crumbs");
        let leaf = stage
            .ui
            .children(&crumbs)
            .unwrap()
            .into_iter()
            .find(|id| {
                let key = stage.ui.key(id).unwrap();
                stage.controller.path_of(key).as_deref() == Some(Path::new(path))
            })
            .unwrap();
        let leaf_key = stage.ui.key(&leaf).unwrap().to_owned();
        let snapshot = stage
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == leaf_key)
            .unwrap();
        let name = snapshot.details.semantics.unwrap().name.unwrap();
        let raw = Path::new(path).file_name().unwrap().to_string_lossy();
        assert_eq!(name, format!("{raw:?}"));
        assert!(!name.trim().is_empty());
        assert!(stage.ui.next_deadline().is_none());
        stage.ui.paint().unwrap();
    }
    let crumbs = widget(&stage.ui, &stage.browser, "crumbs");
    let parent = stage
        .ui
        .children(&crumbs)
        .unwrap()
        .into_iter()
        .find(|id| {
            stage
                .controller
                .path_of(stage.ui.key(id).unwrap())
                .as_deref()
                == Some(Path::new("/r/ "))
        })
        .unwrap();
    stage.ui.focus(&parent).unwrap();
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(stage.current(), PathBuf::from("/r/ "));
}

#[test]
fn whitespace_entry_and_folder_labels_are_visible_without_changing_paths() {
    let mut stage = stage(SelectionMode::Single);
    stage.answer(
        "/r",
        vec![
            dir("/r/ "),
            dir("/r/\u{2003}"),
            file("/r/\t"),
            file("/r/normal"),
        ],
    );
    let tree = widget(&stage.ui, &stage.browser, "folders");
    stage.ui.focus(&tree).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::ArrowRight);
    for (parent, paths) in [
        ("folders", vec!["/r/ ", "/r/\u{2003}"]),
        ("entries", vec!["/r/ ", "/r/\u{2003}", "/r/\t", "/r/normal"]),
    ] {
        for path in paths {
            let name = row_name(&stage, parent, path);
            let raw = Path::new(path).file_name().unwrap().to_string_lossy();
            let expected = if raw.trim().is_empty() {
                format!("{raw:?}")
            } else {
                raw.into_owned()
            };
            assert_eq!(name, expected, "{parent}: {path:?}");
        }
    }
    let entry = stage.row("entries", "/r/\t");
    stage.click(&entry);
    stage.click(&entry);
    assert_eq!(*stage.opened.borrow(), [PathBuf::from("/r/\t")]);
    let folder = stage.row("folders", "/r/\u{2003}");
    stage.click(&folder);
    assert_eq!(stage.current(), PathBuf::from("/r/\u{2003}"));
    stage.ui.paint().unwrap();
}

fn row_name(stage: &Stage, parent: &str, path: &str) -> String {
    let row = stage.row(parent, path);
    let snap = stage.ui.inspect();
    let container = snap.components.iter().find(|c| c.key == parent).unwrap().id;
    let below = |mut id: u64, ancestor: u64| {
        loop {
            if id == ancestor {
                return true;
            }
            let Some(parent) = snap
                .components
                .iter()
                .find(|c| c.id == id)
                .and_then(|c| c.parent)
            else {
                return false;
            };
            id = parent;
        }
    };
    let row_id = snap
        .components
        .iter()
        .find(|c| c.key == stage.ui.key(&row).unwrap() && below(c.id, container))
        .unwrap()
        .id;
    snap.components
        .iter()
        .find(|c| c.key == "name" && below(c.id, row_id))
        .unwrap()
        .details
        .semantics
        .as_ref()
        .unwrap()
        .name
        .as_ref()
        .unwrap()
        .clone()
}

#[test]
fn whitespace_failed_entry_names_remain_visible_after_localization() {
    let mut stage = stage(SelectionMode::Single);
    let path = "/r/\u{2003}";
    stage.answer(
        "/r",
        vec![DirectoryEntry::unreadable(
            path,
            ListingError::new(ListingErrorKind::PermissionDenied, "denied"),
        )],
    );
    let raw = Path::new(path).file_name().unwrap().to_string_lossy();
    assert_eq!(
        row_name(&stage, "entries", path),
        format!("{raw:?} — denied")
    );
    stage
        .ui
        .ui_settings()
        .set_strings(
            UiStrings::english()
                .with(StringRole::EntryFailed, "Cannot read {name}: {reason}")
                .unwrap(),
        )
        .unwrap();
    settle(&mut stage.ui);
    assert_eq!(
        row_name(&stage, "entries", path),
        format!("Cannot read {raw:?}: denied")
    );
}

#[test]
fn folder_filter_permissions_preserve_a_fixed_denial() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let allowed = ui.signal(true);
    let controller = FileBrowserController::new(
        &ui,
        ["/r"],
        ListSelection::new(&ui, SelectionMode::Single),
        |_| {},
    )?;
    ui.mount_tree(Element::new(
        "browser",
        FileBrowser::new(&controller, 24.0)?
            .folder_filter(false)
            .folder_filter(allowed.read_only()),
    ))?;
    ui.flush(Duration::ZERO)?;
    let snapshot = ui.inspect();
    let filter = snapshot
        .components
        .iter()
        .find(|entry| entry.key == "folders")
        .unwrap();
    assert_eq!(
        filter.details.properties.get("offered"),
        Some(&InspectValue::Bool(false))
    );
    Ok(())
}
