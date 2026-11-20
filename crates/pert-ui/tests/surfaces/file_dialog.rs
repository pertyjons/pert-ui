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
    let mut ui = Runtime::new();
    ui.set_viewport(viewport).unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let dialog =
        FileDialogController::new(&ui, ["/r"], move |request| queue.borrow_mut().push(request))
            .unwrap();
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
