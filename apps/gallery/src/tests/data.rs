//! The Data & media category's pages.
use super::support::*;
use pert_ui::*;
use std::time::Duration;

#[test]
fn static_live_wheel_scrolling_reuses_prepared_controls() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "live");
    let scroll = find(&ui, &root, "static-scroll");
    let trace = find(&ui, &root, "live-trace");
    let first = find(&ui, &root, "static-0000");
    for (width, scale) in [(960.0, 1.0), (320.0, 1.25)] {
        ui.set_viewport(Viewport::new(width, 640.0, scale).unwrap())
            .unwrap();
        ui.flush(ui.now()).unwrap();
        ui.paint().unwrap();
        let scroll_bounds = ui.bounds(&scroll).unwrap();
        let [x, y, w, h] = scroll_bounds.unwrap().values();
        assert!(h > 0.0 && y >= 0.0 && y + h <= 640.0);
        let position = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        let trace_bounds = ui.bounds(&trace).unwrap();
        let before_y = ui.bounds(&first).unwrap().unwrap().y();
        ui.clear_profile().unwrap();
        for _ in 0..10 {
            assert_eq!(
                ui.pointer_input(PointerEvent::new(
                    position,
                    PointerKind::Scroll(Point::new(0.0, 40.25).unwrap()),
                ))
                .unwrap(),
                InputOutcome::Handled
            );
            ui.flush(ui.now()).unwrap();
            ui.paint().unwrap();
            assert_eq!(ui.bounds(&trace).unwrap(), trace_bounds);
            assert_eq!(ui.bounds(&scroll).unwrap(), scroll_bounds);
        }
        assert_eq!(ui.bounds(&first).unwrap().unwrap().y(), before_y - 402.5);
        // Scrolling may measure ancestor chrome and repaint changed hover states,
        // but retained controls must not measure or arrange newly revealed rows.
        for row in &ui.profile().stats {
            assert!(
                !matches!(
                    row.operation,
                    ProfileOperation::Measure | ProfileOperation::Arrange
                ) || !row.key.strip_prefix("static-").is_some_and(|suffix| {
                    suffix.len() == 4 && suffix.bytes().all(|byte| byte.is_ascii_digit())
                }),
                "unexpected control work: {row:?}"
            );
        }
        assert!(ui.validation_report().issues.is_empty());
        assert!(ui.next_deadline().is_none());
        assert!(!ui.needs_paint());
    }
}

#[test]
fn static_live_gallery_tab_retains_controls_and_suspends_presentation() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "live");
    let control = find(&ui, &root, "static-0000");
    ui.focus(&control).unwrap();
    press(&mut ui, KeyCode::Space);
    let run = find(&ui, &root, "run-live");
    ui.focus(&run).unwrap();
    press(&mut ui, KeyCode::Space);
    ui.paint().unwrap();
    assert!(ui.next_deadline().is_some());
    open(&mut ui, &root, "layout");
    ui.paint().unwrap();
    assert_eq!(ui.next_deadline(), None);
    assert_eq!(
        ui.flush(Duration::from_secs(30))
            .unwrap()
            .components_updated,
        0
    );
    assert!(!ui.needs_paint());
    open(&mut ui, &root, "live");
    assert_eq!(find(&ui, &root, "static-0000"), control);
    assert_eq!(
        ui.inspect()
            .components
            .iter()
            .find(|item| item.key == "static-0000")
            .unwrap()
            .details
            .properties["checked"],
        InspectValue::Bool(true)
    );
    assert!(ui.next_deadline().is_some());
    ui.paint().unwrap();
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn virtual_list_tab_mounts_a_bounded_slice_and_reveals_by_key() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "virtual");
    let catalog = find(&ui, &root, "catalog");
    let mounted = ui.children(&catalog).unwrap().len();
    assert!(mounted > 0 && mounted < 40, "{mounted} rows mounted");
    let entry = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "catalog")
        .unwrap();
    assert_eq!(
        entry.details.properties["rows"],
        InspectValue::Number(100_000.0)
    );
    let jump = find(&ui, &root, "jump-last");
    click(&mut ui, &jump);
    let last = find(&ui, &root, "entry-99999");
    let [_, y, _, h] = ui.bounds(&last).unwrap().unwrap().values();
    let [_, top, _, height] = ui.bounds(&catalog).unwrap().unwrap().values();
    assert!(
        y >= top && y + h <= top + height,
        "revealed row is inside the list"
    );
    assert!(ui.children(&catalog).unwrap().len() < 40);
    assert_eq!(text(&ui, "jump-status"), ["Row reveal: found and mounted"]);
    // A key no row has completes the receipt as not found instead of
    // leaving the jump silently unanswered.
    let missing = find(&ui, &root, "jump-missing");
    click(&mut ui, &missing);
    assert_eq!(
        text(&ui, "jump-status"),
        ["Row reveal: no row has that key"]
    );
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn the_collections_page_navigates_a_grid_a_lazy_tree_and_a_sorted_table() {
    let (mut ui, root) = mounted();
    // Tall enough to show all three collections; one below the fold is clipped
    // away and cannot take focus.
    ui.set_viewport(Viewport::new(960.0, 2000.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "collections");
    let key = |ui: &mut Runtime, code: KeyCode| {
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    // Every collection mounts a bounded number of rows, whatever its size.
    for (collection, limit) in [("collections-grid", 80), ("collections-tree", 40)] {
        let id = find(&ui, &root, collection);
        assert!(ui.children(&id).unwrap().len() < limit, "{collection}");
    }
    let grid = find(&ui, &root, "collections-grid");
    ui.focus(&grid).unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::ArrowRight);
    assert!(text(&ui, "grid-active").contains(&"Active item: swatch-6".into()));

    // Expanding a folder asks for its twenty children and nothing deeper.
    let tree = find(&ui, &root, "collections-tree");
    ui.focus(&tree).unwrap();
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::ArrowRight);
    key(&mut ui, KeyCode::ArrowRight);
    assert!(text(&ui, "tree-active").contains(&"Active item: folder-1/folder-1".into()));

    let table = find(&ui, &root, "collections-table");
    let body = ui.children(&table).unwrap()[0].clone();
    assert!(ui.children(&body).unwrap().len() < 40);
    ui.focus(&body).unwrap();
    key(&mut ui, KeyCode::End);
    assert!(text(&ui, "table-active").contains(&"Active item: sample-100000".into()));
    // Right enters the first column and then walks past the visible ones, which
    // scrolls the columns; Enter reports the cell.
    for _ in 0..6 {
        key(&mut ui, KeyCode::ArrowRight);
    }
    key(&mut ui, KeyCode::Enter);
    assert!(text(&ui, "table-cell").contains(&"Activated sample-100000, column note.".into()));

    // Rows of their own height stack without gaps and stay bounded.
    let notes = find(&ui, &root, "collections-notes");
    let rows = ui.children(&notes).unwrap();
    assert!(rows.len() < 40, "{} notes mounted", rows.len());
    let bounds: Vec<Rect> = rows
        .iter()
        .map(|row| ui.bounds(row).unwrap().unwrap())
        .collect();
    assert!(
        bounds
            .windows(2)
            .any(|pair| pair[0].height() != pair[1].height())
    );
    for pair in bounds.windows(2) {
        assert!((pair[0].y() + pair[0].height() - pair[1].y()).abs() < 0.01);
    }
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_images_page_decodes_its_png_and_shares_one_upload_per_image() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 1200.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "images");
    let scene = ui.paint().unwrap();
    // One decoded badge drawn three times, one checker drawn twice and the
    // rasterized SVG dial once, plus the pert-ui mark in the menu bar: four
    // entries in the scene, seven commands. The page's monochrome marks are
    // coverage masks instead, so they add none.
    assert_eq!(scene.images().len(), 4);
    let draws = scene
        .commands()
        .iter()
        .filter(|command| matches!(command, DrawCommand::DrawImage { .. }))
        .count();
    assert_eq!(draws, 7);
    assert!(text(&ui, "formats").iter().any(|line| line.contains("Png")));
    let contain = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| {
            component
                .details
                .semantics
                .as_ref()
                .is_some_and(|s| s.role == SemanticRole::Image && s.name.is_some())
                // A `Trace` is a named image too; a raster image reports pixels.
                && component.details.properties.contains_key("pixels")
        })
        .unwrap();
    assert_eq!(
        contain.details.properties.get("pixels"),
        Some(&InspectValue::Text("96x64".into()))
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none(), "images schedule nothing");
}

#[test]
fn the_images_page_tints_its_svg_marks_and_keeps_the_dial_in_color() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 1200.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "images");
    let scene = ui.paint().unwrap();
    let theme = ui.theme();
    let palette = theme.palette(theme.mode().unwrap()).unwrap();
    // Each mark is coverage the theme colors, so three marks in three roles are
    // three tints over three masks and no image at all.
    for role in [ColorRole::OnSurface, ColorRole::Primary, ColorRole::Success] {
        let color = palette.color(role);
        assert!(
            scene.commands().iter().any(|command| matches!(
                command,
                DrawCommand::DrawMask { color: tint, .. } if *tint == color
            )),
            "no mark drawn in {role:?}",
        );
    }
    // The same mark at three sizes is three rasterizations, one per size.
    let marks = ["mark-small", "mark-medium", "mark-large"];
    let widths: Vec<_> = marks
        .iter()
        .map(|key| {
            ui.inspect()
                .components
                .into_iter()
                .find(|component| component.key == *key)
                .and_then(|component| component.details.properties.get("pixel_width").cloned())
        })
        .collect();
    assert_eq!(
        widths,
        [16.0, 24.0, 48.0]
            .map(|width| Some(InspectValue::Number(width)))
            .to_vec(),
    );
    assert!(ui.next_deadline().is_none(), "drawings schedule nothing");
}

#[test]
fn the_images_page_decodes_on_a_worker_and_delivers_through_a_mailbox() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 1600.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "images");
    assert_eq!(text(&ui, "worker-status"), ["Not decoded yet."]);
    let button = find(&ui, &root, "decode-on-worker");
    click(&mut ui, &button);
    // The worker delivers when it finishes; the mailbox marks the runtime due.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while text(&ui, "worker-status") != ["Decoded on a worker thread."] {
        assert!(
            std::time::Instant::now() < deadline,
            "the worker never delivered"
        );
        std::thread::sleep(Duration::from_millis(5));
        settle(&mut ui);
    }
    let scene = ui.paint().unwrap();
    // The worker's image has the same content as the badge, so the scene still
    // holds the same four images (three on the page, the menu bar's mark)
    // while drawing one more.
    assert_eq!(scene.images().len(), 4);
    let draws = scene
        .commands()
        .iter()
        .filter(|command| matches!(command, DrawCommand::DrawImage { .. }))
        .count();
    assert_eq!(draws, 8);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_vector_shapes_page_preserves_geometry_and_supports_mask_fallback() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 1200.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "shapes");
    let bounds = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "vector-shapes" && component.visible)
        .and_then(|component| component.details.box_model.border_box)
        .unwrap();
    assert_eq!([bounds[2], bounds[3]], [640.0, 180.0]);
    let scene = ui.paint().unwrap();
    assert!(scene.vectors().len() >= 15);
    let scene = scene.rasterized().unwrap();
    // Two stars, six cap strokes, three joins, two ellipses, a rounded
    // rectangle and a trace: every one a mask inside the component.
    let inside = scene
        .commands()
        .iter()
        .filter(|command| match command {
            DrawCommand::DrawMask { rect, .. } => {
                rect.x() >= bounds[0] - 8.0
                    && rect.y() >= bounds[1] - 8.0
                    && rect.x() + rect.width() <= bounds[0] + bounds[2] + 8.0
                    && rect.y() + rect.height() <= bounds[1] + bounds[3] + 8.0
            }
            _ => false,
        })
        .count();
    assert!(inside >= 15, "{inside} masks");
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none(), "shapes schedule nothing");
}

#[test]
fn the_level_and_trace_page_reads_its_sources_only_while_running() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "meters");
    assert!(
        ui.next_deadline().is_none(),
        "stopped widgets drain nothing"
    );
    let run = find(&ui, &root, "widgets-run");
    ui.focus(&run).unwrap();
    press(&mut ui, KeyCode::Space);
    assert!(ui.next_deadline().is_some(), "running widgets are polled");
    // Leaving the page hides the readers, which suspends the sources.
    open(&mut ui, &root, "layout");
    assert!(ui.next_deadline().is_none());
    open(&mut ui, &root, "meters");
    assert!(ui.next_deadline().is_some());
    ui.focus(&run).unwrap();
    press(&mut ui, KeyCode::Space);
    let now = ui.now();
    ui.flush(now + Duration::from_millis(50)).unwrap();
    settle(&mut ui);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_drag_and_drop_page_moves_rows_between_lists_and_chips_into_a_bin() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "dragdrop");
    let (list_a, list_b) = (find(&ui, &root, "list-a"), find(&ui, &root, "list-b"));
    let row = find(&ui, &list_a, "Drums 1");
    let from = center_of(&ui, &row);
    let to = center_of(&ui, &list_b);
    pointer_at(
        &mut ui,
        from.0,
        from.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer_at(&mut ui, from.0 + 6.0, from.1, PointerKind::Move);
    assert!(ui.drag_active());
    // The preview is the row itself, built again by key and mounted above the page.
    let preview = ui.inspect().drag.unwrap().preview.expect("a preview");
    assert!(ui.inspect().components.iter().any(|c| c.id == preview));
    pointer_at(&mut ui, to.0, to.1, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::Grabbing);
    pointer_at(
        &mut ui,
        to.0,
        to.1,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!ui.drag_active());
    assert!(
        text(&ui, "drop-status")[0].starts_with("Move of Drums 1 to Bus B"),
        "{:?}",
        text(&ui, "drop-status")
    );
    let keys = |ui: &Runtime, list: &WidgetId| -> Vec<String> {
        ui.children(list)
            .unwrap()
            .iter()
            .map(|row| ui.key(row).unwrap().to_owned())
            .collect()
    };
    assert!(keys(&ui, &list_b).contains(&"Drums 1".to_owned()));
    assert!(!keys(&ui, &list_a).contains(&"Drums 1".to_owned()));

    // A chip is a custom source with its own payload: the open bin takes it,
    // the locked bin claims the drag to refuse it, and the lists ignore it.
    // The section is below the fold, so the page is scrolled to it first.
    let below = center_of(&ui, &find(&ui, &root, "bin-locked")).1 - 500.0;
    // Over the page's empty right side, so no list takes the wheel.
    pointer_at(
        &mut ui,
        800.0,
        300.0,
        PointerKind::Scroll(Point::new(0.0, below.max(0.0)).unwrap()),
    );
    let to = center_of(&ui, &list_b);
    let chip = center_of(&ui, &find(&ui, &root, "chip-kick"));
    let locked = center_of(&ui, &find(&ui, &root, "bin-locked"));
    pointer_at(
        &mut ui,
        chip.0,
        chip.1,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer_at(&mut ui, chip.0 + 6.0, chip.1, PointerKind::Move);
    pointer_at(&mut ui, locked.0, locked.1, PointerKind::Move);
    assert_eq!(ui.cursor_icon(), CursorIcon::NoDrop);
    pointer_at(&mut ui, to.0, to.1, PointerKind::Move);
    assert_eq!(
        ui.inspect().drag.unwrap().target,
        None,
        "a list does not know chips"
    );
    pointer_at(
        &mut ui,
        to.0,
        to.1,
        PointerKind::Release(PointerButton::Primary),
    );
    let open_bin = center_of(&ui, &find(&ui, &root, "bin-open"));
    let chip = center_of(&ui, &find(&ui, &root, "chip-snare"));
    drag_between(&mut ui, chip, open_bin);
    assert!(text(&ui, "received").contains(&"Move: Snare".to_owned()));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_drag_and_drop_page_moves_a_track_into_an_empty_folder() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 2000.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "dragdrop");
    let outline = find(&ui, &root, "outline");
    // A tree row's content carries the row's key too, so a row is its direct child.
    let row = |ui: &Runtime, key: &str| ui.find(&outline, &[key]).unwrap().unwrap();
    // Both rows lie clear of the list's edges, where a resting drag scrolls.
    let piano = center_of(&ui, &row(&ui, "Piano"));
    let archive = center_of(&ui, &row(&ui, "Archive/"));
    drag_between(&mut ui, piano, archive);
    assert!(!ui.drag_active());
    assert_eq!(
        text(&ui, "outline-status"),
        [r#"Moved Piano Into("Archive/")"#]
    );
    // The folder was expanded to show the result, with the track inside it.
    let rows: Vec<String> = ui
        .children(&outline)
        .unwrap()
        .iter()
        .map(|row| ui.key(row).unwrap().to_owned())
        .collect();
    let at = |key: &str| rows.iter().position(|row| row == key);
    assert_eq!(at("Piano"), at("Archive/").map(|archive| archive + 1));
    // A row fills its slot; its content carries the indent.
    let content_x = |key: &str| {
        let row = row(&ui, key);
        let content = ui.children(&row).unwrap()[0].clone();
        ui.bounds(&content).unwrap().unwrap().x()
    };
    assert!(
        content_x("Piano") > content_x("Archive/"),
        "indented under its folder"
    );

    // A row of the lists above carries the same payload type but belongs to
    // their model: the outline refuses it, so neither side changes.
    let list_a = find(&ui, &root, "list-a");
    let drums = center_of(&ui, &find(&ui, &list_a, "Drums 1"));
    let archive = center_of(&ui, &row(&ui, "Archive/"));
    drag_between(&mut ui, drums, archive);
    assert!(!ui.drag_active());
    assert_eq!(
        text(&ui, "outline-status"),
        [r#"Moved Piano Into("Archive/")"#]
    );
    assert!(
        ui.children(&list_a)
            .unwrap()
            .iter()
            .any(|row| ui.key(row).unwrap() == "Drums 1")
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_collections_table_moves_a_middle_column_but_not_the_first() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 2000.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "collections");
    let key = |ui: &mut Runtime, code: KeyCode, alt: bool| {
        if alt {
            ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
        }
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
        if alt {
            ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
        }
        ui.flush(ui.now()).unwrap();
    };
    let table = find(&ui, &root, "collections-table");
    let titles = |ui: &Runtime| -> Vec<String> {
        let key = ui.key(&table).unwrap().to_owned();
        ui.inspect()
            .components
            .iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.semantics.clone())
            .unwrap()
            .items
            .iter()
            .filter_map(|item| item.name.clone())
            .collect()
    };
    let body = ui.children(&table).unwrap()[0].clone();
    ui.focus(&body).unwrap();
    key(&mut ui, KeyCode::ArrowDown, false);
    // Sample stays first: Alt with Right on it does nothing.
    key(&mut ui, KeyCode::ArrowRight, false);
    key(&mut ui, KeyCode::ArrowRight, true);
    assert_eq!(titles(&ui)[..3], ["Sample", "Square root", "Square"]);
    // Square root moves one place to the right.
    key(&mut ui, KeyCode::ArrowRight, false);
    key(&mut ui, KeyCode::ArrowRight, true);
    assert_eq!(titles(&ui)[..3], ["Sample", "Square", "Square root"]);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_file_browser_page_lists_through_a_mailbox_and_shows_failures() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "files");
    // The listing the browser asked for arrives through the mailbox.
    settle(&mut ui);
    let library = find(&ui, &root, "library");
    let entries = ui.find_descendant(&library, "entries").unwrap().unwrap();
    assert_eq!(ui.children(&entries).unwrap().len(), 8);
    ui.focus(&entries).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    let opened = text(&ui, "opened");
    assert!(opened[0].starts_with("Opened ") && opened[0].ends_with("README.txt"));
    // Directories come first: Drums, Latest project, Locked.
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    let rows: Vec<String> = ui
        .children(&entries)
        .unwrap()
        .iter()
        .map(|row| ui.key(row).unwrap().to_owned())
        .collect();
    assert_eq!(rows, ["error"], "a locked folder is an error, not empty");
    press(&mut ui, KeyCode::Backspace);
    settle(&mut ui);
    assert_eq!(ui.children(&entries).unwrap().len(), 8);
    assert_eq!(
        text(&ui, "deliveries"),
        ["Listings applied: 3; stale results dropped: 0"]
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(
        ui.next_deadline().is_none(),
        "the browser schedules nothing"
    );
}

#[test]
fn the_file_browser_page_keeps_a_response_its_full_mailbox_refuses() {
    use crate::pages::data::files::{ROOT, listing, send_or_keep};
    use std::{cell::RefCell, rc::Rc};
    let mut ui = Runtime::new();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let queue = requests.clone();
    let controller = FileBrowserController::new(
        &ui,
        [ROOT],
        ListSelection::new(&ui, SelectionMode::Single),
        move |request| queue.borrow_mut().push(request),
    )
    .unwrap();
    controller.refresh().unwrap();
    let taken: Vec<DirectoryRequest> = requests.borrow_mut().drain(..).collect();
    let mut responses = taken.into_iter().map(|request| {
        let listing = listing(request.directory());
        request.respond(listing)
    });
    let delivered = Rc::new(RefCell::new(0));
    let count = delivered.clone();
    let mailbox = ui
        .mailbox(Capacity::new(1).unwrap(), move |_: DirectoryResponse| {
            *count.borrow_mut() += 1;
            Ok(())
        })
        .unwrap();
    let backlog = RefCell::new(Vec::new());
    send_or_keep(&mailbox.sender(), &backlog, responses.next().unwrap());
    assert!(backlog.borrow().is_empty());
    // The mailbox holds one value: the next response waits in the backlog
    // instead of being dropped.
    send_or_keep(&mailbox.sender(), &backlog, responses.next().unwrap());
    assert_eq!(backlog.borrow().len(), 1);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(*delivered.borrow(), 1);
}

/// Types `query` into the field of the collection filter keyed `filter`, then
/// lets its pause elapse.
fn filter_collection(ui: &mut Runtime, root: &WidgetId, filter: &str, query: &str) {
    let filter = find(ui, root, filter);
    let field = ui.find_descendant(&filter, "filter").unwrap().unwrap();
    ui.focus(&field).unwrap();
    settle(ui);
    ui.text_input(query).unwrap();
    let now = ui.inspect().at;
    ui.flush(now + Duration::from_millis(250)).unwrap();
}

#[test]
fn the_collections_filters_hide_items_and_open_matching_folders() {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(960.0, 2000.0, 1.0).unwrap())
        .unwrap();
    open(&mut ui, &root, "collections");
    let grid = find(&ui, &root, "collections-grid");
    filter_collection(&mut ui, &root, "grid-filter", "item 999");
    let shown: Vec<String> = ui
        .children(&grid)
        .unwrap()
        .iter()
        .map(|row| ui.key(row).unwrap().to_owned())
        .collect();
    // Items 999 and 9990 to 9999 match, by the text each shows.
    assert_eq!(shown.len(), 11, "{shown:?}");
    assert!(shown.contains(&"swatch-998".to_owned()));
    // Down in the field moves focus into the grid.
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(ui.focused(), Some(grid.clone()));

    // A match two levels down shows with the folders above it, which open
    // only in the filter's layer.
    let tree = find(&ui, &root, "collections-tree");
    filter_collection(&mut ui, &root, "tree-filter", "file-7.txt");
    let rows = ui.children(&tree).unwrap().len();
    assert!(rows > 10, "{rows} rows");
    press(&mut ui, KeyCode::Escape);
    let now = ui.inspect().at;
    ui.flush(now + Duration::from_millis(250)).unwrap();
    assert_eq!(ui.children(&tree).unwrap().len(), 10, "no folder was open");
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_file_browser_filters_only_listed_folders() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "files");
    settle(&mut ui);
    let library = find(&ui, &root, "library");
    let folders = ui.find_descendant(&library, "folders").unwrap().unwrap();
    let tree = ui.find_descendant(&folders, "tree").unwrap().unwrap();
    let deliveries = text(&ui, "deliveries");
    // Kicks lies in Drums, which is not listed yet: nothing matches and the
    // filter asks for no listing.
    filter_collection(&mut ui, &root, "folders", "kick");
    settle(&mut ui);
    assert!(ui.children(&tree).unwrap().is_empty());
    assert_eq!(text(&ui, "deliveries"), deliveries);
    let details = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "folders")
        .unwrap()
        .details;
    assert_eq!(details.properties["shown"], InspectValue::Number(0.0));
}
