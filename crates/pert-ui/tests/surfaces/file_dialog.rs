//! The file dialog through the facade: session rules, what each mode
//! accepts, the save name and the replace question, and the retained outcome.
use crate::support;
use pert_ui::component::*;
use pert_ui::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};
use support::{press, settle, widget};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Stage {
    ui: Runtime,
    dialog: FileDialogController,
    requests: Rc<RefCell<Vec<DirectoryRequest>>>,
    root: WidgetId,
}

fn dir(path: &str) -> DirectoryEntry {
    DirectoryEntry::new(path, EntryKind::Directory)
}
fn file(path: &str) -> DirectoryEntry {
    DirectoryEntry::new(path, EntryKind::File)
}
fn listing() -> Vec<DirectoryEntry> {
    vec![
        dir("/r/sub"),
        file("/r/a.txt"),
        file("/r/b.png"),
        file("/r/c.txt"),
        DirectoryEntry::new("/r/pipe", EntryKind::Other),
    ]
}
fn text_filters() -> Vec<FileFilter> {
    vec![
        FileFilter::new("txt", "Text", ["txt"]).unwrap(),
        FileFilter::all("all", "All files").unwrap(),
    ]
}

fn stage() -> Stage {
    stage_in(Viewport::new(1000.0, 800.0, 1.0).unwrap())
}
fn stage_in(viewport: Viewport) -> Stage {
    stage_from(viewport, None)
}
/// A stage whose controller remembers in `memory`.
fn stage_with(memory: &FileDialogMemory) -> Stage {
    stage_from(Viewport::new(1000.0, 800.0, 1.0).unwrap(), Some(memory))
}
fn stage_from(viewport: Viewport, memory: Option<&FileDialogMemory>) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(viewport).unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let dialog =
        FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))
            .unwrap();
    let dialog = match memory {
        Some(memory) => dialog.with_memory(memory),
        None => dialog,
    };
    let root = ui
        .mount_tree(Element::new(
            "open",
            FileDialog::in_window(&ui, &dialog, Element::new("view", Label::new("View"))),
        ))
        .unwrap();
    settle(&mut ui);
    Stage {
        ui,
        dialog,
        requests,
        root,
    }
}

impl Stage {
    fn open(&mut self, request: FileDialogRequest) {
        self.dialog.open(request).unwrap();
        settle(&mut self.ui);
    }
    /// Opens `request` and answers its starting folder with [`listing`].
    fn opened(&mut self, request: FileDialogRequest) {
        self.open(request);
        self.answer("/r", listing());
    }
    #[track_caller]
    fn answer(&mut self, directory: &str, entries: Vec<DirectoryEntry>) {
        let request = {
            let mut requests = self.requests.borrow_mut();
            let index = requests
                .iter()
                .rposition(|request| request.directory() == Path::new(directory))
                .unwrap_or_else(|| panic!("no request for {directory}: {requests:?}"));
            requests.remove(index)
        };
        assert_eq!(
            self.dialog.deliver(request.respond(Ok(entries))).unwrap(),
            ListingDelivery::Applied
        );
        settle(&mut self.ui);
    }
    /// Answers the latest request for `directory` with a failure.
    #[track_caller]
    fn fail(&mut self, directory: &str, kind: ListingErrorKind) -> ListingDelivery {
        let request = {
            let mut requests = self.requests.borrow_mut();
            let index = requests
                .iter()
                .rposition(|request| request.directory() == Path::new(directory))
                .unwrap_or_else(|| panic!("no request for {directory}: {requests:?}"));
            requests.remove(index)
        };
        let delivery = self
            .dialog
            .deliver(request.respond(Err(ListingError::new(kind, "refused"))))
            .unwrap();
        settle(&mut self.ui);
        delivery
    }
    /// The folders requested and not yet answered, oldest first, without
    /// the root, which the folder tree asks for in every session.
    fn awaited(&self) -> Vec<PathBuf> {
        self.requests
            .borrow()
            .iter()
            .map(|request| request.directory().to_owned())
            .filter(|directory| directory != Path::new("/r"))
            .collect()
    }
    /// Takes the oldest unanswered request for `directory`.
    #[track_caller]
    fn take_request(&self, directory: &Path) -> DirectoryRequest {
        let mut requests = self.requests.borrow_mut();
        let index = requests
            .iter()
            .position(|request| request.directory() == directory)
            .unwrap_or_else(|| panic!("no request for {directory:?}"));
        requests.remove(index)
    }
    /// Opens `request`, answers `folder` with one text file, makes it
    /// active and confirms.
    fn accept_in(&mut self, request: FileDialogRequest, folder: &str) -> FileDialogSelection {
        self.open(request);
        self.answer(folder, vec![file(&format!("{folder}/a.txt"))]);
        self.click_row(0);
        assert!(self.confirm());
        match self.outcome() {
            Some(FileDialogOutcome::Accepted(selection)) => {
                settle(&mut self.ui);
                selection
            }
            other => panic!("not accepted: {other:?}"),
        }
    }
    fn status(&self) -> FileDialogStatus {
        self.dialog.status().get().unwrap()
    }
    fn outcome(&self) -> Option<FileDialogOutcome> {
        self.dialog.take_outcome().unwrap()
    }
    #[track_caller]
    fn accepted(&self) -> Vec<PathBuf> {
        match self.outcome() {
            Some(FileDialogOutcome::Accepted(selection)) => selection.into_paths(),
            other => panic!("not accepted: {other:?}"),
        }
    }
    fn current(&self) -> PathBuf {
        self.dialog.current().get().unwrap()
    }
    fn part(&self, name: &str) -> WidgetId {
        let picker = widget(&self.ui, &self.root, "picker");
        self.ui
            .find_exposed_part(&picker, name)
            .unwrap()
            .unwrap_or_else(|| panic!("no part {name}"))
    }
    /// The entry list's row at `index`, in display order.
    fn row(&self, index: usize) -> WidgetId {
        let entries = widget(&self.ui, &self.root, "entries");
        self.ui.children(&entries).unwrap()[index].clone()
    }
    fn point(&self, id: &WidgetId) -> Point {
        let bounds = self.ui.bounds(id).unwrap().expect("laid out");
        Point::new(bounds.x() + 20.0, bounds.y() + bounds.height() / 2.0).unwrap()
    }
    fn click_with(&mut self, id: &WidgetId, times: usize) {
        let point = self.point(id);
        for _ in 0..times {
            for kind in [
                PointerKind::Press(PointerButton::Primary),
                PointerKind::Release(PointerButton::Primary),
            ] {
                self.ui
                    .pointer_input(PointerEvent::new(point, kind))
                    .unwrap();
            }
        }
        settle(&mut self.ui);
    }
    fn click(&mut self, id: &WidgetId) {
        self.click_with(id, 1);
    }
    fn click_row(&mut self, index: usize) {
        let row = self.row(index);
        self.click(&row);
    }
    fn double_click_row(&mut self, index: usize) {
        let row = self.row(index);
        self.click_with(&row, 2);
    }
    fn confirm(&mut self) -> bool {
        let ended = self.dialog.accept().unwrap();
        settle(&mut self.ui);
        ended
    }
    fn type_name(&mut self, text: &str) {
        let name = self.part("name");
        self.ui.focus(&name).unwrap();
        self.ui.text_input(text).unwrap();
        settle(&mut self.ui);
    }
    fn text_of(&self, part: &str) -> String {
        let id = self.part(part);
        support::semantics_of(&self.ui, &id)
            .and_then(|semantics| semantics.name)
            .unwrap_or_default()
    }
    fn enabled(&self, part: &str) -> bool {
        let id = self.part(part);
        support::semantics_of(&self.ui, &id).is_some_and(|semantics| !semantics.disabled)
    }
}

fn request(mode: FileDialogMode) -> FileDialogRequest {
    FileDialogRequest::new(mode)
}

#[test]
fn nothing_is_requested_until_a_session_opens() {
    let mut stage = stage();
    assert!(stage.requests.borrow().is_empty());
    assert_eq!(stage.status(), FileDialogStatus::Idle);
    assert_eq!(stage.outcome(), None);
    stage.open(request(FileDialogMode::OpenFile));
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(stage.requests.borrow().len(), 1);
    assert_eq!(stage.requests.borrow()[0].directory(), Path::new("/r"));
}

#[test]
fn a_busy_controller_refuses_to_open_and_requests_nothing() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::ChooseFolder));
    assert_eq!(
        stage.dialog.open(request(FileDialogMode::OpenFile)),
        Err(FileDialogError::Busy),
        "during a session"
    );
    assert!(stage.dialog.accept().unwrap());
    assert_eq!(
        stage.dialog.open(request(FileDialogMode::OpenFile)),
        Err(FileDialogError::Busy),
        "before the outcome is taken"
    );
    assert_eq!(stage.accepted(), [PathBuf::from("/r")]);
    assert_eq!(
        stage.dialog.open(request(FileDialogMode::OpenFile)),
        Err(FileDialogError::Busy),
        "before the owner observed the closure"
    );
    assert!(stage.requests.borrow().is_empty(), "no refusal requested");
    settle(&mut stage.ui);
    stage.open(request(FileDialogMode::OpenFile));
    assert_eq!(stage.status(), FileDialogStatus::Open);
}

#[test]
fn the_outcome_is_taken_once() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    assert!(stage.dialog.cancel().unwrap());
    assert!(!stage.dialog.cancel().unwrap());
    assert_eq!(stage.status(), FileDialogStatus::Finished);
    assert_eq!(stage.outcome(), Some(FileDialogOutcome::Cancelled));
    assert_eq!(stage.outcome(), None);
    assert_eq!(stage.status(), FileDialogStatus::Idle);
}

#[test]
fn escape_cancels_the_session() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    press(&mut stage.ui, KeyCode::Escape);
    assert_eq!(stage.outcome(), Some(FileDialogOutcome::Cancelled));
}

#[test]
fn accepting_then_removing_the_owner_stays_accepted() -> TestResult {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::ChooseFolder));
    assert!(stage.dialog.accept()?);
    stage.ui.unmount(&stage.root)?;
    assert_eq!(stage.accepted(), [PathBuf::from("/r")]);
    Ok(())
}

#[test]
fn removing_the_owner_dismisses_an_open_session_even_before_layout() -> TestResult {
    let mut stage = stage();
    stage.dialog.open(request(FileDialogMode::OpenFile))?;
    stage.ui.unmount(&stage.root)?;
    assert_eq!(
        stage.outcome(),
        Some(FileDialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved))
    );
    Ok(())
}

#[test]
fn a_session_without_an_owner_waits_and_can_be_cancelled() -> TestResult {
    let ui = Runtime::new();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let dialog = FileDialogController::new(&ui, ["/r"], move |request: DirectoryRequest| {
        queue.borrow_mut().push(request)
    })?;
    dialog.open(request(FileDialogMode::OpenFile))?;
    assert_eq!(requests.borrow().len(), 1, "the listing is requested");
    assert_eq!(dialog.status().get()?, FileDialogStatus::Open);
    assert!(dialog.cancel()?);
    assert_eq!(dialog.take_outcome()?, Some(FileDialogOutcome::Cancelled));
    // Without an owner there is no closure to wait for.
    dialog.open(request(FileDialogMode::OpenFile))?;
    Ok(())
}

#[test]
fn confirm_is_unavailable_while_the_folder_loads_or_failed() {
    let mut stage = stage();
    stage.open(request(FileDialogMode::ChooseFolder));
    assert!(!stage.enabled("accept"));
    assert!(!stage.confirm(), "loading");
    assert_eq!(stage.status(), FileDialogStatus::Open);
    let pending = stage.requests.borrow_mut().remove(0);
    let error = ListingError::new(ListingErrorKind::NotFound, "gone");
    stage.dialog.deliver(pending.respond(Err(error))).unwrap();
    settle(&mut stage.ui);
    assert!(!stage.confirm(), "failed");
    assert_eq!(
        stage.status(),
        FileDialogStatus::Open,
        "a failure is no outcome"
    );
}

#[test]
fn open_file_accepts_the_active_file_and_opens_a_folder() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    assert!(!stage.enabled("accept"), "nothing is active");
    assert!(!stage.confirm());

    // Display order: sub, a.txt, b.png, c.txt, pipe.
    stage.click_row(0);
    assert!(!stage.confirm(), "a folder is opened, not accepted");
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    stage.answer("/r/sub", vec![file("/r/sub/x.txt")]);
    stage.click_row(0);
    assert!(stage.enabled("accept"));
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/sub/x.txt")]);
}

#[test]
fn an_unusable_entry_is_refused_in_place_with_a_message() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    stage.click_row(4);
    assert!(!stage.confirm());
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(stage.text_of("refusal"), "This entry cannot be used");
    // Choosing something else clears the refusal.
    stage.click_row(1);
    assert_eq!(stage.text_of("refusal"), "");
}

#[test]
fn a_double_click_accepts_a_file() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    stage.double_click_row(2);
    assert_eq!(stage.accepted(), [PathBuf::from("/r/b.png")]);
}

#[test]
fn open_files_reports_the_selection_in_display_order() {
    let mut stage = stage();
    // c.txt is listed a session before a.txt, so its row key sorts first
    // while the list shows it last.
    stage.open(request(FileDialogMode::OpenFiles));
    stage.answer("/r", vec![file("/r/c.txt")]);
    assert!(stage.dialog.cancel().unwrap());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    stage.open(request(FileDialogMode::OpenFiles));
    stage.answer("/r", vec![file("/r/a.txt"), file("/r/c.txt")]);
    assert!(!stage.confirm(), "nothing selected");
    // Select c.txt, then add a.txt with Control: display order is a, c.
    stage.click_row(1);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    stage.click_row(0);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Release)
        .unwrap();
    assert!(stage.confirm());
    assert_eq!(
        stage.accepted(),
        [PathBuf::from("/r/a.txt"), PathBuf::from("/r/c.txt")]
    );
}

#[test]
fn open_files_refuses_a_selection_that_is_not_all_files() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFiles));
    stage.click_row(1);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    stage.click_row(4);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Release)
        .unwrap();
    assert!(!stage.confirm());
    assert_eq!(
        stage.text_of("refusal"),
        "The selection must contain only files"
    );
    assert_eq!(stage.status(), FileDialogStatus::Open);
}

#[test]
fn the_selected_type_filters_the_list_and_is_reported() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::OpenFile)
            .filters(text_filters())
            .unwrap(),
    );
    // Shown: sub, a.txt, c.txt.
    stage.click_row(2);
    assert!(stage.confirm());
    let Some(FileDialogOutcome::Accepted(selection)) = stage.outcome() else {
        panic!("accepted");
    };
    assert_eq!(selection.paths(), [PathBuf::from("/r/c.txt")]);
    assert_eq!(selection.filter(), Some("txt"));
}

#[test]
fn choose_folder_accepts_the_active_folder_or_the_one_shown() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::ChooseFolder));
    assert_eq!(stage.text_of("accept"), "Choose “r”");
    stage.click_row(0);
    assert_eq!(stage.text_of("accept"), "Choose “sub”");
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/sub")]);
}

#[test]
fn choose_folder_ignores_file_types() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::ChooseFolder)
            .filters(text_filters())
            .unwrap(),
    );
    assert!(stage.confirm());
    let Some(FileDialogOutcome::Accepted(selection)) = stage.outcome() else {
        panic!("accepted");
    };
    assert_eq!(selection.filter(), None);
}

#[test]
fn an_accept_label_replaces_the_folder_caption() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::ChooseFolder).accept_label("Import"));
    assert_eq!(stage.text_of("accept"), "Import");
}

#[test]
fn save_completes_a_new_name_with_the_filters_extension() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::SaveFile)
            .filters(text_filters())
            .unwrap(),
    );
    assert!(!stage.enabled("accept"), "no name yet");
    stage.type_name("report");
    assert!(stage.confirm());
    let Some(FileDialogOutcome::Accepted(selection)) = stage.outcome() else {
        panic!("accepted");
    };
    assert_eq!(selection.paths(), [PathBuf::from("/r/report.txt")]);
    assert_eq!(selection.filter(), Some("txt"));
}

#[test]
fn save_keeps_names_that_have_an_extension_and_respects_the_switch() {
    for (typed, append, expected) in [
        (".env", true, "/r/.env.txt"),
        ("report.", true, "/r/report."),
        ("a.tar.gz", true, "/r/a.tar.gz"),
        ("photo.jpg", true, "/r/photo.jpg"),
        ("plain", false, "/r/plain"),
    ] {
        let mut stage = stage();
        stage.opened(
            request(FileDialogMode::SaveFile)
                .filters(text_filters())
                .unwrap()
                .append_extension(append),
        );
        stage.type_name(typed);
        assert!(stage.confirm(), "{typed}");
        assert_eq!(stage.accepted(), [PathBuf::from(expected)], "{typed}");
    }
}

#[test]
fn save_refuses_text_that_is_not_one_file_name() {
    for typed in ["..", ".", "a/b", "/etc/passwd"] {
        let mut stage = stage();
        stage.opened(request(FileDialogMode::SaveFile));
        stage.type_name(typed);
        assert!(!stage.confirm(), "{typed}");
        assert_eq!(
            stage.text_of("refusal"),
            "Enter a file name without folder separators",
            "{typed}"
        );
        assert_eq!(stage.status(), FileDialogStatus::Open);
    }
}

#[test]
fn save_opens_a_folder_whose_name_is_typed() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    stage.type_name("sub");
    assert!(!stage.confirm());
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    // With the folder listed, only an empty name keeps confirm unavailable.
    stage.answer("/r/sub", vec![file("/r/sub/x.txt")]);
    assert!(!stage.enabled("accept"), "the name was cleared");
}

#[test]
fn save_asks_before_replacing_and_replace_accepts_the_listed_path() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::SaveFile)
            .filters(text_filters())
            .unwrap(),
    );
    // The extension is completed first, then found in the listing.
    stage.type_name("a");
    assert!(!stage.confirm(), "the question is asked");
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(
        stage.text_of("question"),
        "“a.txt” already exists. Replace it?"
    );
    assert_eq!(stage.text_of("accept"), "Replace");
    assert_eq!(stage.text_of("cancel"), "Back");

    // Back returns to editing without ending the session.
    let back = stage.part("cancel");
    stage.click(&back);
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(stage.text_of("cancel"), "Cancel");

    assert!(!stage.confirm());
    let replace = stage.part("accept");
    stage.click(&replace);
    assert_eq!(stage.accepted(), [PathBuf::from("/r/a.txt")]);
}

#[test]
fn save_finds_a_file_the_type_filter_hides() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::SaveFile)
            .filters(text_filters())
            .unwrap(),
    );
    stage.type_name("b.png");
    assert!(!stage.confirm());
    assert_eq!(
        stage.text_of("question"),
        "“b.png” already exists. Replace it?"
    );
}

#[test]
fn save_refuses_an_unusable_entry_and_an_ambiguous_name() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    stage.type_name("pipe");
    assert!(!stage.confirm());
    assert_eq!(stage.text_of("refusal"), "This entry cannot be used");

    let mut stage = self::stage();
    stage.open(request(FileDialogMode::SaveFile));
    // A host may list two paths with one name.
    stage.answer("/r", vec![file("/r/same"), file("/elsewhere/same")]);
    stage.type_name("same");
    assert!(!stage.confirm());
    assert_eq!(stage.text_of("refusal"), "Several entries have this name");
}

#[test]
fn an_active_file_names_the_save_target_exactly() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    stage.click_row(1);
    assert!(stage.enabled("accept"), "the name was copied");
    assert!(!stage.confirm());
    assert_eq!(
        stage.text_of("question"),
        "“a.txt” already exists. Replace it?"
    );
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/a.txt")]);
}

#[cfg(unix)]
#[test]
fn editing_a_copied_name_ends_its_tie_to_the_listed_file() {
    use std::os::unix::ffi::OsStrExt;
    let odd = PathBuf::from("/r").join(std::ffi::OsStr::from_bytes(b"caf\xe9.txt"));
    let mut stage = stage();
    stage.open(request(FileDialogMode::SaveFile));
    stage.answer("/r", vec![DirectoryEntry::new(&odd, EntryKind::File)]);
    stage.click_row(0);
    // Append a character and delete it: the text reads the same again, but
    // it is typed text now and names another file.
    let name = stage.part("name");
    stage.ui.focus(&name).unwrap();
    press(&mut stage.ui, KeyCode::End);
    stage.ui.text_input("x").unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::Backspace);
    assert!(stage.confirm(), "a new file: nothing listed has this name");
    let accepted = stage.accepted();
    assert_ne!(accepted, [odd]);
    assert_eq!(accepted, [PathBuf::from("/r/caf\u{fffd}.txt")]);
}

#[test]
fn a_copied_name_is_judged_by_what_the_listing_holds_now() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    assert!(stage.dialog.cancel().unwrap());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    // The second session shows the kept listing while the fresh one is due.
    stage.open(request(FileDialogMode::SaveFile));
    stage.click_row(1);
    stage.answer("/r", vec![dir("/r/sub"), dir("/r/a.txt"), file("/r/c.txt")]);
    assert!(!stage.confirm(), "a.txt is a folder now: it is opened");
    assert_eq!(stage.current(), PathBuf::from("/r/a.txt"));
    assert_eq!(stage.status(), FileDialogStatus::Open);
}

#[cfg(unix)]
#[test]
fn saving_over_a_file_whose_name_is_not_unicode_returns_the_listed_path() {
    use std::os::unix::ffi::OsStrExt;
    let odd = PathBuf::from("/r").join(std::ffi::OsStr::from_bytes(b"caf\xe9.txt"));
    let mut stage = stage();
    stage.open(request(FileDialogMode::SaveFile));
    stage.answer("/r", vec![DirectoryEntry::new(&odd, EntryKind::File)]);
    stage.click_row(0);
    assert!(!stage.confirm());
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [odd]);
}

#[test]
fn a_listing_after_the_session_ended_moves_nothing() {
    let mut stage = stage();
    stage.open(request(FileDialogMode::OpenFile));
    let late = stage.requests.borrow_mut().remove(0);
    assert!(stage.dialog.cancel().unwrap());
    assert_eq!(stage.outcome(), Some(FileDialogOutcome::Cancelled));
    settle(&mut stage.ui);
    let _ = stage.dialog.deliver(late.respond(Ok(listing()))).unwrap();
    assert_eq!(stage.status(), FileDialogStatus::Idle);
    assert_eq!(stage.outcome(), None);
}

#[test]
fn a_new_session_starts_clean_in_its_starting_folder() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    stage.click_row(1);
    assert!(stage.confirm());
    let _ = stage.accepted();
    settle(&mut stage.ui);

    stage.open(request(FileDialogMode::OpenFiles).start_folder("/r/sub"));
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    stage.answer("/r/sub", vec![file("/r/sub/x"), file("/r/sub/y")]);
    assert!(!stage.confirm(), "the earlier selection is gone");
    // Several files can now be selected.
    stage.click_row(0);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    stage.click_row(1);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Release)
        .unwrap();
    assert!(stage.confirm());
    assert_eq!(stage.accepted().len(), 2);
}

#[test]
fn requests_and_filters_are_validated_when_built() {
    assert_eq!(
        request(FileDialogMode::OpenFile).context("  "),
        Err(FileDialogError::InvalidContext)
    );
    assert_eq!(
        request(FileDialogMode::OpenFile).context("x".repeat(65)),
        Err(FileDialogError::InvalidContext)
    );
    assert!(
        request(FileDialogMode::OpenFile)
            .context("open-project")
            .is_ok()
    );
    for name in ["", "..", "a/b", "a\nb"] {
        assert_eq!(
            request(FileDialogMode::SaveFile).suggested_name(name),
            Err(FileDialogError::InvalidName),
            "{name:?}"
        );
    }
    for extensions in [
        vec![],
        vec![""],
        vec![".txt"],
        vec!["txt."],
        vec!["*.txt"],
        vec!["txt:backup"],
        vec!["a b"],
    ] {
        assert_eq!(
            FileFilter::new("id", "Label", extensions.clone()),
            Err(FileDialogError::InvalidFilter),
            "{extensions:?}"
        );
    }
    assert_eq!(
        FileFilter::new("", "Label", ["txt"]),
        Err(FileDialogError::InvalidFilter)
    );
    assert_eq!(
        FileFilter::all("all", " "),
        Err(FileDialogError::InvalidFilter)
    );
    assert_eq!(
        FileFilter::all("all", "All\nfiles"),
        Err(FileDialogError::InvalidFilter)
    );
    let twice = [
        FileFilter::new("txt", "Text", ["txt"]).unwrap(),
        FileFilter::new("txt", "Other", ["md"]).unwrap(),
    ];
    assert_eq!(
        request(FileDialogMode::OpenFile).filters(twice),
        Err(FileDialogError::DuplicateFilter("txt".to_owned()))
    );
}

#[test]
fn roots_and_sizes_are_validated() {
    let ui = Runtime::new();
    let none: [&str; 0] = [];
    assert!(matches!(
        FileDialogController::new(&ui, none, |_| {}),
        Err(FileDialogError::NoRoots)
    ));
    assert!(matches!(
        FileDialogController::new(&ui, ["/r", "/r"], |_| {}),
        Err(FileDialogError::DuplicateRoot(_))
    ));
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {}).unwrap();
    let describe = || FileDialog::in_view(&ui, &dialog, Element::new("view", Label::new("View")));
    assert!(matches!(
        describe().size(Size::from([400, 400])),
        Err(FileDialogError::TooSmall)
    ));
    assert!(describe().size(Size::from([480, 320])).is_ok());
    assert!(matches!(
        describe().row_height(0.0),
        Err(FileDialogError::Geometry(_))
    ));
}

#[test]
fn a_window_smaller_than_the_content_scrolls_it_at_full_size() {
    for view_modal in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 300.0, 1.0).unwrap())
            .unwrap();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let queue = requests.clone();
        let dialog =
            FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))
                .unwrap();
        let anchor = Element::new("view", Label::new("View"));
        let described = if view_modal {
            FileDialog::in_view(&ui, &dialog, anchor)
        } else {
            FileDialog::in_window(&ui, &dialog, anchor)
        };
        let root = ui.mount_tree(Element::new("open", described)).unwrap();
        let mut stage = Stage {
            ui,
            dialog,
            requests,
            root,
        };
        stage.opened(request(FileDialogMode::OpenFile));
        let bounds = |stage: &Stage, id: &WidgetId| stage.ui.bounds(id).unwrap().expect("laid out");
        let picker = bounds(&stage, &widget(&stage.ui, &stage.root, "picker"));
        assert_eq!((picker.width(), picker.height()), (720.0, 480.0));
        let browser = bounds(&stage, &widget(&stage.ui, &stage.root, "browser"));
        assert_eq!(browser.width(), 720.0);
        assert!(browser.height() > 300.0, "{browser:?}");

        // The confirm button starts outside the window; Tab brings it in.
        stage.click_row(1);
        let accept = stage.part("accept");
        assert!(bounds(&stage, &accept).y() > 300.0);
        let mut tabs = 0;
        while stage.ui.focused().as_ref() != Some(&accept) {
            press(&mut stage.ui, KeyCode::Tab);
            tabs += 1;
            assert!(tabs < 40, "Tab never reached the confirm button");
        }
        let shown = bounds(&stage, &accept);
        assert!(
            shown.y() >= 0.0 && shown.y() + shown.height() <= 300.0,
            "view_modal {view_modal}: {shown:?}"
        );
        press(&mut stage.ui, KeyCode::Enter);
        assert_eq!(stage.accepted(), [PathBuf::from("/r/a.txt")]);
    }
}

#[test]
fn hiding_the_owner_dismisses_the_session() -> TestResult {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFile));
    stage.ui.set_visible(&stage.root, false)?;
    settle(&mut stage.ui);
    assert!(matches!(
        stage.outcome(),
        Some(FileDialogOutcome::Dismissed(_))
    ));
    Ok(())
}

#[test]
fn a_modal_conflict_ends_the_session_as_failed() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let other = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "other",
                Dialog::new(
                    other.clone(),
                    Element::new("a", Label::new("A")),
                    Element::new("body", Label::new("Body")),
                ),
            ),
            Element::new(
                "open",
                FileDialog::in_window(&ui, &dialog, Element::new("b", Label::new("B"))),
            ),
        ]),
    ))?;
    settle(&mut ui);
    other.open()?;
    settle(&mut ui);
    dialog.open(request(FileDialogMode::OpenFile))?;
    let _ = ui.flush(ui.now());
    assert_eq!(
        dialog.take_outcome()?,
        Some(FileDialogOutcome::Failed(FileDialogFailure::Surface(
            SurfaceDismissal::ModalConflict
        )))
    );
    Ok(())
}

#[test]
fn describing_the_dialog_again_keeps_the_session_and_reconciles_the_anchor() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let dialog =
        FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))?;
    let describe = |ui: &Runtime, text: &str| {
        Element::new(
            "open",
            FileDialog::in_window(ui, &dialog, Element::new("view", Label::new(text))),
        )
    };
    let root = ui.mount_tree(Element::new("root", Column::new([describe(&ui, "One")])))?;
    settle(&mut ui);
    dialog.open(request(FileDialogMode::ChooseFolder))?;
    settle(&mut ui);
    let picker = widget(&ui, &root, "picker");

    ui.reconcile_children(&root, vec![describe(&ui, "Two")])?;
    settle(&mut ui);
    assert_eq!(dialog.status().get()?, FileDialogStatus::Open);
    assert_eq!(widget(&ui, &root, "picker"), picker, "the content is kept");
    let view = widget(&ui, &root, "view");
    assert_eq!(
        support::semantics_of(&ui, &view).and_then(|semantics| semantics.name),
        Some("Two".to_owned())
    );
    Ok(())
}

#[test]
fn another_controller_under_the_same_key_dismisses_the_session_and_takes_over() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let first = FileDialogController::new(&ui, ["/first"], |_| {})?;
    let second = FileDialogController::new(&ui, ["/second"], |_| {})?;
    let describe = |ui: &Runtime, dialog: &FileDialogController| {
        Element::new(
            "open",
            FileDialog::in_window(ui, dialog, Element::new("view", Label::new("View"))),
        )
    };
    let root = ui.mount_tree(Element::new("root", Column::new([describe(&ui, &first)])))?;
    settle(&mut ui);
    first.open(request(FileDialogMode::ChooseFolder))?;
    settle(&mut ui);

    ui.reconcile_children(&root, vec![describe(&ui, &second)])?;
    settle(&mut ui);
    assert_eq!(
        first.take_outcome()?,
        Some(FileDialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved))
    );
    second.open(request(FileDialogMode::ChooseFolder))?;
    settle(&mut ui);
    // The mounted content is the second controller's: it shows its folder.
    let browser = widget(&ui, &root, "browser");
    assert_eq!(
        support::semantics_of(&ui, &browser).and_then(|semantics| semantics.value),
        Some(SemanticValue::Text("/second".to_owned()))
    );
    Ok(())
}

#[test]
fn a_placeholder_row_is_no_entry() {
    let mut stage = stage();
    stage.open(request(FileDialogMode::OpenFile));
    // The loading row can be clicked, and names nothing.
    stage.click_row(0);
    assert!(!stage.confirm());
    stage.answer("/r", Vec::new());
    stage.click_row(0);
    assert!(!stage.enabled("accept"));
    assert!(!stage.confirm());
    assert_eq!(stage.status(), FileDialogStatus::Open);
}

#[test]
fn a_selected_entry_the_filter_text_hides_is_not_reported() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFiles));
    stage.click_row(1);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    stage.click_row(3);
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Release)
        .unwrap();
    // Hide a.txt; c.txt stays shown and selected.
    let browser = widget(&stage.ui, &stage.root, "browser");
    let filter = widget(&stage.ui, &browser, "filter");
    stage.ui.focus(&filter).unwrap();
    stage.ui.text_input("c").unwrap();
    settle(&mut stage.ui);
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/c.txt")]);
}

#[test]
fn enter_in_the_name_field_confirms() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    assert_eq!(stage.text_of("name"), "Name", "the field is named");
    stage.type_name("notes.md");
    press(&mut stage.ui, KeyCode::Enter);
    assert_eq!(stage.accepted(), [PathBuf::from("/r/notes.md")]);
}

#[test]
fn a_suggested_name_starts_the_name_field() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::SaveFile)
            .suggested_name("draft.txt")
            .unwrap(),
    );
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/draft.txt")]);
}

#[test]
fn default_texts_follow_the_string_table() -> TestResult {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile));
    assert_eq!(stage.text_of("accept"), "Save");
    let swedish = UiStrings::english()
        .with_text(StringRole::FileDialogSave, "Spara")?
        .with_text(StringRole::DialogCancel, "Avbryt")?;
    stage.ui.ui_settings().set_strings(swedish)?;
    settle(&mut stage.ui);
    assert_eq!(stage.text_of("accept"), "Spara");
    assert_eq!(stage.text_of("cancel"), "Avbryt");
    Ok(())
}

#[test]
fn the_browsers_split_is_workspace_state_and_a_session_is_not() -> TestResult {
    let mut stage = stage();
    // Captured before any session: the closed dialog's content is mounted.
    let closed = stage.ui.capture_workspace()?;
    // The path is what a stored document holds: the dialog's `dialog`
    // segment and the content's keys, none of the structural ones.
    assert!(
        closed.entries().any(|(path, _)| {
            let segments = path.segments().collect::<Vec<_>>();
            segments.starts_with(&["open", "dialog", "picker", "browser"])
                && segments.ends_with(&["panes"])
        }),
        "the split takes part while the dialog is closed: {:?}",
        closed
            .entries()
            .map(|(path, _)| path.segments().collect::<Vec<_>>())
            .collect::<Vec<_>>()
    );
    stage.opened(request(FileDialogMode::SaveFile));
    stage.type_name("draft");
    stage.click_row(1);
    let open = stage.ui.capture_workspace()?;
    let paths = |workspace: &Workspace| {
        workspace
            .entries()
            .map(|(path, _)| path.segments().collect::<Vec<_>>().join("/"))
            .collect::<Vec<_>>()
    };
    assert_eq!(paths(&open), paths(&closed), "a session adds no entry");
    assert_eq!(open, closed, "and changes no entry's state");
    assert!(stage.dialog.cancel()?);
    settle(&mut stage.ui);
    assert!(stage.ui.restore_workspace(&closed)?.is_complete());
    Ok(())
}

/// The advanced path `FileDialog` makes ordinary: a picker composed by hand
/// from `Dialog`, an idle `FileBrowser` and its public listing signals.
#[test]
fn a_picker_can_be_composed_by_hand_from_the_public_parts() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let browser = FileBrowserController::new_idle(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        ["/r"],
        move |request| queue.borrow_mut().push(request),
    )?;
    browser.set_entry_predicate(|entry| {
        entry.opens_as_directory() || entry.path().extension().is_some_and(|found| found == "txt")
    })?;
    let session = DialogController::new(&ui);
    let chosen: Rc<RefCell<Option<PathBuf>>> = Rc::default();

    // Confirm accepts the active shown entry, read in display order.
    let (confirm, picked, state) = (session.clone(), chosen.clone(), browser.clone());
    let accept = Button::new("Open", move |_| {
        let active = state.selection().active().get()?;
        let path = active.and_then(|key| state.path_of(&key));
        let shown = state.shown_entries().get()?;
        if let Some(entry) = shown
            .iter()
            .find(|entry| Some(entry.path()) == path.as_deref())
        {
            *picked.borrow_mut() = Some(entry.path().to_owned());
            confirm.accept()?;
        }
        Ok(())
    });
    let content = Element::new(
        "picker",
        RegionLayout::new()
            .fixed()
            .center(Element::new("browser", FileBrowser::new(&browser, 24.0)?))
            .bottom(Element::new("accept", accept)),
    );
    let root = ui.mount_tree(Element::new(
        "open",
        Dialog::new(
            session.clone(),
            Element::new("view", Label::new("View")),
            content,
        ),
    ))?;
    settle(&mut ui);
    assert!(requests.borrow().is_empty(), "an idle browser asks nothing");

    session.open()?;
    browser.navigate("/r")?;
    settle(&mut ui);
    let request = requests.borrow_mut().remove(0);
    browser.deliver(request.respond(Ok(vec![file("/r/a.txt"), file("/r/b.png")])))?;
    settle(&mut ui);
    let CurrentListing::Loaded(all) = browser.listing().get()? else {
        panic!("loaded");
    };
    assert_eq!((all.len(), browser.shown_entries().get()?.len()), (2, 1));

    let key = {
        let entries = widget(&ui, &root, "entries");
        ui.key(&ui.children(&entries)?[0])?.to_owned()
    };
    browser.selection().set_active(Some(key))?;
    settle(&mut ui);
    let button = widget(&ui, &root, "accept");
    ui.focus(&button)?;
    press(&mut ui, KeyCode::Enter);

    // The outcome outlives the widget.
    ui.unmount(&root)?;
    assert_eq!(session.take_outcome()?, Some(DialogOutcome::Accepted));
    assert_eq!(chosen.borrow().clone(), Some(PathBuf::from("/r/a.txt")));
    Ok(())
}

#[test]
fn an_active_row_that_is_not_selected_is_no_part_of_several_files() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::OpenFiles));
    stage.click_row(1);
    // Control moves the active row without selecting: down to the pipe.
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Press)
        .unwrap();
    for _ in 0..3 {
        press(&mut stage.ui, KeyCode::ArrowDown);
    }
    stage
        .ui
        .key_input(KeyCode::ControlLeft, KeyAction::Release)
        .unwrap();
    assert!(stage.confirm());
    assert_eq!(stage.accepted(), [PathBuf::from("/r/a.txt")]);
}

/// Runs a closure once from a component's update, where writes are refused.
struct InUpdate(Option<Box<dyn FnOnce()>>);
impl Component for InUpdate {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if let Some(run) = self.0.take() {
            run();
        }
        Ok(())
    }
}

#[test]
fn a_refused_accept_or_take_keeps_the_session_and_its_selection_together() -> TestResult {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::ChooseFolder));
    let seen: Rc<RefCell<Vec<String>>> = Rc::default();

    let (dialog, log) = (stage.dialog.clone(), seen.clone());
    let probe = stage.ui.mount_tree(Element::new(
        "probe",
        InUpdate(Some(Box::new(move || {
            log.borrow_mut().push(format!("{:?}", dialog.accept()));
        }))),
    ))?;
    settle(&mut stage.ui);
    assert_eq!(*seen.borrow(), ["Err(WriteDuringUpdate)"]);
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(stage.outcome(), None);
    stage.ui.unmount(&probe)?;

    assert!(stage.dialog.accept()?);
    let (dialog, log) = (stage.dialog.clone(), seen.clone());
    stage.ui.mount_tree(Element::new(
        "probe",
        InUpdate(Some(Box::new(move || {
            log.borrow_mut()
                .push(format!("{:?}", dialog.take_outcome().map(|_| ())));
        }))),
    ))?;
    settle(&mut stage.ui);
    assert_eq!(seen.borrow()[1], "Err(WriteDuringUpdate)");
    // The refused take released nothing: the selection is still there, once.
    assert_eq!(stage.accepted(), [PathBuf::from("/r")]);
    assert_eq!(stage.outcome(), None);
    Ok(())
}

#[test]
fn a_session_opened_under_a_hidden_owner_stays_open_until_it_is_shown() {
    let mut stage = stage();
    stage.ui.set_visible(&stage.root, false).unwrap();
    settle(&mut stage.ui);
    stage.open(request(FileDialogMode::ChooseFolder));
    assert_eq!(stage.status(), FileDialogStatus::Open);
    assert_eq!(stage.outcome(), None);
    stage.ui.set_visible(&stage.root, true).unwrap();
    settle(&mut stage.ui);
    stage.answer("/r", listing());
    assert_eq!(stage.status(), FileDialogStatus::Open);
    // The content is presented: its confirm button takes the click.
    let accept = stage.part("accept");
    stage.click(&accept);
    assert_eq!(stage.accepted(), [PathBuf::from("/r")]);
}

#[test]
fn choosing_another_type_in_the_selector_filters_the_list() {
    let mut stage = stage();
    stage.opened(
        request(FileDialogMode::OpenFile)
            .filters(text_filters())
            .unwrap(),
    );
    let rows = |stage: &Stage| {
        let entries = widget(&stage.ui, &stage.root, "entries");
        stage.ui.children(&entries).unwrap().len()
    };
    assert_eq!(rows(&stage), 3, "sub, a.txt, c.txt");
    let selector = stage.part("filter");
    let control = stage
        .ui
        .find_descendant(&selector, "select")
        .unwrap()
        .expect("the selector's control");
    let named = |id: &WidgetId| support::semantics_of(&stage.ui, id).and_then(|found| found.name);
    assert_eq!(named(&control).as_deref(), Some("File type"));
    stage.ui.focus(&control).unwrap();
    // Open the list, move to "All files", choose it.
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::ArrowDown);
    press(&mut stage.ui, KeyCode::Enter);
    settle(&mut stage.ui);
    assert_eq!(rows(&stage), 5, "every entry shows");
    stage.click_row(2);
    assert!(stage.confirm());
    let Some(FileDialogOutcome::Accepted(selection)) = stage.outcome() else {
        panic!("accepted");
    };
    assert_eq!(selection.paths(), [PathBuf::from("/r/b.png")]);
    assert_eq!(selection.filter(), Some("all"));
}

#[test]
fn the_dialog_is_named_by_the_sessions_title() {
    let mut stage = stage();
    stage.opened(request(FileDialogMode::SaveFile).title("Export drawing"));
    let dialogs: Vec<_> = stage
        .ui
        .inspect()
        .components
        .into_iter()
        .filter_map(|component| component.details.semantics)
        .filter(|semantics| semantics.role == SemanticRole::Dialog)
        .collect();
    assert_eq!(dialogs.len(), 1);
    assert_eq!(dialogs[0].name.as_deref(), Some("Export drawing"));
    assert_eq!(dialogs[0].expanded, Some(true));
}

#[test]
fn on_outcome_hands_each_ended_session_to_the_application_once() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let seen: Rc<RefCell<Vec<FileDialogOutcome>>> = Rc::default();
    let (log, again) = (seen.clone(), dialog.clone());
    let root = ui.mount_tree(Element::new(
        "open",
        FileDialog::in_window(&ui, &dialog, Element::new("view", Label::new("View"))).on_outcome(
            move |_, outcome| {
                // The first ending opens the next session from the callback.
                if log.borrow().is_empty() {
                    again
                        .open(request(FileDialogMode::OpenFile))
                        .map_err(|error| match error {
                            FileDialogError::Runtime(error) => error,
                            other => panic!("reopening: {other}"),
                        })?;
                }
                log.borrow_mut().push(outcome);
                Ok(())
            },
        ),
    ))?;
    settle(&mut ui);
    dialog.open(request(FileDialogMode::OpenFile))?;
    settle(&mut ui);
    assert!(seen.borrow().is_empty());
    assert!(dialog.cancel()?);
    settle(&mut ui);
    assert_eq!(*seen.borrow(), [FileDialogOutcome::Cancelled]);
    assert_eq!(dialog.status().get()?, FileDialogStatus::Open, "reopened");
    assert_eq!(dialog.take_outcome()?, None, "the dialog took the first");

    // Removed before the call: the callback does not run, the outcome stays.
    assert!(dialog.cancel()?);
    ui.unmount(&root)?;
    assert_eq!(seen.borrow().len(), 1);
    assert_eq!(dialog.take_outcome()?, Some(FileDialogOutcome::Cancelled));
    Ok(())
}

#[test]
fn a_name_the_field_cannot_show_is_neither_suggested_nor_copied() {
    // A Unicode line separator is no control character, yet the field
    // refuses it; the dialog must not judge text the field does not show.
    assert_eq!(
        request(FileDialogMode::SaveFile).suggested_name("a\u{2028}b"),
        Err(FileDialogError::InvalidName)
    );
    let mut stage = stage();
    stage.open(request(FileDialogMode::SaveFile));
    stage.answer("/r", vec![file("/r/a\u{2028}b")]);
    stage.click_row(0);
    assert!(!stage.enabled("accept"), "the name stays empty");
    assert!(!stage.confirm());
    assert_eq!(stage.status(), FileDialogStatus::Open);
}

/// Which dialog's callback received which outcome, in call order.
type Reported = Rc<RefCell<Vec<(&'static str, FileDialogOutcome)>>>;

/// Mounts one dialog per controller in a column, each logging its name with
/// the outcomes its callback receives.
fn logging(
    ui: &mut Runtime,
    dialogs: &[(&'static str, &FileDialogController)],
) -> (WidgetId, Reported) {
    let seen: Reported = Rc::default();
    let children: Vec<Element> = dialogs
        .iter()
        .map(|&(name, dialog)| {
            let log = seen.clone();
            Element::new(
                name,
                FileDialog::in_view(ui, dialog, Element::new("view", Label::new(name))).on_outcome(
                    move |_, outcome| {
                        log.borrow_mut().push((name, outcome));
                        Ok(())
                    },
                ),
            )
        })
        .collect();
    let root = ui
        .mount_tree(Element::new("root", Column::new(children)))
        .unwrap();
    (root, seen)
}

#[test]
fn the_application_taking_the_outcome_first_leaves_the_callback_unrun() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let (_root, seen) = logging(&mut ui, &[("a", &dialog)]);
    settle(&mut ui);
    dialog.open(request(FileDialogMode::OpenFile))?;
    settle(&mut ui);
    assert!(dialog.cancel()?);
    assert_eq!(dialog.take_outcome()?, Some(FileDialogOutcome::Cancelled));
    settle(&mut ui);
    assert!(seen.borrow().is_empty(), "the outcome has one taker");
    Ok(())
}

#[test]
fn callbacks_run_in_the_order_the_sessions_ended() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let first = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let second = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let (_root, seen) = logging(&mut ui, &[("first", &first), ("second", &second)]);
    settle(&mut ui);
    first.open(request(FileDialogMode::OpenFile))?;
    second.open(request(FileDialogMode::OpenFile))?;
    settle(&mut ui);
    // Ended against mount order, within one flush.
    assert!(second.cancel()?);
    assert!(first.cancel()?);
    settle(&mut ui);
    let names: Vec<_> = seen.borrow().iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["second", "first"]);
    Ok(())
}

#[test]
fn a_hidden_dialog_still_reports_its_dismissed_session() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let (root, seen) = logging(&mut ui, &[("a", &dialog)]);
    settle(&mut ui);
    dialog.open(request(FileDialogMode::OpenFile))?;
    settle(&mut ui);
    ui.set_visible(&widget(&ui, &root, "a"), false)?;
    settle(&mut ui);
    assert!(
        matches!(
            seen.borrow().as_slice(),
            [("a", FileDialogOutcome::Dismissed(_))]
        ),
        "{:?}",
        seen.borrow()
    );
    Ok(())
}

#[test]
fn a_second_dialog_on_the_controller_never_reports_the_owners_session() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 800.0, 1.0)?)?;
    let dialog = FileDialogController::new(&ui, ["/r"], |_| {})?;
    let (root, seen) = logging(&mut ui, &[("kept", &dialog), ("other", &dialog)]);
    // The second mount is refused while the first owns the controller.
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::DialogBusy));
    dialog.open(request(FileDialogMode::OpenFile))?;
    let _ = ui.flush(ui.now());
    // While the refused mount fails every flush nothing is delivered, and
    // the outcome waits with the controller.
    assert!(dialog.cancel()?);
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::DialogBusy));
    assert!(seen.borrow().is_empty());
    assert_eq!(dialog.take_outcome()?, Some(FileDialogOutcome::Cancelled));
    let _ = ui.flush(ui.now());

    // The owner is removed with a session open: that outcome is nobody's
    // callback's, also once the other dialog has taken the controller over.
    dialog.open(request(FileDialogMode::OpenFile))?;
    let _ = ui.flush(ui.now());
    ui.unmount(&widget(&ui, &root, "kept"))?;
    settle(&mut ui);
    settle(&mut ui);
    assert!(seen.borrow().is_empty());
    assert_eq!(
        dialog.take_outcome()?,
        Some(FileDialogOutcome::Dismissed(SurfaceDismissal::OwnerRemoved))
    );

    // The dialog that took over reports the sessions it owns.
    settle(&mut ui);
    dialog.open(request(FileDialogMode::OpenFile))?;
    settle(&mut ui);
    assert!(dialog.cancel()?);
    settle(&mut ui);
    assert_eq!(*seen.borrow(), [("other", FileDialogOutcome::Cancelled)]);
    Ok(())
}

// Memory: what an accepted session leaves for the next one.

fn folder_in(context: &str) -> FileDialogRequest {
    request(FileDialogMode::ChooseFolder)
        .context(context)
        .unwrap()
}

#[test]
fn an_accepted_session_is_remembered_per_context() {
    let memory = FileDialogMemory::new();
    let mut stage = stage_with(&memory);
    let selection = stage.accept_in(
        request(FileDialogMode::OpenFile)
            .context("project")
            .unwrap()
            .filters(text_filters())
            .unwrap()
            .start_folder("/r/sub"),
        "/r/sub",
    );
    assert!(selection.folder_remembered());
    assert_eq!(memory.last_folder(Some("project")), Some("/r/sub".into()));
    assert_eq!(memory.last_filter(Some("project")).as_deref(), Some("txt"));
    assert_eq!(memory.recent_folders(), [PathBuf::from("/r/sub")]);
    assert_eq!(memory.last_folder(None), None, "another purpose");
    assert_eq!(memory.last_folder(Some("export")), None);

    // The same purpose starts where it was; another starts at the root.
    stage.open(folder_in("project"));
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    assert!(stage.dialog.cancel().unwrap());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    stage.open(folder_in("export"));
    assert_eq!(stage.current(), PathBuf::from("/r"));
}

#[test]
fn a_session_that_is_not_accepted_remembers_nothing() {
    let memory = FileDialogMemory::new();
    let mut stage = stage_with(&memory);
    stage.opened(folder_in("project"));
    assert!(stage.dialog.cancel().unwrap());
    assert_eq!(stage.outcome(), Some(FileDialogOutcome::Cancelled));
    assert_eq!(memory, FileDialogMemory::new());
    assert_eq!(memory.encode(), "pert-ui-file-dialog-memory 1\n");
}

#[test]
fn a_dialog_without_a_memory_reports_the_folder_as_not_remembered() {
    let mut stage = stage();
    let selection = stage.accept_in(request(FileDialogMode::ChooseFolder), "/r");
    assert!(!selection.folder_remembered());
}

#[test]
fn the_remembered_type_is_selected_when_the_request_offers_it() {
    let memory = FileDialogMemory::decode(
        "pert-ui-file-dialog-memory 1\ncontext \"project\" filter=\"all\"\n",
    )
    .unwrap();
    let mut stage = stage_with(&memory);
    let asked = || {
        request(FileDialogMode::OpenFile)
            .context("project")
            .unwrap()
    };
    stage.opened(asked().filters(text_filters()).unwrap());
    stage.click_row(2);
    assert!(stage.confirm(), "b.png shows under the remembered type");
    match stage.outcome() {
        Some(FileDialogOutcome::Accepted(selection)) => {
            assert_eq!(selection.filter(), Some("all"));
        }
        other => panic!("{other:?}"),
    }
    settle(&mut stage.ui);
    // A request that no longer offers it starts with its first type.
    stage.opened(
        asked()
            .filters([FileFilter::new("png", "Images", ["png"]).unwrap()])
            .unwrap(),
    );
    stage.click_row(1);
    assert!(stage.confirm());
    match stage.outcome() {
        Some(FileDialogOutcome::Accepted(selection)) => {
            assert_eq!(selection.filter(), Some("png"));
            assert_eq!(selection.paths(), [PathBuf::from("/r/b.png")]);
        }
        other => panic!("{other:?}"),
    }
}

fn remembering(folder: &str) -> FileDialogMemory {
    FileDialogMemory::decode(&format!(
        "pert-ui-file-dialog-memory 1\ncontext \"project\" folder=\"{folder}\"\n"
    ))
    .unwrap()
}

#[test]
fn an_explicit_starting_folder_takes_precedence_and_is_not_searched_for() {
    let mut stage = stage_with(&remembering("/r/sub"));
    stage.open(folder_in("project").start_folder("/r/else"));
    assert_eq!(stage.current(), PathBuf::from("/r/else"));
    assert_eq!(
        stage.fail("/r/else", ListingErrorKind::NotFound),
        ListingDelivery::Applied
    );
    assert_eq!(stage.current(), PathBuf::from("/r/else"), "shown as failed");
    assert!(stage.awaited().is_empty());
}

#[test]
fn a_remembered_folder_that_is_gone_gives_way_to_its_ancestors() {
    let mut stage = stage_with(&remembering("/r/a/b"));
    stage.open(folder_in("project"));
    assert_eq!(stage.awaited(), [PathBuf::from("/r/a/b")]);
    stage.fail("/r/a/b", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r/a"));
    stage.fail("/r/a", ListingErrorKind::NotADirectory);
    assert_eq!(stage.current(), PathBuf::from("/r"));
    stage.answer("/r", listing());
    assert_eq!(stage.current(), PathBuf::from("/r"));
    assert!(stage.awaited().is_empty());
    assert_eq!(stage.status(), FileDialogStatus::Open);
    // The search is over: a later failure of the folder moves nothing.
    let sub = stage.row(0);
    stage.click_with(&sub, 2);
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    stage.fail("/r/sub", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
}

#[test]
fn the_search_ends_at_the_first_root_and_at_a_refusal_of_another_kind() {
    // Outside every root: the ancestors run out and the first root is tried.
    let mut stage = stage_with(&remembering("/gone"));
    stage.open(folder_in("project"));
    stage.fail("/gone", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/"));
    stage.fail("/", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r"));
    stage.fail("/r", ListingErrorKind::NotFound);
    assert_eq!(
        stage.current(),
        PathBuf::from("/r"),
        "nothing is left to try"
    );
    assert!(stage.awaited().is_empty());

    // A folder the user may not read is shown as that, not searched past.
    let mut stage = stage_with(&remembering("/r/a/b"));
    stage.open(folder_in("project"));
    stage.fail("/r/a/b", ListingErrorKind::PermissionDenied);
    assert_eq!(stage.current(), PathBuf::from("/r/a/b"));
    assert!(stage.awaited().is_empty());
}

#[test]
fn the_search_stops_when_the_user_goes_elsewhere() {
    let mut stage = stage_with(&remembering("/r/a/b"));
    stage
        .dialog
        .places()
        .set(vec![FilePlace::new("Samples", "/r/samples").unwrap()])
        .unwrap();
    stage.open(folder_in("project"));
    let place = widget(&stage.ui, &stage.root, "place-0");
    stage.click(&place);
    assert_eq!(stage.current(), PathBuf::from("/r/samples"));
    // The late answer for the remembered folder moves nothing.
    assert_eq!(
        stage.fail("/r/a/b", ListingErrorKind::NotFound),
        ListingDelivery::Stale
    );
    assert_eq!(stage.current(), PathBuf::from("/r/samples"));
    assert_eq!(stage.awaited(), [PathBuf::from("/r/samples")]);
    // Nor does the place's own failure start a search.
    stage.fail("/r/samples", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r/samples"));
}

#[test]
fn a_reopened_dialog_is_not_moved_by_the_earlier_sessions_search() {
    let mut stage = stage_with(&remembering("/r/a/b"));
    stage.open(folder_in("project"));
    assert!(stage.dialog.cancel().unwrap());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    stage.open(folder_in("export").start_folder("/r/z"));
    let late = stage.take_request(Path::new("/r/a/b"));
    assert_eq!(
        stage
            .dialog
            .deliver(late.respond(Err(ListingError::new(ListingErrorKind::NotFound, "gone"))))
            .unwrap(),
        ListingDelivery::Stale
    );
    settle(&mut stage.ui);
    assert_eq!(stage.current(), PathBuf::from("/r/z"));
    assert_eq!(stage.awaited(), [PathBuf::from("/r/z")]);
}

#[test]
fn dialogs_sharing_a_memory_keep_each_others_updates() {
    let memory = FileDialogMemory::new();
    let (mut first, mut second) = (stage_with(&memory), stage_with(&memory));
    // Both open before either accepts: neither writes back what it opened with.
    first.open(folder_in("project").start_folder("/r/one"));
    second.open(folder_in("export").start_folder("/r/two"));
    first.answer("/r/one", listing());
    second.answer("/r/two", listing());
    assert!(first.confirm());
    assert!(second.confirm());
    assert_eq!(memory.last_folder(Some("project")), Some("/r/one".into()));
    assert_eq!(memory.last_folder(Some("export")), Some("/r/two".into()));
    assert_eq!(
        memory.recent_folders(),
        [PathBuf::from("/r/two"), PathBuf::from("/r/one")]
    );
    let (_, _) = (first.outcome(), second.outcome());
    settle(&mut first.ui);
    settle(&mut second.ui);

    // For one context the later accept wins.
    first.open(folder_in("project").start_folder("/r/a"));
    second.open(folder_in("project").start_folder("/r/b"));
    first.answer("/r/a", listing());
    second.answer("/r/b", listing());
    assert!(second.confirm());
    assert!(first.confirm());
    assert_eq!(memory.last_folder(Some("project")), Some("/r/a".into()));
}

#[cfg(unix)]
#[test]
fn a_folder_whose_path_is_not_unicode_is_not_remembered() {
    use std::os::unix::ffi::OsStrExt;
    let odd = PathBuf::from("/r").join(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    let memory = remembering("/r/sub");
    let mut stage = stage_with(&memory);
    stage.open(folder_in("project").start_folder(&odd));
    let request = stage.take_request(&odd);
    stage
        .dialog
        .deliver(request.respond(Ok(Vec::new())))
        .unwrap();
    settle(&mut stage.ui);
    assert!(stage.confirm());
    match stage.outcome() {
        Some(FileDialogOutcome::Accepted(selection)) => {
            assert_eq!(selection.paths(), [odd]);
            assert!(!selection.folder_remembered());
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(memory, remembering("/r/sub"), "nothing lossy is stored");
}

#[test]
fn places_and_recent_folders_are_offered_beside_the_browser() {
    let memory = FileDialogMemory::new();
    let mut stage = stage_with(&memory);
    stage.opened(request(FileDialogMode::ChooseFolder));
    let left = |stage: &Stage| {
        let browser = widget(&stage.ui, &stage.root, "browser");
        stage.ui.bounds(&browser).unwrap().expect("laid out").x()
    };
    let alone = left(&stage);
    assert!(
        stage
            .ui
            .bounds(&widget(&stage.ui, &stage.root, "shortcuts"))
            .unwrap()
            .is_none(),
        "no column without places or recent folders"
    );
    stage
        .dialog
        .places()
        .set(vec![FilePlace::new("Samples", "/r/samples").unwrap()])
        .unwrap();
    settle(&mut stage.ui);
    assert!(left(&stage) > alone, "the browser makes room");
    let place = widget(&stage.ui, &stage.root, "place-0");
    assert_eq!(
        support::semantics_of(&stage.ui, &place).and_then(|semantics| semantics.name),
        Some("Samples".to_owned())
    );
    stage.click(&place);
    assert_eq!(stage.current(), PathBuf::from("/r/samples"));
    stage.answer("/r/samples", listing());
    assert!(stage.confirm());
    let _ = stage.outcome();
    settle(&mut stage.ui);

    // The accepted folder is a recent one in the next session.
    stage.opened(request(FileDialogMode::ChooseFolder).start_folder("/r"));
    let recent = widget(&stage.ui, &stage.root, "recent-0");
    assert_eq!(
        support::semantics_of(&stage.ui, &recent).and_then(|semantics| semantics.name),
        Some("samples".to_owned())
    );
    stage.click(&recent);
    assert_eq!(stage.current(), PathBuf::from("/r/samples"));
}

#[test]
fn a_place_needs_a_label_and_a_folder() {
    for (label, folder) in [("", "/r"), ("  ", "/r"), ("a\nb", "/r"), ("Home", "")] {
        assert_eq!(
            FilePlace::new(label, folder),
            Err(FileDialogError::InvalidPlace),
            "{label:?} {folder:?}"
        );
    }
    let place = FilePlace::new("Home", "/home/ada").unwrap();
    assert_eq!(
        (place.label(), place.folder()),
        ("Home", Path::new("/home/ada"))
    );
}

#[test]
fn the_memory_document_round_trips_and_is_deterministic() {
    let memory = FileDialogMemory::new();
    let mut stage = stage_with(&memory);
    for (context, folder) in [(None, "/r/plain"), (Some("project"), "/r/a \"b\" \\c")] {
        let asked = request(FileDialogMode::OpenFile)
            .filters(text_filters())
            .unwrap()
            .start_folder(folder);
        let asked = match context {
            Some(context) => asked.context(context).unwrap(),
            None => asked,
        };
        stage.open(asked);
        stage.answer(folder, vec![file(&format!("{folder}/a.txt"))]);
        stage.click_row(0);
        assert!(stage.confirm());
        let _ = stage.outcome();
        settle(&mut stage.ui);
    }
    let text = memory.encode();
    assert_eq!(
        text,
        "pert-ui-file-dialog-memory 1\n\
         default folder=\"/r/plain\" filter=\"txt\"\n\
         context \"project\" folder=\"/r/a \\\"b\\\" \\\\c\" filter=\"txt\"\n\
         recent \"/r/a \\\"b\\\" \\\\c\"\n\
         recent \"/r/plain\"\n"
    );
    let decoded = FileDialogMemory::decode(&text).unwrap();
    assert_eq!(decoded, memory);
    assert_eq!(decoded.encode(), text);
    // A transport's line layout is tolerated and encodes canonically again.
    let loose = format!(
        "\n{}",
        text.replace('\n', "\r\n\r\n")
            .replace(" filter", "   filter")
    );
    assert_eq!(FileDialogMemory::decode(&loose).unwrap().encode(), text);
    memory.clear();
    assert_eq!(memory, FileDialogMemory::new());
}

#[test]
fn an_invalid_memory_document_is_rejected_whole() {
    use FileDialogMemoryError as E;
    let head = "pert-ui-file-dialog-memory 1\n";
    let malformed = |body: &str| {
        let result = FileDialogMemory::decode(&format!("{head}{body}"));
        assert!(
            matches!(result, Err(E::Malformed(_))),
            "{body:?}: {result:?}"
        );
    };
    assert!(matches!(FileDialogMemory::decode(""), Err(E::Malformed(_))));
    assert!(matches!(
        FileDialogMemory::decode("pert-ui-workspace 1\n"),
        Err(E::Malformed(_))
    ));
    for version in ["0", "2"] {
        assert_eq!(
            FileDialogMemory::decode(&format!("pert-ui-file-dialog-memory {version}\n")),
            Err(E::UnsupportedVersion {
                found: version.parse().unwrap(),
                supported: 1
            })
        );
    }
    malformed("size width=\"1\"\n");
    malformed("default\n");
    malformed("default folder=\"/a\" colour=\"red\"\n");
    malformed("default folder=\"/a\" folder=\"/b\"\n");
    malformed("default folder=\"/a\\n\"\n");
    malformed("default folder=\"/a\"\ndefault folder=\"/b\"\n");
    malformed("recent \"/a\"\ncontext \"x\" folder=\"/a\"\n");
    malformed("context \"x\" folder=\"/a\"\ndefault folder=\"/a\"\n");
    malformed("recent \"/a\" \"/b\"\n");
    malformed("recent /a\n");
    // Tokens that run together, and a stray carriage return, are no layout.
    malformed("context\"x\" folder=\"/a\"\n");
    malformed("context \"x\"folder=\"/a\"\n");
    malformed("default folder=\"/a\"filter=\"f\"\n");
    malformed("recent\"/a\"\n");
    malformed("recent \"/a\"\r\r\n");
    let decode = |body: &str| FileDialogMemory::decode(&format!("{head}{body}"));
    assert_eq!(
        decode("context \"  \" folder=\"/a\"\n"),
        Err(E::InvalidKey("  ".to_owned()))
    );
    assert_eq!(
        decode("default filter=\"\"\n"),
        Err(E::InvalidKey(String::new()))
    );
    assert_eq!(decode("default folder=\"\"\n"), Err(E::InvalidPath));
    assert_eq!(decode("recent \"a\tb\"\n"), Err(E::InvalidPath));
    assert_eq!(
        decode(&format!("recent \"/{}\"\n", "a".repeat(4096))),
        Err(E::InvalidPath)
    );
    assert_eq!(
        decode("context \"x\" folder=\"/a\"\ncontext \"x\" folder=\"/b\"\n"),
        Err(E::DuplicateContext("x".to_owned()))
    );
    assert_eq!(
        decode("recent \"/a\"\nrecent \"/a\"\n"),
        Err(E::DuplicateRecent("/a".to_owned()))
    );
    // The same folder under another spelling is the same folder.
    assert_eq!(
        decode("recent \"/a/b\"\nrecent \"/a/./b\"\n"),
        Err(E::DuplicateRecent("/a/./b".to_owned()))
    );
    let recents = |count: usize| {
        (0..count)
            .map(|index| format!("recent \"/r/{index}\"\n"))
            .collect::<String>()
    };
    assert!(decode(&recents(FileDialogMemory::MAX_RECENT)).is_ok());
    assert_eq!(
        decode(&recents(FileDialogMemory::MAX_RECENT + 1)),
        Err(E::TooManyRecent)
    );
    let contexts = |count: usize| {
        (0..count)
            .map(|index| format!("context \"c{index}\" folder=\"/r\"\n"))
            .collect::<String>()
    };
    assert!(decode(&contexts(FileDialogMemory::MAX_CONTEXTS)).is_ok());
    assert_eq!(
        decode(&contexts(FileDialogMemory::MAX_CONTEXTS + 1)),
        Err(E::TooManyContexts)
    );
    // One invalid entry in an otherwise valid document rejects all of it.
    assert_eq!(
        decode("context \"x\" folder=\"/a\"\nrecent \"/a\"\nrecent \"\"\n"),
        Err(E::InvalidPath)
    );
    assert_eq!(
        FileDialogMemory::decode(&" ".repeat(FileDialogMemory::MAX_DOCUMENT_BYTES + 1)),
        Err(E::TooLarge)
    );
}

#[test]
fn the_search_stops_when_the_user_opens_the_parent_in_the_browser() {
    let mut stage = stage_with(&remembering("/r/a/b"));
    stage.open(folder_in("project"));
    let entries = widget(&stage.ui, &stage.root, "entries");
    stage.ui.focus(&entries).unwrap();
    press(&mut stage.ui, KeyCode::Backspace);
    settle(&mut stage.ui);
    assert_eq!(stage.current(), PathBuf::from("/r/a"));
    assert_eq!(
        stage.fail("/r/a/b", ListingErrorKind::NotFound),
        ListingDelivery::Stale
    );
    // The folder the user opened fails: it is shown, not searched past.
    stage.fail("/r/a", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r/a"));
    assert_eq!(stage.awaited(), Vec::<PathBuf>::new());
}

#[test]
fn another_folders_listing_neither_ends_nor_advances_the_search() {
    // A kept listing of the remembered folder shows while the fresh one is
    // due: the tree's listing arriving first must not end the search.
    let memory = FileDialogMemory::new();
    let mut stage = stage_with(&memory);
    let asked = || {
        request(FileDialogMode::OpenFile)
            .context("project")
            .unwrap()
    };
    stage.accept_in(asked().start_folder("/r/sub"), "/r/sub");
    stage.open(asked());
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    stage.answer("/r", listing());
    assert_eq!(stage.current(), PathBuf::from("/r/sub"));
    stage.fail("/r/sub", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r"), "the parent is tried");

    // A kept failure of the remembered folder shows while the fresh answer
    // is due: another folder's listing must not move the dialog on.
    let mut stage = stage_with(&remembering("/r/x"));
    stage.open(folder_in("other").start_folder("/r/x"));
    stage.fail("/r/x", ListingErrorKind::NotFound);
    assert!(stage.dialog.cancel().unwrap());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    stage.open(folder_in("project"));
    stage.answer("/r", listing());
    assert_eq!(stage.current(), PathBuf::from("/r/x"));
    stage.answer("/r/x", Vec::new());
    assert_eq!(stage.current(), PathBuf::from("/r/x"), "it is there again");
}

#[test]
fn the_search_goes_on_above_the_first_root() {
    let mut stage = stage_with(&remembering("/r/a"));
    stage.open(folder_in("project"));
    stage.fail("/r/a", ListingErrorKind::NotFound);
    assert_eq!(stage.current(), PathBuf::from("/r"));
    stage.fail("/r", ListingErrorKind::NotFound);
    assert_eq!(
        stage.current(),
        PathBuf::from("/"),
        "the root is an ancestor too"
    );
    let pending = stage.requests.borrow().len();
    stage.fail("/", ListingErrorKind::NotFound);
    // The first root was tried on the way: nothing is requested again.
    assert_eq!(stage.current(), PathBuf::from("/"));
    assert_eq!(stage.requests.borrow().len(), pending - 1);
}

#[test]
fn a_memory_given_during_a_session_is_the_next_sessions() {
    let (first, second) = (FileDialogMemory::new(), FileDialogMemory::new());
    let mut stage = stage_with(&first);
    stage.open(request(FileDialogMode::OpenFile).start_folder("/r/one"));
    let _ = stage.dialog.clone().with_memory(&second);
    stage.answer("/r/one", vec![file("/r/one/a.txt")]);
    stage.click_row(0);
    assert!(stage.confirm());
    let _ = stage.outcome();
    settle(&mut stage.ui);
    assert_eq!(first.last_folder(None), Some("/r/one".into()));
    assert_eq!(second, FileDialogMemory::new());
    stage.accept_in(
        request(FileDialogMode::OpenFile).start_folder("/r/two"),
        "/r/two",
    );
    assert_eq!(first.last_folder(None), Some("/r/one".into()));
    assert_eq!(second.last_folder(None), Some("/r/two".into()));
}

#[test]
fn a_dismissed_session_leaves_the_memory_as_it_was() -> TestResult {
    let memory = remembering("/r/sub");
    let mut stage = stage_with(&memory);
    stage.open(folder_in("project").start_folder("/r/else"));
    stage.answer("/r/else", listing());
    stage.ui.set_visible(&stage.root, false)?;
    settle(&mut stage.ui);
    assert!(matches!(
        stage.outcome(),
        Some(FileDialogOutcome::Dismissed(_))
    ));
    assert_eq!(memory, remembering("/r/sub"));
    Ok(())
}

#[test]
fn a_reordered_place_is_not_opened_by_a_press_on_another() {
    let mut stage = stage();
    let place = |label: &str, folder: &str| FilePlace::new(label, folder).unwrap();
    let places = stage.dialog.places();
    places
        .set(vec![place("One", "/r/one"), place("Two", "/r/two")])
        .unwrap();
    stage.opened(request(FileDialogMode::ChooseFolder));
    let one = widget(&stage.ui, &stage.root, "place-0");
    let point = stage.point(&one);
    stage
        .ui
        .pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))
        .unwrap();
    // The list is reordered under the held press.
    places
        .set(vec![place("Two", "/r/two"), place("One", "/r/one")])
        .unwrap();
    settle(&mut stage.ui);
    assert_eq!(
        widget(&stage.ui, &stage.root, "place-0"),
        one,
        "One keeps its key"
    );
    stage
        .ui
        .pointer_input(PointerEvent::new(
            point,
            PointerKind::Release(PointerButton::Primary),
        ))
        .unwrap();
    settle(&mut stage.ui);
    assert_ne!(stage.current(), PathBuf::from("/r/two"));
    // A place given twice is one item.
    places
        .set(vec![place("Two", "/r/two"), place("Two", "/r/two")])
        .unwrap();
    settle(&mut stage.ui);
    let shortcuts = widget(&stage.ui, &stage.root, "shortcuts");
    assert!(
        stage
            .ui
            .find_descendant(&shortcuts, "place-1")
            .unwrap()
            .is_some()
    );
    assert!(
        stage
            .ui
            .find_descendant(&shortcuts, "place-0")
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_place_scrolled_out_of_view_is_reached_and_opened_by_keyboard() {
    let mut stage = stage();
    let places = (0..40)
        .map(|index| FilePlace::new(format!("Place {index}"), format!("/r/p{index}")).unwrap())
        .collect::<Vec<_>>();
    stage.dialog.places().set(places).unwrap();
    stage.opened(request(FileDialogMode::ChooseFolder));
    let last = widget(&stage.ui, &stage.root, "place-39");
    stage.ui.focus(&last).unwrap();
    settle(&mut stage.ui);
    press(&mut stage.ui, KeyCode::Enter);
    settle(&mut stage.ui);
    assert_eq!(stage.current(), PathBuf::from("/r/p39"));
}

#[test]
fn recent_folders_that_share_a_name_are_shown_by_their_paths() {
    let memory = FileDialogMemory::decode(
        "pert-ui-file-dialog-memory 1\n\
         recent \"/r/a/src\"\nrecent \"/r/b/src\"\nrecent \"/r/docs\"\n",
    )
    .unwrap();
    let mut stage = stage_with(&memory);
    stage.opened(request(FileDialogMode::ChooseFolder));
    let name = |stage: &Stage, key: &str| {
        let item = widget(&stage.ui, &stage.root, key);
        support::semantics_of(&stage.ui, &item)
            .and_then(|semantics| semantics.name)
            .unwrap()
    };
    assert_eq!(name(&stage, "recent-0"), "/r/a/src");
    assert_eq!(name(&stage, "recent-1"), "/r/b/src");
    assert_eq!(name(&stage, "recent-2"), "docs");
}
