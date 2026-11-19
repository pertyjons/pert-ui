//! The file browser over a synthetic sample library, answered through a
//! mailbox as a host answers from a worker thread.
use crate::gallery::{label, page, section, stage, toggle};
use pert_ui::live::*;
use pert_ui::prelude::*;
use std::{
    cell::{OnceCell, RefCell},
    path::Path,
    rc::Rc,
    time::Duration,
};

/// The library's root folder.
pub(crate) const ROOT: &str = "/Library";

/// Lists a folder of the synthetic library. Pure and `Send`, so a worker can
/// call it, as a real host would call `list_directory`.
pub(crate) fn listing(directory: &Path) -> Result<Vec<DirectoryEntry>, ListingError> {
    let at = |name: &str| directory.join(name);
    let folder = |name: &str| DirectoryEntry::new(at(name), EntryKind::Directory);
    let file = |name: &str| DirectoryEntry::new(at(name), EntryKind::File);
    let numbered = |prefix: &str, count: usize| {
        (1..=count)
            .map(|index| file(&format!("{prefix}-{index:02}.wav")))
            .collect::<Vec<_>>()
    };
    let relative = directory
        .strip_prefix(ROOT)
        .map_err(|_| ListingError::new(ListingErrorKind::NotFound, "outside the sample library"))?;
    let relative = relative.to_string_lossy();
    Ok(match relative.as_ref() {
        "" => vec![
            folder("Drums"),
            folder("Synths"),
            folder("Projects"),
            folder("Locked"),
            DirectoryEntry::new(
                at("Latest project"),
                EntryKind::Symlink(LinkTarget::Directory),
            ),
            DirectoryEntry::new(at("Old link"), EntryKind::Symlink(LinkTarget::Broken)),
            file("README.txt"),
            DirectoryEntry::unreadable(
                at("damaged.bin"),
                ListingError::new(ListingErrorKind::Other, "the entry could not be read"),
            ),
        ],
        "Drums" => vec![folder("Kicks"), folder("Snares"), file("loop-120.wav")],
        "Drums/Kicks" => numbered("kick", 40),
        "Drums/Snares" => numbered("snare", 12),
        "Synths" => vec![folder("Pads"), file("bass.wav"), file("lead.wav")],
        "Synths/Pads" => numbered("pad", 6),
        "Projects" | "Latest project" => {
            vec![folder("Mixes"), file("song.project"), file("notes.txt")]
        }
        "Projects/Mixes" | "Latest project/Mixes" => vec![],
        "Locked" => {
            return Err(ListingError::new(
                ListingErrorKind::PermissionDenied,
                "Permission denied",
            ));
        }
        _ => {
            return Err(ListingError::new(
                ListingErrorKind::NotFound,
                "No such folder",
            ));
        }
    })
}

/// Sends a response produced on the UI thread, which must not wait: when the
/// mailbox is full the response joins `backlog`, which the mailbox handler
/// delivers after the queued values. A closed mailbox means the page and its
/// browser are gone, so the response has nowhere to go and is dropped.
pub(crate) fn send_or_keep(
    sender: &UiSender<DirectoryResponse>,
    backlog: &RefCell<Vec<DirectoryResponse>>,
    response: DirectoryResponse,
) {
    match sender.send(response) {
        Ok(()) | Err(SendError::Closed(_)) => {}
        Err(SendError::Full(refused)) => backlog.borrow_mut().push(refused),
    }
}

pub(crate) fn file_browser(ui: &mut Runtime) -> Result<Element, AppError> {
    // The mailbox stands in for the channel back from a worker thread: its
    // handler runs on the UI thread at the start of a flush and delivers.
    let slot: Rc<OnceCell<FileBrowserController>> = Rc::new(OnceCell::new());
    let deliveries = ui.signal((0_u32, 0_u32));
    // Responses produced on the UI thread while the mailbox was full. A full
    // mailbox has undelivered values, so its handler runs at the next flush
    // and delivers these after them; none is lost.
    let backlog: Rc<RefCell<Vec<DirectoryResponse>>> = Rc::default();
    let (target, counts, waiting) = (slot.clone(), deliveries.clone(), backlog.clone());
    let mailbox = ui.mailbox(Capacity::new(64)?, move |response: DirectoryResponse| {
        let Some(controller) = target.get() else {
            return Ok(());
        };
        let queued = std::mem::take(&mut *waiting.borrow_mut());
        for response in std::iter::once(response).chain(queued) {
            let (applied, stale) = counts.get()?;
            counts.set(match controller.deliver(response)? {
                ListingDelivery::Stale => (applied, stale + 1),
                _ => (applied + 1, stale),
            })?;
        }
        Ok(())
    })?;
    let slow = ui.signal(false);
    let (sender, latency) = (mailbox.sender(), slow.clone());
    let controller = FileBrowserController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Multiple),
        [ROOT],
        move |request| {
            let sender = sender.clone();
            // The callback must not block: a slow listing waits on a thread.
            if latency.get().unwrap_or(false) {
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(600));
                    let listing = listing(request.directory());
                    let mut response = request.respond(listing);
                    // A worker may wait: retry while the mailbox is full.
                    loop {
                        match sender.send(response) {
                            Ok(()) => break,
                            Err(SendError::Full(refused)) => {
                                response = refused;
                                std::thread::sleep(Duration::from_millis(10));
                            }
                            // The page and its browser are gone: nothing is
                            // left to show the listing.
                            Err(SendError::Closed(_)) => break,
                        }
                    }
                });
            } else {
                let listing = listing(request.directory());
                send_or_keep(&sender, &backlog, request.respond(listing));
            }
        },
    )
    .map_err(ApplicationError::new)?;
    let _ = slot.set(controller.clone());
    let opened = ui.signal("Double-click a file or press Enter on it.".to_owned());
    let sink = opened.clone();
    // The runtime keeps the mailbox until it shuts down, as the browser
    // that answers through it lives as long as the gallery.
    ui.keep_alive(mailbox);
    controller.on_open(move |_, entry| sink.set(format!("Opened {}", entry.path().display())));
    let paths = controller.clone();
    let selected = controller.selection().selected().map(move |keys| {
        let mut names: Vec<String> = keys
            .iter()
            .filter_map(|key| paths.path_of(key))
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        names.sort();
        match names.len() {
            0 => "Selected: nothing".to_owned(),
            _ => format!("Selected: {}", names.join(", ")),
        }
    })?;
    let counted = deliveries.map(|(applied, stale)| {
        format!("Listings applied: {applied}; stale results dropped: {stale}")
    })?;
    let refresh = controller.clone();
    let dialog_section = file_dialog(ui)?;
    Ok(page([
        section(
            "browser-section",
            "A file browser over a host-supplied source",
            [
                label(
                    "browser-help",
                    "The browser asks the host for each folder and shows what arrives; it never reads the disk. Expand folders in the tree or click one to open it, double-click or press Enter on an entry, filter the entries by name, filter the folder tree from the field above it (it searches only folders already listed, so it never asks for a listing), and press Backspace in a list for the parent folder. Locked fails with an error row, Latest project is a link to a folder, and Kicks has enough samples to scroll. Turn on slow listing to see loading rows, then open another folder before one arrives: its late result is dropped as stale.",
                ),
                toggle("slow", "Slow listing (600 ms on a thread)", &slow)?,
                Element::new(
                    "refresh",
                    Button::new("Refresh", move |_| refresh.refresh()),
                ),
                stage(
                    "browser-stage",
                    360,
                    Element::new(
                        "library",
                        FileBrowser::new(&controller, 24.0)?.folder_filter(true),
                    ),
                ),
                Element::new("opened", Label::new(opened.read_only())),
                Element::new("selected", Label::new(selected)),
                Element::new("deliveries", Label::new(counted)),
            ],
        ),
        dialog_section,
    ]))
}

/// What the page says about an ended file dialog session.
fn describe(outcome: FileDialogOutcome) -> String {
    match outcome {
        FileDialogOutcome::Accepted(selection) => {
            let paths: Vec<String> = selection
                .paths()
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
            match selection.filter() {
                Some(filter) => format!("Chose {} as {filter}", paths.join(", ")),
                None => format!("Chose {}", paths.join(", ")),
            }
        }
        FileDialogOutcome::Cancelled => "Cancelled".to_owned(),
        other => format!("Ended without a choice: {other:?}"),
    }
}

/// A file dialog over the same synthetic library, one request per button.
fn file_dialog(ui: &mut Runtime) -> Result<Element, AppError> {
    let slot: Rc<OnceCell<FileDialogController>> = Rc::new(OnceCell::new());
    let backlog: Rc<RefCell<Vec<DirectoryResponse>>> = Rc::default();
    let (target, waiting) = (slot.clone(), backlog.clone());
    let mailbox = ui.mailbox(Capacity::new(64)?, move |response: DirectoryResponse| {
        let Some(dialog) = target.get() else {
            return Ok(());
        };
        let queued = std::mem::take(&mut *waiting.borrow_mut());
        for response in std::iter::once(response).chain(queued) {
            dialog.deliver(response)?;
        }
        Ok(())
    })?;
    let sender = mailbox.sender();
    let dialog = FileDialogController::new(ui, [ROOT], move |request| {
        let listing = listing(request.directory());
        send_or_keep(&sender, &backlog, request.respond(listing));
    })
    .map_err(ApplicationError::new)?;
    let _ = slot.set(dialog.clone());
    ui.keep_alive(mailbox);
    let report = ui.signal("Open a dialog to choose something.".to_owned());
    let audio = || -> Result<Vec<FileFilter>, FileDialogError> {
        Ok(vec![
            FileFilter::new("wav", "Samples (*.wav)", ["wav"])?,
            FileFilter::new("text", "Notes (*.txt)", ["txt"])?,
            FileFilter::all("all", "All files")?,
        ])
    };
    let opener = |key: &str, title: &str, request: FileDialogRequest| {
        let dialog = dialog.clone();
        Element::new(
            key,
            Button::new(title, move |_| match dialog.open(request.clone()) {
                // A press while a session is open is ignored.
                Ok(()) | Err(FileDialogError::Busy) => Ok(()),
                Err(FileDialogError::Runtime(error)) => Err(error),
                Err(error) => Err(ApplicationError::new(error).into()),
            }),
        )
    };
    let filters = audio().map_err(ApplicationError::new)?;
    let buttons = Row::new([
        opener(
            "open-file",
            "Open file…",
            FileDialogRequest::new(FileDialogMode::OpenFile)
                .filters(filters.clone())
                .map_err(ApplicationError::new)?,
        ),
        opener(
            "open-files",
            "Open files…",
            FileDialogRequest::new(FileDialogMode::OpenFiles)
                .start_folder(Path::new(ROOT).join("Drums/Kicks")),
        ),
        opener(
            "choose-folder",
            "Choose folder…",
            FileDialogRequest::new(FileDialogMode::ChooseFolder),
        ),
        opener(
            "save",
            "Save as…",
            FileDialogRequest::new(FileDialogMode::SaveFile)
                .title("Save notes")
                .start_folder(Path::new(ROOT).join("Projects"))
                // Notes first, so the suggested name completes to notes.txt.
                .filters([filters[1].clone(), filters[0].clone(), filters[2].clone()])
                .and_then(|request| request.suggested_name("notes"))
                .map_err(ApplicationError::new)?,
        ),
    ])
    .gap(MetricRole::ItemGap);
    let anchor = Element::new(
        "dialog-anchor",
        Column::new([
            Element::new("dialog-buttons", buttons),
            Element::new("dialog-result", Label::new(report.read_only())),
        ])
        .gap(MetricRole::ItemGap),
    );
    Ok(section(
        "dialog-section",
        "A file dialog",
        [
            label(
                "dialog-help",
                "Each button opens the same dialog with another request: one file of a chosen type, several files, a folder, or a name to save under. The dialog is modal to the window and hands each outcome to the page when the session ends. Save as suggests notes, which completes to the existing notes.txt, so confirming asks before replacing it; choose another type to save under another extension. damaged.bin and Old link cannot be opened and are refused in place.",
            ),
            Element::new(
                "file-dialog",
                FileDialog::in_window(ui, &dialog, anchor)
                    .on_outcome(move |_, outcome| report.set(describe(outcome))),
            ),
        ],
    ))
}
