//! The table as a consumer of the virtualized list and the shared selection:
//! columns, a header that does not scroll, sorting by callback and resizing.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 24.0;

/// `order[i]` is the number shown on row `i`; the application owns the order.
struct Numbers(Vec<u32>);
impl TableItems for Numbers {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|number| format!("n{number}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let number = *self.0.get(index)?;
        let text = match column {
            "number" => number.to_string(),
            "square" => (u64::from(number) * u64::from(number)).to_string(),
            _ => return None,
        };
        Some(Element::new("text", Label::new(text)))
    }
}

struct Fixture {
    ui: Runtime,
    state: TableController,
    table: WidgetId,
    sorts: Rc<RefCell<Vec<TableSort>>>,
}
fn fixture(count: u32) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("number", "Number")
                .width(Length::new(100.0)?)
                .resizable(true)
                .sortable(true),
            TableColumn::new("square", "Square"),
        ],
        TableSource::new(Numbers((0..count).collect())),
    )?;
    let sorts = Rc::new(RefCell::new(Vec::new()));
    let (log, sorted) = (sorts.clone(), state.clone());
    let table = ui.mount_tree(Element::new(
        "table",
        Table::new(&state, ROW)?.on_sort(move |_, sort| {
            log.borrow_mut().push(sort.clone());
            // The application sorts its own data and publishes it.
            let mut numbers: Vec<u32> = (0..count).collect();
            if sort.direction == SortDirection::Descending {
                numbers.reverse();
            }
            sorted.set_source(TableSource::new(Numbers(numbers)))
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(Fixture {
        ui,
        state,
        table,
        sorts,
    })
}

#[test]
fn actionable_header_and_resize_edge_use_distinct_hover_colors() -> TestResult {
    let mut f = fixture(3)?;
    let colors = f.ui.theme().palette(ThemeMode::Dark)?;
    let has = |ui: &mut Runtime, role: ColorRole| {
        let color = colors.color(role);
        ui.paint().unwrap().commands().iter().any(|command| {
            matches!(command,
                DrawCommand::FillRect { color: fill, .. }
                | DrawCommand::DrawMask { color: fill, .. }
                if *fill == color)
        })
    };
    assert!(!has(&mut f.ui, ColorRole::ResizeHandleHover));
    f.ui.pointer_input(PointerEvent::new(
        Point::new(50.0, 10.0)?,
        PointerKind::Move,
    ))?;
    f.ui.flush(Duration::ZERO)?;
    assert!(has(&mut f.ui, ColorRole::SurfaceHover));
    f.ui.pointer_input(PointerEvent::new(
        Point::new(100.0, 10.0)?,
        PointerKind::Move,
    ))?;
    f.ui.flush(Duration::ZERO)?;
    assert!(has(&mut f.ui, ColorRole::ResizeHandleHover));
    Ok(())
}
fn body(f: &Fixture) -> WidgetId {
    f.ui.children(&f.table).unwrap()[0].clone()
}
fn row_keys(f: &Fixture) -> Vec<String> {
    f.ui.children(&body(f))
        .unwrap()
        .iter()
        .map(|row| f.ui.key(row).unwrap().to_owned())
        .collect()
}
/// The cells of the first mounted row, as `(x, width)` in window coordinates.
fn cells(f: &Fixture) -> Vec<(f32, f32)> {
    let row = f.ui.children(&body(f)).unwrap()[0].clone();
    f.ui.children(&row)
        .unwrap()
        .iter()
        .map(|cell| {
            let bounds = f.ui.bounds(cell).unwrap().unwrap();
            (bounds.x(), bounds.width())
        })
        .collect()
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
}

#[test]
fn workspace_widths_restore_but_refuse_an_owned_resize() -> TestResult {
    let mut f = fixture(20)?;
    let saved = f.ui.capture_workspace()?;
    pointer(
        &mut f.ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut f.ui, 150.0, 10.0, PointerKind::Move);
    let during = f.ui.capture_workspace()?;
    let report = f.ui.restore_workspace(&saved)?;
    assert!(
        report
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    assert_eq!(f.ui.capture_workspace()?, during);
    pointer(
        &mut f.ui,
        150.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f)[0].1, 150.0);
    assert!(f.ui.restore_workspace(&saved)?.is_complete());
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f)[0].1, 100.0);
    Ok(())
}

/// The widths the table's captured entry records, by column key.
fn captured_widths(workspace: &Workspace) -> Vec<(String, f64)> {
    let (_, state) = workspace
        .entries()
        .find(|(_, state)| state.kind() == "table")
        .unwrap();
    state
        .map("widths")
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), value.as_number().unwrap()))
        .collect()
}

#[test]
fn a_sized_column_whose_key_cannot_be_stored_fails_capture_by_path() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let long = "c".repeat(Workspace::MAX_STRING_BYTES + 1);
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new(&long, "Long")
                .width(Length::new(100.0)?)
                .resizable(true),
            TableColumn::new("square", "Square"),
        ],
        TableSource::new(Numbers((0..5).collect())),
    )?;
    ui.mount_tree(Element::new("table", Table::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    // Before the user sizes it, the column has no entry to write.
    assert_eq!(ui.capture_workspace()?.len(), 1);
    pointer(
        &mut ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 130.0, 10.0, PointerKind::Move);
    pointer(
        &mut ui,
        130.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO)?;
    // The table used to fall out of a capture that reported success.
    match ui.capture_workspace() {
        Err(WorkspaceError::Participant { path, source }) => {
            assert_eq!(path, "table");
            assert_eq!(*source, WorkspaceError::TextTooLong);
        }
        other => panic!("expected a participant failure, got {other:?}"),
    }
    Ok(())
}

#[test]
fn only_widths_the_user_sized_are_captured() -> TestResult {
    let mut f = fixture(20)?;
    // "number" has the width the application described, which stays the
    // application's: a later version may describe another one.
    let described = f.ui.capture_workspace()?;
    assert!(captured_widths(&described).is_empty());

    pointer(
        &mut f.ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut f.ui, 130.0, 10.0, PointerKind::Move);
    pointer(
        &mut f.ui,
        130.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    let dragged = f.ui.capture_workspace()?;
    assert_eq!(captured_widths(&dragged), [("number".to_owned(), 130.0)]);

    // Restoring the described capture makes the column the application's
    // again, so capturing reproduces it.
    assert!(f.ui.restore_workspace(&described)?.is_complete());
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f)[0].1, 100.0);
    assert_eq!(f.ui.capture_workspace()?, described);

    // A width an entry gives, such as one an older capture recorded for a
    // column the application had fixed, counts as the user's.
    let (path, _) = described.entries().next().unwrap();
    let older = Workspace::new(
        *described.settings(),
        [(
            path.clone(),
            ViewState::new("table")?.with(
                "widths",
                WorkspaceValue::map([("number".to_owned(), WorkspaceValue::number(100.0)?)])?,
            )?,
        )],
    )?;
    assert!(f.ui.restore_workspace(&older)?.is_complete());
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f)[0].1, 100.0);
    assert_eq!(f.ui.capture_workspace()?, older);
    Ok(())
}

#[test]
fn restoring_returns_a_dragged_flexible_column_to_its_share() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("square", "Square").resizable(true),
            TableColumn::new("number", "Number").width(Length::new(100.0)?),
        ],
        TableSource::new(Numbers((0..20).collect())),
    )?;
    ui.mount_tree(Element::new("table", Table::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    let described = &state.columns()[0];
    assert_eq!(
        (described.flex_weight(), described.fixed_width()),
        (Some(1.0), None)
    );
    assert!(described.is_resizable() && !described.is_sortable() && !described.is_movable());
    let saved = ui.capture_workspace()?;

    // The flexible column takes the 300 pixels the fixed one leaves; dragging
    // its edge fixes it at the width it was given.
    pointer(
        &mut ui,
        300.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 250.0, 10.0, PointerKind::Move);
    pointer(
        &mut ui,
        250.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO)?;
    let dragged = &state.columns()[0];
    assert_eq!(
        (dragged.flex_weight(), dragged.fixed_width()),
        (None, Some(250.0))
    );
    assert_ne!(ui.capture_workspace()?, saved);

    // The saved entry lists no width for it, so it returns to its share, and
    // capturing again reproduces the entry.
    assert!(ui.restore_workspace(&saved)?.is_complete());
    ui.flush(Duration::ZERO)?;
    assert_eq!(state.columns()[0].flex_weight(), Some(1.0));
    assert_eq!(ui.capture_workspace()?, saved);
    Ok(())
}

#[test]
fn a_missing_cell_is_drawn_empty_rather_than_failing_the_row() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    // `Numbers` has no content for the "notes" column.
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("number", "Number"),
            TableColumn::new("notes", "Notes"),
        ],
        TableSource::new(Numbers((0..3).collect())),
    )?;
    let table = ui.mount_tree(Element::new("table", Table::new(&state, ROW)?))?;
    ui.flush(Duration::ZERO)?;
    let body = ui.children(&table)?[0].clone();
    let rows = ui.children(&body)?;
    assert_eq!(rows.len(), 3);
    let cells = ui.children(&rows[0])?;
    assert_eq!(cells.len(), 2, "the empty cell keeps its column");
    assert_eq!(ui.children(&cells[0])?.len(), 1);
    assert!(ui.children(&cells[1])?.is_empty());
    assert_eq!(ui.bounds(&cells[1])?.map(|cell| cell.x()), Some(200.0));
    Ok(())
}

#[test]
fn rows_are_virtualized_under_a_header_that_does_not_scroll() -> TestResult {
    let mut f = fixture(100_000)?;
    let header = ThemeMetrics::default().metric(MetricRole::ControlHeight);
    let bounds = f.ui.bounds(&body(&f))?.unwrap();
    assert_eq!((bounds.y(), bounds.height()), (header, 200.0 - header));
    assert!(
        row_keys(&f).len() < 40,
        "{} rows mounted",
        row_keys(&f).len()
    );
    // A fixed column keeps its width and the flexible one takes the rest.
    assert_eq!(cells(&f), [(0.0, 100.0), (100.0, 300.0)]);

    pointer(
        &mut f.ui,
        50.0,
        100.0,
        PointerKind::Scroll(Point::new(0.0, ROW * 1_000.0)?),
    );
    f.ui.flush(Duration::ZERO)?;
    assert!(row_keys(&f).contains(&"n1000".to_owned()));
    assert_eq!(
        f.ui.bounds(&body(&f))?.unwrap().y(),
        header,
        "the header stays"
    );
    assert_eq!(f.ui.next_deadline(), None);
    Ok(())
}

#[test]
fn the_controller_reveals_a_row_by_key_below_the_header() -> TestResult {
    let mut f = fixture(100_000)?;
    let header = ThemeMetrics::default().metric(MetricRole::ControlHeight);
    let receipt = f.state.reveal("n77777")?;
    f.ui.flush(Duration::ZERO)?;
    let row =
        f.ui.children(&body(&f))?
            .into_iter()
            .find(|id| f.ui.key(id).unwrap() == "n77777")
            .expect("the revealed row is mounted");
    assert_eq!(receipt.status(), ListRevealStatus::Mounted(row.clone()));
    let [_, y, _, h] = f.ui.bounds(&row)?.unwrap().values();
    assert!(y >= header && y + h <= 200.0, "revealed at {y}");
    assert_eq!(f.ui.bounds(&body(&f))?.unwrap().y(), header);
    // An unknown key changes nothing.
    let mounted = row_keys(&f);
    let unknown = f.state.reveal("n-nope")?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(row_keys(&f), mounted);
    assert_eq!(unknown.status(), ListRevealStatus::NotFound);

    // A table created later over the same controller does not replay the
    // served request, and serves the ones made after it.
    f.ui.unmount(&f.table)?;
    f.table =
        f.ui.mount_tree(Element::new("again", Table::new(&f.state, ROW)?))?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(row_keys(&f)[0], "n0");
    let _ = f.state.reveal("n500")?;
    f.ui.flush(Duration::ZERO)?;
    assert!(row_keys(&f).contains(&"n500".to_owned()));
    Ok(())
}

#[test]
fn a_sortable_header_reports_and_the_application_sorts() -> TestResult {
    let mut f = fixture(50)?;
    let click = |f: &mut Fixture, x: f32| {
        pointer(
            &mut f.ui,
            x,
            10.0,
            PointerKind::Press(PointerButton::Primary),
        );
        pointer(
            &mut f.ui,
            x,
            10.0,
            PointerKind::Release(PointerButton::Primary),
        );
        f.ui.flush(Duration::ZERO).unwrap();
    };
    assert_eq!(f.state.sort().get()?, None);
    click(&mut f, 40.0);
    let ascending = TableSort {
        column: "number".into(),
        direction: SortDirection::Ascending,
    };
    assert_eq!(f.state.sort().get()?, Some(ascending.clone()));
    assert_eq!(row_keys(&f)[0], "n0");

    // The same column again flips the direction; the rows follow the application.
    click(&mut f, 40.0);
    assert_eq!(f.sorts.borrow().len(), 2);
    assert_eq!(f.sorts.borrow()[1].direction, SortDirection::Descending);
    assert_eq!(row_keys(&f)[0], "n49");

    // A column that is not sortable reports nothing.
    click(&mut f, 300.0);
    assert_eq!(f.sorts.borrow().len(), 2);
    // A press that leaves its column before release is abandoned.
    pointer(
        &mut f.ui,
        40.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut f.ui,
        300.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(f.sorts.borrow().len(), 2);
    Ok(())
}

#[test]
fn a_narrow_sorted_header_clips_its_title_before_the_indicator() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(70.0, 100.0, 1.0)?)?;
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("number", "A very long number title")
            .width(Length::new(70.0)?)
            .sortable(true)],
        TableSource::new(Numbers(vec![1])),
    )?;
    ui.mount_tree(Element::new(
        "table",
        Table::new(&state, ROW)?.on_sort(|_, _| Ok(())),
    ))?;
    ui.flush(Duration::ZERO)?;
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        pointer(&mut ui, 20.0, 10.0, kind);
    }
    ui.flush(Duration::ZERO)?;
    assert!(state.sort().get()?.is_some());

    let theme = ui.theme();
    let palette = theme.palette(theme.mode()?)?;
    let title_color = palette.color(ColorRole::OnSurface);
    let metrics = theme.metrics(theme.density()?)?;
    let header = metrics.metric(MetricRole::ControlHeight);
    let title_end = 70.0
        - metrics.metric(MetricRole::IconSize)
        - metrics.metric(MetricRole::ControlPaddingX) / 2.0
        - metrics.metric(MetricRole::IconGap);
    let scene = ui.paint()?;
    let title_clips: Vec<_> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, color, clip, ..
            } if *color == title_color && rect.y() < header => *clip,
            _ => None,
        })
        .collect();
    assert!(!title_clips.is_empty(), "the title is painted");
    assert!(
        title_clips
            .iter()
            .all(|clip| clip.x() + clip.width() <= title_end)
    );
    Ok(())
}

#[test]
fn dragging_a_column_edge_moves_the_cells_of_mounted_rows() -> TestResult {
    let mut f = fixture(50)?;
    let rows = f.ui.children(&body(&f))?;
    // The edge of the first column is at x = 100.
    pointer(
        &mut f.ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut f.ui, 160.0, 10.0, PointerKind::Move);
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)]);
    // Narrower than the minimum is held at the minimum.
    pointer(&mut f.ui, 5.0, 10.0, PointerKind::Move);
    pointer(
        &mut f.ui,
        5.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    let minimum = TableColumn::MIN_WIDTH;
    assert_eq!(cells(&f), [(0.0, minimum), (minimum, 400.0 - minimum)]);
    // Resizing rearranged the rows that were mounted; it did not rebuild them.
    assert_eq!(f.ui.children(&body(&f))?, rows);
    assert_eq!(f.state.sort().get()?, None, "a resize is not a sort");
    assert_eq!(f.ui.next_deadline(), None);
    Ok(())
}

fn escape(ui: &mut Runtime) -> TestResult {
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    ui.flush(ui.now())?;
    Ok(())
}

#[test]
fn escape_ends_a_column_resize_where_it_is() -> TestResult {
    let mut f = fixture(50)?;
    f.ui.focus(&body(&f))?;
    pointer(
        &mut f.ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut f.ui, 160.0, 10.0, PointerKind::Move);
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)]);
    escape(&mut f.ui)?;
    assert_eq!(f.ui.focused(), Some(body(&f)), "Escape kept the focus");
    // The pointer no longer resizes, and the release that follows neither
    // resizes nor sorts.
    pointer(&mut f.ui, 60.0, 10.0, PointerKind::Move);
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)]);
    pointer(
        &mut f.ui,
        60.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)]);
    assert!(f.sorts.borrow().is_empty());
    assert_eq!(f.state.sort().get()?, None);
    Ok(())
}

#[test]
fn a_header_press_focuses_the_body_so_escape_reaches_it_without_prior_focus() -> TestResult {
    let mut f = fixture(50)?;
    assert_eq!(f.ui.focused(), None);
    // A column resize: the press moves the focus to the body at the next
    // flush, which a native host runs before it delivers a key.
    pointer(
        &mut f.ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut f.ui, 160.0, 10.0, PointerKind::Move);
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(f.ui.focused(), Some(body(&f)));
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)], "still resizing");
    escape(&mut f.ui)?;
    pointer(&mut f.ui, 60.0, 10.0, PointerKind::Move);
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f), [(0.0, 160.0), (160.0, 240.0)]);
    pointer(
        &mut f.ui,
        60.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert!(f.sorts.borrow().is_empty());

    // A title press, with the focus taken away again.
    f.ui.clear_focus();
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(f.ui.focused(), None);
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(f.ui.focused(), Some(body(&f)));
    escape(&mut f.ui)?;
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert!(
        f.sorts.borrow().is_empty(),
        "the cancelled press did not sort"
    );
    assert_eq!(f.state.sort().get()?, None);
    Ok(())
}

#[test]
fn escape_cancels_a_title_press_so_its_release_does_not_sort() -> TestResult {
    let mut f = fixture(50)?;
    f.ui.focus(&body(&f))?;
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    escape(&mut f.ui)?;
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert!(f.sorts.borrow().is_empty());
    assert_eq!(f.state.sort().get()?, None);
    // The next press and release sorts as usual.
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut f.ui,
        50.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(f.sorts.borrow().len(), 1);
    Ok(())
}

#[test]
fn rows_use_the_shared_selection_and_the_table_describes_its_columns() -> TestResult {
    let mut f = fixture(1_000)?;
    let selection = f.state.selection();
    f.ui.focus(&body(&f))?;
    for key in [KeyCode::ArrowDown, KeyCode::End] {
        f.ui.key_input(key, KeyAction::Press)?;
        f.ui.key_input(key, KeyAction::Release)?;
        f.ui.flush(Duration::ZERO)?;
    }
    assert_eq!(selection.active().get()?.as_deref(), Some("n999"));
    assert!(row_keys(&f).contains(&"n999".to_owned()));

    let semantics =
        f.ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == "table")
            .and_then(|component| component.details.semantics)
            .unwrap();
    assert_eq!(semantics.role, SemanticRole::Table);
    let names: Vec<_> = semantics
        .items
        .iter()
        .map(|item| item.name.as_deref().unwrap())
        .collect();
    assert_eq!(names, ["Number", "Square"]);
    assert_eq!(
        semantics.items[0].actions,
        [SemanticAction::Select, SemanticAction::Resize]
    );
    assert!(semantics.items[1].actions.is_empty());
    Ok(())
}

#[test]
fn columns_are_validated() {
    let ui = Runtime::new();
    let state = |columns: Vec<TableColumn>| {
        TableController::new(
            &ui,
            ListSelection::new(&ui, SelectionMode::Single),
            columns,
            TableSource::new(Numbers(Vec::new())),
        )
        .err()
    };
    assert_eq!(state(Vec::new()), Some(LayoutError::NoColumns.into()));
    assert_eq!(
        state(vec![TableColumn::new("", "x")]),
        Some(RuntimeError::EmptyKey)
    );
    assert_eq!(
        state(vec![TableColumn::new("a", "x"), TableColumn::new("a", "y")]),
        Some(RuntimeError::DuplicateKey("a".into()))
    );
    for weight in [0.0, -1.0, f32::NAN] {
        assert_eq!(
            TableColumn::new("a", "x").flex(weight).err(),
            Some(LayoutError::InvalidWeight)
        );
    }
}

/// Five fixed 150-pixel columns in a 400-pixel table: 350 pixels overflow.
/// Each activation as `(row, column)`.
type Activations = Rc<RefCell<Vec<(String, Option<String>)>>>;
type Wide = (Runtime, TableController, WidgetId, Activations);

fn wide() -> Result<Wide, Box<dyn std::error::Error>> {
    struct Wide;
    impl TableItems for Wide {
        fn len(&self) -> usize {
            200
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < 200).then(|| format!("r{index}"))
        }
        fn cell(&self, index: usize, column: &str) -> Option<Element> {
            Some(Element::new("text", Label::new(format!("{column}{index}"))))
        }
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let columns = ["a", "b", "c", "d", "e"].map(|key| {
        TableColumn::new(key, key.to_uppercase())
            .width(Length::new(150.0).unwrap())
            .resizable(true)
    });
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        columns,
        TableSource::new(Wide),
    )?;
    let activated = Rc::new(RefCell::new(Vec::new()));
    let log = activated.clone();
    let table = ui.mount_tree(Element::new(
        "table",
        Table::new(&state, ROW)?.on_activate(move |_, row, column| {
            log.borrow_mut()
                .push((row.to_owned(), column.map(str::to_owned)));
            Ok(())
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok((ui, state, table, activated))
}
fn scroll_x(ui: &Runtime) -> f64 {
    let snapshot = ui.inspect();
    let table = snapshot
        .components
        .iter()
        .find(|component| component.key == "table")
        .unwrap();
    match table.details.properties.get("scroll_x") {
        Some(InspectValue::Number(value)) => *value,
        other => panic!("no scroll_x: {other:?}"),
    }
}
fn first_row_cells(ui: &Runtime, table: &WidgetId) -> Vec<f32> {
    let body = ui.children(table).unwrap()[0].clone();
    let row = ui.children(&body).unwrap()[0].clone();
    ui.children(&row)
        .unwrap()
        .iter()
        .map(|cell| ui.bounds(cell).unwrap().unwrap().x())
        .collect()
}
fn tap(ui: &mut Runtime, key: KeyCode) {
    ui.key_input(key, KeyAction::Press).unwrap();
    ui.key_input(key, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
}

#[test]
fn a_wide_table_scrolls_header_and_rows_by_one_offset() -> TestResult {
    let (mut ui, _state, table, _) = wide()?;
    assert_eq!(
        first_row_cells(&ui, &table),
        [0.0, 150.0, 300.0, 450.0, 600.0]
    );
    let rows = ui.children(&ui.children(&table)?[0])?;

    // A horizontal wheel moves every cell and clamps at the overflow.
    pointer(
        &mut ui,
        50.0,
        100.0,
        PointerKind::Scroll(Point::new(120.0, 0.0)?),
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(scroll_x(&ui), 120.0);
    assert_eq!(first_row_cells(&ui, &table)[0], -120.0);
    pointer(
        &mut ui,
        50.0,
        100.0,
        PointerKind::Scroll(Point::new(9_000.0, 0.0)?),
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(scroll_x(&ui), 350.0, "750 of columns in 400 of table");
    assert_eq!(
        first_row_cells(&ui, &table)[4],
        250.0,
        "the last column is reachable"
    );
    // Scrolling rearranged the mounted rows; it rebuilt nothing and scheduled nothing.
    assert_eq!(ui.children(&ui.children(&table)?[0])?, rows);
    assert_eq!(ui.next_deadline(), None);

    // The header follows: a press where column E's edge now is resizes E.
    pointer(
        &mut ui,
        400.0 - 1.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 350.0, 10.0, PointerKind::Move);
    pointer(
        &mut ui,
        350.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO)?;
    let widths = match ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "table")
        .and_then(|component| component.details.properties.get("column_widths").cloned())
    {
        Some(InspectValue::List(widths)) => widths,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        widths[4],
        InspectValue::Number(101.0),
        "narrowed by the pointer's movement, not collapsed by the clamped offset"
    );

    // Dragging the bar along the bottom edge to its start scrolls back.
    pointer(
        &mut ui,
        200.0,
        197.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, -500.0, 197.0, PointerKind::Move);
    pointer(
        &mut ui,
        -500.0,
        197.0,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(scroll_x(&ui), 0.0);
    Ok(())
}

#[test]
fn the_active_cell_moves_with_the_arrows_and_scrolls_its_column_into_view() -> TestResult {
    let (mut ui, state, table, activated) = wide()?;
    let body = ui.children(&table)?[0].clone();
    ui.focus(&body)?;
    tap(&mut ui, KeyCode::ArrowDown);
    tap(&mut ui, KeyCode::ArrowDown);
    assert_eq!(
        state.active_column().get()?,
        None,
        "rows alone name no column"
    );
    tap(&mut ui, KeyCode::Enter);
    assert_eq!(*activated.borrow(), [("r1".to_owned(), None)]);

    // Right enters the first column, then walks to the last and stops there.
    tap(&mut ui, KeyCode::ArrowRight);
    assert_eq!(state.active_column().get()?.as_deref(), Some("a"));
    for _ in 0..6 {
        tap(&mut ui, KeyCode::ArrowRight);
    }
    assert_eq!(state.active_column().get()?.as_deref(), Some("e"));
    assert_eq!(
        scroll_x(&ui),
        350.0,
        "the active column was scrolled into view"
    );
    tap(&mut ui, KeyCode::Enter);
    assert_eq!(
        activated.borrow().last(),
        Some(&("r1".to_owned(), Some("e".to_owned())))
    );
    // Up and Down are still the rows', and keep the column.
    tap(&mut ui, KeyCode::ArrowDown);
    assert_eq!(state.selection().active().get()?.as_deref(), Some("r2"));
    assert_eq!(state.active_column().get()?.as_deref(), Some("e"));
    tap(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(state.active_column().get()?.as_deref(), Some("d"));
    assert_eq!(scroll_x(&ui), 350.0, "already in view, so nothing scrolls");

    // Exactly one cell is outlined: row r2, column d, in the focus ring role.
    let theme = ui.theme();
    let primary = theme.palette(theme.mode()?)?.color(ColorRole::FocusRing);
    let outlined: Vec<Rect> = ui
        .paint()?
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color } | DrawCommand::DrawMask { rect, color, .. }
                if *color == primary =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert!(!outlined.is_empty());
    let header = ThemeMetrics::default().metric(MetricRole::ControlHeight);
    assert!(
        outlined.iter().all(|rect| rect.x() >= 100.0 - 0.01
            && rect.x() + rect.width() <= 250.0 + 0.01
            && rect.y() >= header + 2.0 * ROW - 0.01
            && rect.y() + rect.height() <= header + 3.0 * ROW + 0.01),
        "{outlined:?}"
    );
    Ok(())
}

#[test]
fn a_press_names_the_cell_and_cells_are_described_by_their_column() -> TestResult {
    let (mut ui, state, _table, _) = wide()?;
    let header = ThemeMetrics::default().metric(MetricRole::ControlHeight);
    // Row 3, column B (150..300).
    pointer(
        &mut ui,
        200.0,
        header + 3.5 * ROW,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(
        &mut ui,
        200.0,
        header + 3.5 * ROW,
        PointerKind::Release(PointerButton::Primary),
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(state.selection().active().get()?.as_deref(), Some("r3"));
    assert_eq!(state.active_column().get()?.as_deref(), Some("b"));

    let cells: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter_map(|component| component.details.semantics)
        .filter(|semantics| semantics.role == SemanticRole::Cell)
        .collect();
    assert!(!cells.is_empty());
    assert!(cells.iter().any(|cell| cell.name.as_deref() == Some("B")));

    assert_eq!(
        state.set_active_column(Some("nope".into())),
        Err(LayoutError::UnknownKey("nope".into()).into())
    );
    state.set_active_column(None)?;
    assert_eq!(state.active_column().get()?, None);
    Ok(())
}

/// Holds Alt around a press and release of `key`, then flushes.
fn alt_tap(ui: &mut Runtime, key: KeyCode) {
    ui.key_input(KeyCode::AltLeft, KeyAction::Press).unwrap();
    ui.key_input(key, KeyAction::Press).unwrap();
    ui.key_input(key, KeyAction::Release).unwrap();
    ui.key_input(KeyCode::AltLeft, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
}

#[test]
fn alt_with_up_or_down_sorts_the_active_cells_column_from_the_keyboard() -> TestResult {
    let mut f = fixture(50)?;
    let body = body(&f);
    f.ui.focus(&body)?;
    // No active column: the chord is left alone and nothing sorts.
    alt_tap(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(f.state.sort().get()?, None);
    assert!(f.sorts.borrow().is_empty());

    f.state.selection().set_active(Some("n0".into()))?;
    f.state.set_active_column(Some("number".into()))?;
    f.ui.flush(Duration::ZERO)?;
    alt_tap(&mut f.ui, KeyCode::ArrowDown);
    let descending = TableSort {
        column: "number".into(),
        direction: SortDirection::Descending,
    };
    assert_eq!(f.state.sort().get()?, Some(descending.clone()));
    assert_eq!(
        f.sorts.borrow().as_slice(),
        std::slice::from_ref(&descending)
    );
    assert_eq!(
        row_keys(&f)[0],
        "n49",
        "the application sorted on the request"
    );

    alt_tap(&mut f.ui, KeyCode::ArrowUp);
    assert_eq!(
        f.state.sort().get()?.map(|sort| sort.direction),
        Some(SortDirection::Ascending)
    );
    assert_eq!(f.sorts.borrow().len(), 2);
    assert_eq!(row_keys(&f)[0], "n0");

    // Without Alt the arrows move the active row, as before.
    tap(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(f.state.selection().active().get()?.as_deref(), Some("n1"));
    assert_eq!(f.sorts.borrow().len(), 2);

    // A column that is not sortable leaves the chord to command bindings.
    f.state.set_active_column(Some("square".into()))?;
    f.ui.flush(Duration::ZERO)?;
    alt_tap(&mut f.ui, KeyCode::ArrowDown);
    assert_eq!(f.sorts.borrow().len(), 2);
    assert_eq!(
        f.state.sort().get()?,
        Some(TableSort {
            column: "number".into(),
            direction: SortDirection::Ascending,
        })
    );
    Ok(())
}

#[test]
fn the_controller_fixes_a_column_width_like_a_drag_except_during_one() -> TestResult {
    let (mut ui, state, table, _) = wide()?;
    assert_eq!(first_row_cells(&ui, &table)[1], 150.0);
    state.set_column_width("a", Length::new(100.0)?)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(state.columns()[0].fixed_width(), Some(100.0));
    assert_eq!(
        first_row_cells(&ui, &table)[1],
        100.0,
        "the rows follow the new width after the flush"
    );
    let header_widths = {
        let snapshot = ui.inspect();
        let table = snapshot
            .components
            .iter()
            .find(|component| component.key == "table")
            .unwrap();
        match table.details.properties.get("column_widths") {
            Some(InspectValue::List(widths)) => widths.clone(),
            other => panic!("no column_widths: {other:?}"),
        }
    };
    assert_eq!(header_widths[0], InspectValue::Number(100.0));

    // The width counts as the user's, so a captured workspace keeps it.
    let workspace = ui.capture_workspace()?;
    assert_eq!(captured_widths(&workspace), vec![("a".to_owned(), 100.0)]);

    // Below the floor, the floor applies; an unknown column is refused.
    state.set_column_width("b", Length::new(1.0)?)?;
    assert_eq!(
        state.columns()[1].fixed_width(),
        Some(TableColumn::MIN_WIDTH)
    );
    assert_eq!(
        state.set_column_width("nope", Length::new(100.0)?),
        Err(LayoutError::UnknownKey("nope".into()).into())
    );

    // A held edge drag owns the widths until it ends.
    ui.flush(Duration::ZERO)?;
    pointer(
        &mut ui,
        100.0,
        10.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 120.0, 10.0, PointerKind::Move);
    assert_eq!(
        state.set_column_width("c", Length::new(90.0)?),
        Err(RuntimeError::GestureInProgress)
    );
    assert_eq!(state.columns()[2].fixed_width(), Some(150.0));
    pointer(
        &mut ui,
        120.0,
        10.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(state.columns()[0].fixed_width(), Some(120.0));
    state.set_column_width("c", Length::new(90.0)?)?;
    assert_eq!(state.columns()[2].fixed_width(), Some(90.0));
    Ok(())
}

#[test]
fn a_saved_width_below_the_minimum_is_rejected_and_the_minimum_restores() -> TestResult {
    let mut f = fixture(5)?;
    let saved = |width: f64| -> Result<Workspace, Box<dyn std::error::Error>> {
        Ok(Workspace::new(
            WorkspaceSettings::default(),
            [(
                WorkspacePath::new(["table"])?,
                ViewState::new("table")?.with(
                    "widths",
                    WorkspaceValue::map([("number".to_owned(), WorkspaceValue::number(width)?)])?,
                )?,
            )],
        )?)
    };
    let before = (cells(&f), f.ui.capture_workspace()?);
    let minimum = f64::from(TableColumn::MIN_WIDTH);
    // The second is just below the minimum, yet rounds up to it as an `f32`.
    for below in [minimum - 1.0, minimum - 1.0e-6] {
        assert_eq!(
            below as f32,
            if below == minimum - 1.0 { 39.0 } else { 40.0 }
        );
        let report = f.ui.restore_workspace(&saved(below)?)?;
        assert_eq!(
            report
                .rejected()
                .map(|(_, reason)| reason.clone())
                .collect::<Vec<_>>(),
            [RestoreRejection::OutOfRange("number".into())],
            "{below} is rejected, not raised to the minimum"
        );
        f.ui.flush(Duration::ZERO)?;
        assert_eq!((cells(&f), f.ui.capture_workspace()?), before, "{below}");
    }

    let at = saved(f64::from(TableColumn::MIN_WIDTH))?;
    assert!(f.ui.restore_workspace(&at)?.is_complete());
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(cells(&f)[0].1, TableColumn::MIN_WIDTH);
    assert_eq!(
        captured_widths(&f.ui.capture_workspace()?),
        [("number".to_owned(), f64::from(TableColumn::MIN_WIDTH))]
    );
    Ok(())
}
