//! A table in tests: its column titles as items with window bounds, which
//! `Driver::click_item` clicks, the cell addresses, and development
//! validation of content taller than its row.
use pert_ui::testing::{Driver, HostIntegration, TestError};
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 24.0;
/// How far a [`Frame`] insets its child, so the table's origin is not its
/// parent's.
const PAD: f32 = 10.0;
/// ControlHeight and ResizeWidth at the default Standard density.
const HEADER: f32 = 32.0;

/// Rows keyed `r0`, `r1`, …, each cell a label keyed `text` showing its column.
struct Cells(usize);
impl TableItems for Cells {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        (index < self.0).then(|| Element::new("text", Label::new(column)))
    }
}

/// A frame of a fixed height, and a fixed or the offered width, that places
/// its one child inset by [`PAD`].
struct Frame {
    width: Option<f32>,
    height: f32,
    child: Option<Element>,
}
impl Component for Frame {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let width = self.width.or(available.width()).unwrap_or(0.0);
        Ok(Size::new(width, self.height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        cx.place(
            0,
            Rect::new(
                PAD,
                PAD,
                size.width() - 2.0 * PAD,
                size.height() - 2.0 * PAD,
            )?,
        )
    }
}
fn framed(key: &str, width: Option<f32>, height: f32, child: Element) -> Element {
    Element::new(
        key,
        Frame {
            width,
            height,
            child: Some(child),
        },
    )
}

/// A leaf that fills the offered width and fails to arrange while `fail` is
/// set.
struct Failing(Rc<Cell<bool>>);
impl Component for Failing {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(available.width().unwrap_or(20.0), 20.0)?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if self.0.get() {
            Err(LayoutError::InvalidChild.into())
        } else {
            Ok(())
        }
    }
}

/// A 100 × 20 leaf reporting two items, each half of it.
struct Segments;
impl Component for Segments {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 20.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        let half =
            |x: f32| SemanticItem::new("half").local_bounds(Rect::new(x, 0.0, 50.0, 20.0).unwrap());
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Group).items([half(0.0), half(50.0)])),
            ..ComponentDetails::default()
        }
    }
}

fn column(key: &str, width: f32) -> TableColumn {
    TableColumn::new(key, key.to_uppercase())
        .width(Length::new(width).unwrap())
        .sortable(true)
}

type Sorts = Rc<RefCell<Vec<TableSort>>>;

/// A table keyed `table` over `rows` rows that records the sorts it is asked
/// for, with its controller.
fn table(
    ui: &Runtime,
    columns: Vec<TableColumn>,
    rows: usize,
    row_height: f32,
) -> Result<(Element, TableController, Sorts), Box<dyn std::error::Error>> {
    let controller = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        columns,
        TableSource::new(Cells(rows)),
    )?;
    let sorts: Sorts = Rc::default();
    let log = sorts.clone();
    let element = Element::new(
        "table",
        Table::new(&controller, row_height)?.on_sort(move |_, sort| {
            log.borrow_mut().push(sort.clone());
            Ok(())
        }),
    );
    Ok((element, controller, sorts))
}

fn ui(width: f32, height: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui
}

fn settle(ui: &mut Runtime) {
    ui.flush(ui.now()).unwrap();
}

fn record(ui: &Runtime, id: &WidgetId) -> ComponentInspection {
    let id = ui.inspection_id(id).unwrap();
    ui.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
        .components
        .remove(0)
}

fn items(ui: &Runtime, id: &WidgetId) -> Vec<SemanticItem> {
    record(ui, id)
        .details
        .semantics
        .expect("the component reports semantics")
        .items
}

fn item_bounds(ui: &Runtime, id: &WidgetId) -> Vec<Option<[f32; 4]>> {
    items(ui, id).into_iter().map(|item| item.bounds).collect()
}

fn border_box(ui: &Runtime, id: &WidgetId) -> Option<[f32; 4]> {
    record(ui, id).details.box_model.border_box
}

fn column_widths(ui: &Runtime, id: &WidgetId) -> Option<InspectValue> {
    record(ui, id)
        .details
        .properties
        .get("column_widths")
        .cloned()
}

fn sorted(sorts: &Sorts) -> Vec<(String, SortDirection)> {
    sorts
        .borrow()
        .iter()
        .map(|sort| (sort.column.clone(), sort.direction))
        .collect()
}

/// Whether `inner` lies within `outer`, both `[x, y, width, height]`.
fn within(inner: [f32; 4], outer: [f32; 4]) -> bool {
    inner[0] >= outer[0]
        && inner[1] >= outer[1]
        && inner[0] + inner[2] <= outer[0] + outer[2]
        && inner[1] + inner[3] <= outer[1] + outer[3]
}

/// Scrolls the table's columns by `by` with a horizontal wheel over its header.
fn wheel(ui: &mut Runtime, table: &WidgetId, by: f32) {
    let bounds = ui.bounds(table).unwrap().unwrap();
    let at = Point::new(bounds.x() + 20.0, bounds.y() + 10.0).unwrap();
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Scroll(Point::new(by, 0.0).unwrap()),
    ))
    .unwrap();
    settle(ui);
}

#[test]
fn header_titles_report_their_window_bounds_and_click_item_sorts_by_them() -> TestResult {
    let mut ui = ui(400.0, 200.0);
    let (element, _, sorts) = table(
        &ui,
        vec![column("a", 80.0), TableColumn::new("b", "B").sortable(true)],
        3,
        ROW,
    )?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    settle(&mut ui);
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    // The frame insets the table by PAD; the flexible column takes the rest.
    assert_eq!(
        item_bounds(&ui, &table),
        [
            Some([PAD, PAD, 80.0, HEADER]),
            Some([PAD + 80.0, PAD, 400.0 - 2.0 * PAD - 80.0, HEADER]),
        ]
    );
    let mut driver = Driver::new(&mut ui);
    driver.click_item(&table, 1)?;
    assert_eq!(sorted(&sorts), [("b".to_owned(), SortDirection::Ascending)]);
    driver.advance(ClickCounter::DEFAULT_INTERVAL + Driver::FRAME)?;
    driver.click_item(&table, 1)?;
    assert_eq!(
        sorted(&sorts)[1..],
        [("b".to_owned(), SortDirection::Descending)]
    );
    Ok(())
}

/// Four 200 px columns in a 300 px table, scrolled 340 px: `a` is out of view,
/// `b` shows its last 60 px at the table's left edge.
fn scrolled() -> Result<(Runtime, WidgetId, TableController, Sorts), Box<dyn std::error::Error>> {
    let mut ui = ui(320.0, 200.0);
    let (element, controller, sorts) = table(
        &ui,
        ["a", "b", "c", "d"].map(|key| column(key, 200.0)).to_vec(),
        3,
        ROW,
    )?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    settle(&mut ui);
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    wheel(&mut ui, &table, 340.0);
    Ok((ui, table, controller, sorts))
}

#[test]
fn a_title_cut_by_horizontal_scrolling_reports_and_clicks_its_visible_part() -> TestResult {
    let (mut ui, table, _, sorts) = scrolled()?;
    let bounds = item_bounds(&ui, &table);
    assert_eq!(bounds[1], Some([PAD, PAD, 60.0, HEADER]));
    // The third title fits; the fourth is cut by the table's right edge.
    assert_eq!(bounds[2], Some([PAD + 60.0, PAD, 200.0, HEADER]));
    assert_eq!(bounds[3], Some([PAD + 260.0, PAD, 40.0, HEADER]));
    Driver::new(&mut ui).click_item(&table, 1)?;
    assert_eq!(sorted(&sorts), [("b".to_owned(), SortDirection::Ascending)]);
    Ok(())
}

#[test]
fn a_title_scrolled_out_of_view_has_no_bounds_and_is_not_clicked() -> TestResult {
    let (mut ui, table, _, sorts) = scrolled()?;
    assert_eq!(item_bounds(&ui, &table)[0], None);
    assert_eq!(
        Driver::new(&mut ui).click_item(&table, 0),
        Err(TestError::ItemWithoutBounds {
            key: "table".into(),
            index: 0
        })
    );
    assert!(sorts.borrow().is_empty());
    Ok(())
}

#[test]
fn a_press_on_a_cut_title_within_its_edges_grip_resizes_instead_of_sorting() -> TestResult {
    let mut ui = ui(320.0, 200.0);
    // The flexible column has no spare width and keeps its 40 px minimum.
    let (element, controller, sorts) = table(
        &ui,
        vec![
            TableColumn::new("a", "A").sortable(true).resizable(true),
            column("b", 200.0),
            column("c", 200.0),
            column("d", 200.0),
        ],
        3,
        ROW,
    )?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    settle(&mut ui);
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    // 4 px of `a` remain, so the middle of its visible part is 2 px from its
    // trailing edge, inside the Standard 8 px grip.
    wheel(&mut ui, &table, 36.0);
    assert_eq!(item_bounds(&ui, &table)[0], Some([PAD, PAD, 4.0, HEADER]));
    assert_eq!(controller.columns()[0].fixed_width(), None);
    Driver::new(&mut ui).click_item(&table, 0)?;
    assert!(sorts.borrow().is_empty(), "the press resized");
    // The resize fixed the column at its width, which makes it user sized.
    assert_eq!(controller.columns()[0].fixed_width(), Some(40.0));
    assert!(ui.capture_workspace()?.encode().contains("\"a\"=40"));
    Ok(())
}

#[test]
fn titles_follow_a_view_scale() -> TestResult {
    let mut ui = ui(600.0, 400.0);
    let zoom = ZoomController::new(&ui);
    zoom.set(ViewTransform::new(ViewScale::new(2.0)?, Point::ZERO))?;
    let (element, _, sorts) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(Element::new(
        "zoom",
        ZoomView::new([(Point::ZERO, framed("frame", Some(200.0), 140.0, element))])
            .controller(&zoom),
    ))?;
    settle(&mut ui);
    let table = ui.find(&root, &["frame", "table"])?.ok_or("mounted")?;
    let widths: Vec<f32> = item_bounds(&ui, &table)
        .into_iter()
        .map(|bounds| bounds.map_or(0.0, |bounds| bounds[2]))
        .collect();
    assert_eq!(widths, [160.0, 200.0]);
    assert_eq!(record(&ui, &table).view_scale, 2.0);
    Driver::new(&mut ui).click_item(&table, 1)?;
    assert_eq!(sorted(&sorts), [("b".to_owned(), SortDirection::Ascending)]);
    Ok(())
}

/// A table below a 1000 px spacer in a scroll area, out of view.
fn below_the_fold() -> Result<(Runtime, WidgetId, Sorts), Box<dyn std::error::Error>> {
    let mut ui = ui(400.0, 200.0);
    let (element, _, sorts) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Column::new([
                framed(
                    "spacer",
                    Some(40.0),
                    1000.0,
                    Element::new("empty", Label::new("")),
                ),
                framed("frame", Some(300.0), 160.0, element),
            ]),
        )),
    ))?;
    settle(&mut ui);
    let table = ui
        .find(&root, &["content", "frame", "table"])?
        .ok_or("mounted")?;
    assert_eq!(item_bounds(&ui, &table), [None, None], "out of view");
    Ok((ui, table, sorts))
}

#[test]
fn an_index_out_of_range_is_refused_before_the_reveal_scrolls() -> TestResult {
    let (mut ui, table, sorts) = below_the_fold()?;
    let before = ui.bounds(&table)?;
    assert_eq!(
        Driver::new(&mut ui).click_item(&table, 9),
        Err(TestError::NoItem {
            key: "table".into(),
            index: 9,
            count: 2
        })
    );
    assert_eq!(ui.bounds(&table)?, before, "nothing scrolled");
    assert!(sorts.borrow().is_empty());
    Ok(())
}

#[test]
fn click_item_reads_the_items_again_after_a_reveal_that_scrolls() -> TestResult {
    let (mut ui, table, sorts) = below_the_fold()?;
    Driver::new(&mut ui).click_item(&table, 0)?;
    assert_eq!(sorted(&sorts), [("a".to_owned(), SortDirection::Ascending)]);
    Ok(())
}

#[test]
fn titles_have_no_bounds_before_the_first_layout() -> TestResult {
    let mut ui = ui(400.0, 200.0);
    let (element, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    assert_eq!(item_bounds(&ui, &table), [None, None]);
    assert_eq!(record(&ui, &table).view_scale, 1.0);
    Ok(())
}

#[test]
fn a_width_change_not_yet_laid_out_shows_in_the_titles_after_the_flush() -> TestResult {
    let mut ui = ui(400.0, 200.0);
    let (element, controller, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    settle(&mut ui);
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    let before = item_bounds(&ui, &table);
    controller.set_column_width("a", Length::new(120.0)?)?;
    assert_eq!(item_bounds(&ui, &table), before, "the last layout's titles");
    settle(&mut ui);
    assert_eq!(
        item_bounds(&ui, &table),
        [
            Some([PAD, PAD, 120.0, HEADER]),
            Some([PAD + 120.0, PAD, 100.0, HEADER]),
        ]
    );
    Ok(())
}

/// A frame [`PAD`] around its child whose width a signal sets; a change lays
/// the frame out again, but not its child.
struct Resizing {
    width: ReadSignal<f32>,
    current: f32,
    child: Option<Element>,
}
impl Component for Resizing {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read(&self.width)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.current, 150.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(
            0,
            Rect::new(
                PAD,
                PAD,
                size.width() - 2.0 * PAD,
                size.height() - 2.0 * PAD,
            )?,
        )
    }
}

/// A leaf that fails to arrange while `fail` holds true; a change of it lays
/// the leaf out again.
struct FailsOn {
    fail: ReadSignal<bool>,
    failing: bool,
}
impl Component for FailsOn {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.failing = cx.read(&self.fail)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(20.0, 20.0)?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if self.failing {
            Err(LayoutError::InvalidChild.into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn a_failed_layout_withholds_the_titles_it_arranged_and_the_next_one_restores_them() -> TestResult {
    let mut ui = ui(400.0, 300.0);
    let (width, fail) = (ui.signal(400.0_f32), ui.signal(false));
    let (element, _, _) = table(
        &ui,
        vec![column("a", 80.0), TableColumn::new("b", "B")],
        3,
        ROW,
    )?;
    // Only the frame and the failing leaf read the signals, so a pass that
    // returns to the first width meets a clean table at its committed
    // rectangle, which only the failure-path invalidation arranges again.
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new(
                "frame",
                Resizing {
                    width: width.read_only(),
                    current: 0.0,
                    child: Some(element),
                },
            ),
            Element::new(
                "failing",
                FailsOn {
                    fail: fail.read_only(),
                    failing: false,
                },
            ),
            Element::new("later", Segments),
        ]),
    ))?;
    settle(&mut ui);
    let table = ui.find(&root, &["frame", "table"])?.ok_or("mounted")?;
    let later = ui.find(&root, &["later"])?.ok_or("mounted")?;
    let (first, first_box, first_widths) = (
        item_bounds(&ui, &table),
        border_box(&ui, &table),
        column_widths(&ui, &table),
    );
    let later_items = item_bounds(&ui, &later);
    assert!(first.iter().all(Option::is_some) && later_items.iter().all(Option::is_some));

    // The narrower pass arranges the table at a new width, then fails.
    width.set(300.0)?;
    fail.set(true)?;
    assert!(ui.flush(ui.now()).is_err());
    assert_eq!(item_bounds(&ui, &table), [None, None]);
    assert_eq!(
        border_box(&ui, &table),
        first_box,
        "the committed box stays"
    );
    assert_eq!(
        item_bounds(&ui, &later),
        later_items,
        "a component the pass never reached keeps its items"
    );

    // Back at the first width, the table is arranged again and pairs its
    // columns with its bounds.
    width.set(400.0)?;
    fail.set(false)?;
    settle(&mut ui);
    assert_eq!(item_bounds(&ui, &table), first);
    assert_eq!(column_widths(&ui, &table), first_widths);
    Ok(())
}

#[test]
fn a_hidden_table_reports_no_title_bounds_but_keeps_its_own() -> TestResult {
    let mut ui = ui(400.0, 200.0);
    let (element, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(framed("frame", None, 200.0, element))?;
    settle(&mut ui);
    let table = ui.find(&root, &["table"])?.ok_or("mounted")?;
    ui.set_visible(&table, false)?;
    settle(&mut ui);
    assert_eq!(item_bounds(&ui, &table), [None, None]);
    assert!(ui.bounds(&table)?.is_some());
    Ok(())
}

#[test]
fn a_table_in_a_promoted_surface_or_a_detached_window_reports_its_own_windows_bounds() -> TestResult
{
    let mut ui = ui(800.0, 600.0);
    let zoom = ZoomController::new(&ui);
    zoom.set(ViewTransform::new(ViewScale::new(2.0)?, Point::ZERO))?;
    let (popover_table, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let (view_table, _, sorts) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let open = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "zoom",
        ZoomView::new([
            (
                Point::ZERO,
                Element::new(
                    "popup",
                    Popover::new(
                        open,
                        Element::new("anchor", Label::new("Anchor")),
                        framed("popup-frame", Some(200.0), 140.0, popover_table),
                    ),
                ),
            ),
            (
                Point::new(0.0, 100.0)?,
                Element::new(
                    "view",
                    View::new(framed("frame", Some(200.0), 140.0, view_table)),
                ),
            ),
        ])
        .controller(&zoom),
    ))?;
    settle(&mut ui);
    let widths = |ui: &Runtime, table: &WidgetId| -> Vec<f32> {
        item_bounds(ui, table)
            .into_iter()
            .map(|bounds| bounds.map_or(0.0, |bounds| bounds[2]))
            .collect()
    };
    // A promoted surface leaves the view transform.
    let surfaced = ui.find_descendant(&root, "popup-frame")?.ok_or("mounted")?;
    let surfaced = ui.find(&surfaced, &["table"])?.ok_or("mounted")?;
    assert_eq!(widths(&ui, &surfaced), [80.0, 100.0]);
    let outer = border_box(&ui, &surfaced).ok_or("laid out")?;
    assert!(
        item_bounds(&ui, &surfaced)
            .into_iter()
            .flatten()
            .all(|bounds| within(bounds, outer))
    );

    // So does a detached view, in its own window's coordinates.
    let view = ui.find(&root, &["view"])?.ok_or("mounted")?;
    let detached_table = ui.find(&view, &["frame", "table"])?.ok_or("mounted")?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Table")?)?;
    let detached = ui.commit_detach(prepared)?;
    settle(&mut ui);
    ui.select_window(&primary)?;
    settle(&mut ui);
    let detached_record = record(&ui, &detached_table);
    assert_ne!(detached_record.window, record(&ui, &root).window);
    assert_eq!(detached_record.view_scale, 1.0);
    assert_eq!(widths(&ui, &detached_table), [80.0, 100.0]);
    let outer = border_box(&ui, &detached_table).ok_or("laid out")?;
    assert!(
        item_bounds(&ui, &detached_table)
            .into_iter()
            .flatten()
            .all(|bounds| within(bounds, outer))
    );
    assert_eq!(
        Driver::new(&mut ui).click_item(&detached_table, 1),
        Err(TestError::OtherWindow {
            key: "table".into()
        })
    );
    ui.select_window(&detached)?;
    Driver::new(&mut ui).click_item(&detached_table, 1)?;
    assert_eq!(sorted(&sorts), [("b".to_owned(), SortDirection::Ascending)]);
    Ok(())
}

#[test]
fn a_restored_column_order_is_resolved_at_once() -> TestResult {
    let columns = || {
        vec![
            column("a", 80.0).movable(true),
            column("b", 120.0).movable(true),
        ]
    };
    // Moves `a` after `b` from the keyboard and captures the order.
    let mut source = ui(400.0, 200.0);
    let (element, controller, _) = table(&source, columns(), 3, ROW)?;
    let table_id = source.mount_tree(element)?;
    settle(&mut source);
    let body = source.children(&table_id)?[0].clone();
    source.focus(&body)?;
    controller.selection().set_active(Some("r0".into()))?;
    controller.set_active_column(Some("a".into()))?;
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    Driver::new(&mut source).chord(
        KeyChord::new(CommandKey::Named(KeyCode::ArrowRight), alt),
        KeyCode::ArrowRight,
    )?;
    assert_eq!(controller.columns()[0].key(), "b");
    let moved = source.capture_workspace()?;

    let mut ui = ui(400.0, 200.0);
    let (element, _, _) = table(&ui, columns(), 3, ROW)?;
    let table = ui.mount_tree(element)?;
    settle(&mut ui);
    assert!(ui.restore_workspace(&moved)?.is_complete());
    // Before any layout, the first title is the restored first column at
    // that column's width.
    let first = items(&ui, &table).remove(0);
    assert_eq!(first.name.as_deref(), Some("B"));
    assert_eq!(first.bounds.map(|bounds| bounds[2]), Some(120.0));
    Ok(())
}

/// A zoom view at 200 % beside a plain column, each in a fixed frame.
#[test]
fn a_table_reparented_into_another_scale_keeps_its_committed_scale_until_laid_out() -> TestResult {
    let mut ui = ui(800.0, 600.0);
    let zoom = ZoomController::new(&ui);
    zoom.set(ViewTransform::new(ViewScale::new(2.0)?, Point::ZERO))?;
    let (element, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new(
                "plain",
                Column::new([framed("frame", Some(200.0), 140.0, element)]),
            ),
            Element::new("zoom", ZoomView::new([]).controller(&zoom)),
        ]),
    ))?;
    settle(&mut ui);
    let frame = ui.find(&root, &["plain", "frame"])?.ok_or("mounted")?;
    let table = ui.find(&frame, &["table"])?.ok_or("mounted")?;
    let zoom_view = ui.find(&root, &["zoom"])?.ok_or("mounted")?;
    let outer = border_box(&ui, &table).ok_or("laid out")?;
    ui.reparent(&frame, &zoom_view, 0)?;
    let moved = record(&ui, &table);
    assert_eq!(moved.view_scale, 1.0, "the committed layout's scale");
    assert_eq!(moved.details.box_model.border_box, Some(outer));
    let bounds = item_bounds(&ui, &table);
    assert_eq!(
        bounds.iter().map(|b| b.map(|b| b[2])).collect::<Vec<_>>(),
        [Some(80.0), Some(100.0)]
    );
    assert!(
        bounds
            .into_iter()
            .flatten()
            .all(|bounds| within(bounds, outer))
    );
    settle(&mut ui);
    assert_eq!(record(&ui, &table).view_scale, 2.0);
    let widths: Vec<_> = item_bounds(&ui, &table)
        .into_iter()
        .map(|b| b.map(|b| b[2]))
        .collect();
    assert_eq!(widths, [Some(160.0), Some(200.0)]);
    Ok(())
}

#[test]
fn a_zoom_whose_layout_fails_before_the_table_leaves_its_committed_scale() -> TestResult {
    let mut ui = ui(800.0, 600.0);
    let zoom = ZoomController::new(&ui);
    let fail = Rc::new(Cell::new(false));
    let (element, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(Element::new(
        "zoom",
        ZoomView::new([
            (Point::ZERO, Element::new("failing", Failing(fail.clone()))),
            (
                Point::new(0.0, 40.0)?,
                framed("frame", Some(200.0), 140.0, element),
            ),
        ])
        .controller(&zoom),
    ))?;
    settle(&mut ui);
    let table = ui.find(&root, &["frame", "table"])?.ok_or("mounted")?;
    let (before, outer) = (item_bounds(&ui, &table), border_box(&ui, &table));
    fail.set(true);
    zoom.set(ViewTransform::new(ViewScale::new(2.0)?, Point::ZERO))?;
    assert!(ui.flush(ui.now()).is_err());
    let failed = record(&ui, &table);
    assert_eq!(failed.view_scale, 1.0);
    assert_eq!(failed.details.box_model.border_box, outer);
    assert_eq!(item_bounds(&ui, &table), before);
    fail.set(false);
    settle(&mut ui);
    assert_eq!(record(&ui, &table).view_scale, 2.0);
    Ok(())
}

/// A table in a view that a later step detaches, after a failing component
/// that fails while `fail` is set.
fn in_a_view(
    fail: &Rc<Cell<bool>>,
) -> Result<(Runtime, WidgetId, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = ui(800.0, 600.0);
    let (element, _, _) = table(&ui, vec![column("a", 80.0), column("b", 100.0)], 3, ROW)?;
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("title", Label::new("Tables")),
            Element::new(
                "view",
                View::new(Element::new(
                    "content",
                    Column::new([
                        Element::new("failing", Failing(fail.clone())),
                        framed("frame", Some(300.0), 140.0, element),
                    ]),
                )),
            ),
        ]),
    ))?;
    settle(&mut ui);
    let view = ui.find(&root, &["view"])?.ok_or("mounted")?;
    let table = ui
        .find(&view, &["content", "frame", "table"])?
        .ok_or("mounted")?;
    assert!(item_bounds(&ui, &table).iter().all(Option::is_some));
    Ok((ui, view, table))
}

/// Whether the record of `table` carries no geometry of the window it is in.
fn without_geometry(ui: &Runtime, table: &WidgetId) -> bool {
    let record = record(ui, table);
    record.details.box_model.border_box.is_none()
        && record.details.validation.clip.is_none()
        && record
            .details
            .semantics
            .is_some_and(|semantics| semantics.items.iter().all(|item| item.bounds.is_none()))
}

fn window_of(ui: &Runtime, table: &WidgetId) -> u64 {
    record(ui, table).window
}

#[test]
fn a_moved_table_reports_no_geometry_until_its_new_window_lays_it_out() -> TestResult {
    let fail = Rc::new(Cell::new(false));
    let (mut ui, view, table) = in_a_view(&fail)?;
    let primary_serial = window_of(&ui, &table);
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Table")?)?;
    let detached = ui.commit_detach(prepared)?;
    let moved = record(&ui, &table);
    assert_ne!(moved.window, primary_serial, "the destination window");
    assert!(without_geometry(&ui, &table));
    assert_eq!(moved.view_scale, 1.0, "the scale it will be laid out at");
    settle(&mut ui);
    assert!(item_bounds(&ui, &table).iter().all(Option::is_some));
    let detached_serial = window_of(&ui, &table);
    let outer = border_box(&ui, &table).ok_or("laid out")?;
    assert!(
        item_bounds(&ui, &table)
            .into_iter()
            .flatten()
            .all(|bounds| within(bounds, outer))
    );

    // Back again, the same until the primary window lays it out.
    ui.redock(&detached)?;
    assert_ne!(window_of(&ui, &table), detached_serial);
    assert_eq!(window_of(&ui, &table), primary_serial);
    assert!(without_geometry(&ui, &table));
    settle(&mut ui);
    assert!(item_bounds(&ui, &table).iter().all(Option::is_some));
    assert!(border_box(&ui, &table).is_some());
    Ok(())
}

#[test]
fn a_destination_layout_that_fails_before_the_table_leaves_it_without_geometry() -> TestResult {
    let fail = Rc::new(Cell::new(false));
    let (mut ui, view, table) = in_a_view(&fail)?;
    fail.set(true);
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Table")?)?;
    let _detached = ui.commit_detach(prepared)?;
    assert!(
        ui.flush(ui.now()).is_err(),
        "the detached window's first layout fails"
    );
    assert!(without_geometry(&ui, &table));
    fail.set(false);
    settle(&mut ui);
    assert!(item_bounds(&ui, &table).iter().all(Option::is_some));
    assert!(border_box(&ui, &table).is_some());
    Ok(())
}

/// A component of no size.
struct Empty;
impl Component for Empty {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::ZERO)
    }
}

#[test]
fn a_zero_size_component_follows_a_continuing_zoom() -> TestResult {
    let mut ui = ui(400.0, 300.0);
    let zoom = ZoomController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "zoom",
        ZoomView::new([(Point::ZERO, Element::new("empty", Empty))]).controller(&zoom),
    ))?;
    settle(&mut ui);
    let empty = ui.find(&root, &["empty"])?.ok_or("mounted")?;
    // Steps closer together than the view scale's settling delay.
    for scale in [1.25_f32, 1.5, 1.75, 2.0] {
        zoom.set(ViewTransform::new(ViewScale::new(scale)?, Point::ZERO))?;
        ui.flush(ui.now() + Duration::from_millis(20))?;
        assert_eq!(record(&ui, &empty).view_scale, scale);
    }
    Ok(())
}

/// Rows `r0`, `r1`, `r2` whose `state` column holds content keyed `state`, and
/// whose `note` column is edited in place.
struct States;
impl TableItems for States {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("r{index}"))
    }
    fn cell(&self, _: usize, column: &str) -> Option<Element> {
        Some(Element::new(column, Label::new(column)))
    }
}

#[test]
fn a_cell_is_addressed_by_row_column_and_content_keys() -> TestResult {
    let mut ui = ui(400.0, 200.0);
    let controller = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("state", "State"),
            TableColumn::new("note", "Note").editable(true),
        ],
        TableSource::new(States),
    )?;
    let theme = ui.theme();
    let row_height = theme
        .metrics(theme.density()?)?
        .metric(MetricRole::ControlHeight);
    let editor = ui.signal(String::new());
    let table = ui.mount_tree(Element::new(
        "table",
        Table::new(&controller, row_height)?.cell_editor(move |edit| {
            Ok(Element::new(
                "field",
                TextField::new(editor.clone()).session(edit.session()),
            ))
        }),
    ))?;
    settle(&mut ui);
    let content = ui
        .find(&table, &["r1", "state", "state"])?
        .ok_or("content")?;
    assert_eq!(ui.key(&content)?, "state");
    let cell = ui.find(&table, &["r1", "state"])?.ok_or("cell")?;
    assert_ne!(cell, content);
    let role = record(&ui, &cell)
        .details
        .semantics
        .map(|semantics| semantics.role);
    assert!(matches!(role, Some(SemanticRole::Cell)), "{role:?}");
    let row = ui.find(&table, &["r1"])?.ok_or("row")?;
    assert_eq!(
        ui.find_descendant(&row, "state"),
        Err(RuntimeError::AmbiguousKey("state".into()))
    );
    // An edited cell's editor is below the cell under its own key.
    let body = ui.children(&table)?[0].clone();
    ui.focus(&body)?;
    controller.selection().set_active(Some("r1".into()))?;
    controller.set_active_column(Some("note".into()))?;
    Driver::new(&mut ui).key(KeyCode::Enter)?;
    let field = ui
        .find(&table, &["r1", "note", "field"])?
        .ok_or("the editor")?;
    assert_eq!(ui.focused(), Some(field));
    Ok(())
}

/// The issues of `code` in the validation report, as their components' keys.
#[cfg(debug_assertions)]
fn issues(ui: &Runtime, code: ValidationCode) -> Vec<(Vec<String>, String)> {
    let mut found: Vec<_> = ui
        .validation_report()
        .issues
        .into_iter()
        .filter(|issue| issue.code == code)
        .map(|issue| {
            let keys = issue
                .components
                .iter()
                .map(|serial| {
                    ui.inspect()
                        .components
                        .into_iter()
                        .find(|component| component.id == *serial)
                        .map_or_else(String::new, |component| component.key)
                })
                .collect();
            (keys, issue.message)
        })
        .collect();
    found.sort();
    found
}

/// Builds the content of a row's cell in a column.
type CellFn = Rc<dyn Fn(usize, &str) -> Option<Element>>;

/// Rows `r0`, `r1`, `r2` whose cells are built by `cell(row, column)`.
struct Built(CellFn);
impl TableItems for Built {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        (self.0)(index, column)
    }
}

/// A table of `columns` over [`Built`] rows at `row_height`, with a text
/// field as the cell editor; settled.
#[cfg(debug_assertions)]
fn built(
    row_height: f32,
    columns: Vec<TableColumn>,
    cell: impl FnOnce(&Runtime) -> CellFn,
) -> Result<(Runtime, WidgetId, TableController), Box<dyn std::error::Error>> {
    let mut ui = ui(600.0, 300.0);
    let cell = cell(&ui);
    let controller = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        columns,
        TableSource::new(Built(cell)),
    )?;
    let editor = ui.signal(String::new());
    let table = ui.mount_tree(Element::new(
        "table",
        Table::new(&controller, row_height)?.cell_editor(move |edit| {
            Ok(Element::new(
                "editor",
                TextField::new(editor.clone()).session(edit.session()),
            ))
        }),
    ))?;
    settle(&mut ui);
    Ok((ui, table, controller))
}

#[cfg(debug_assertions)]
#[test]
fn a_button_laid_out_taller_than_its_row_is_reported_once_per_row() -> TestResult {
    let button = |_: &Runtime| -> CellFn {
        Rc::new(|_, _| Some(Element::new("button", Button::new("Go", |_| Ok(())))))
    };
    let (ui, table, _) = built(28.0, vec![TableColumn::new("go", "Go")], button)?;
    let found = issues(&ui, ValidationCode::ExceedsFixedHeight);
    assert_eq!(found.len(), 3, "{found:?}");
    for (keys, message) in &found {
        assert_eq!(keys, &["button", "go"]);
        assert!(
            message.contains("measured 32 px tall in \"go\"")
                && message.contains("which gives its children a fixed 28 px height, so it is compressed or cut off; give the parent more height, such as a taller Table row, or use smaller content"),
            "{message}"
        );
    }
    let cell = ui.find(&table, &["r0", "go"])?.ok_or("cell")?;
    let content = ui.find(&cell, &["button"])?.ok_or("button")?;
    let (content, cell) = (ui.inspection_id(&content)?, ui.inspection_id(&cell)?);
    let issue = ui
        .validation_report()
        .issues
        .into_iter()
        .find(|issue| issue.components.first() == Some(&content))
        .ok_or("the first row's issue")?;
    assert_eq!(issue.components, [content, cell]);
    assert_eq!(
        issue.message,
        format!(
            "Component \"button\" (#{content}): measured 32 px tall in \"go\" (#{cell}), which gives its children a fixed 28 px height, so it is compressed or cut off; give the parent more height, such as a taller Table row, or use smaller content"
        )
    );

    let (ui, _, _) = built(32.0, vec![TableColumn::new("go", "Go")], button)?;
    assert!(issues(&ui, ValidationCode::ExceedsFixedHeight).is_empty());
    Ok(())
}

/// A text field, a checkbox, an editable note and a label of text that
/// wraps in its column, in `row_height` rows; the label is cut to one line
/// with an ellipsis when `ellipsis` is set.
#[cfg(debug_assertions)]
fn controls(
    row_height: f32,
    ellipsis: bool,
) -> Result<(Runtime, WidgetId, TableController), Box<dyn std::error::Error>> {
    let cell = move |ui: &Runtime| -> CellFn {
        let texts: Vec<Signal<String>> = (0..3).map(|_| ui.signal(String::new())).collect();
        Rc::new(move |index: usize, column: &str| -> Option<Element> {
            Some(match column {
                "field" => Element::new("field", TextField::new(texts.get(index)?.clone())),
                "check" => {
                    Element::new("checkbox", Checkbox::controlled(false, "On", |_, _| Ok(())))
                }
                "note" => Element::new("note", Label::new("Note")),
                "wrap" => {
                    let label = Label::new("alpha beta");
                    let label = if ellipsis {
                        label.overflow(TextOverflow::Ellipsis)
                    } else {
                        label
                    };
                    Element::new("label", label)
                }
                _ => return None,
            })
        })
    };
    built(
        row_height,
        vec![
            TableColumn::new("field", "Field").width(Length::new(120.0)?),
            TableColumn::new("check", "Check").width(Length::new(120.0)?),
            TableColumn::new("note", "Note")
                .width(Length::new(120.0)?)
                .editable(true),
            TableColumn::new("wrap", "Wrap").width(Length::new(70.0)?),
        ],
        cell,
    )
}

#[cfg(debug_assertions)]
fn open_note(ui: &mut Runtime, table: &WidgetId, controller: &TableController) -> TestResult {
    let body = ui.children(table)?[0].clone();
    ui.focus(&body)?;
    controller.selection().set_active(Some("r1".into()))?;
    controller.set_active_column(Some("note".into()))?;
    Driver::new(ui).key(KeyCode::Enter)?;
    ui.find(table, &["r1", "note", "editor"])?
        .ok_or("the editor")?;
    Ok(())
}

#[cfg(debug_assertions)]
fn exceeding(ui: &Runtime) -> Vec<Vec<String>> {
    issues(ui, ValidationCode::ExceedsFixedHeight)
        .into_iter()
        .map(|(keys, _)| keys)
        .collect()
}

#[cfg(debug_assertions)]
#[test]
fn content_that_measured_taller_than_its_row_is_reported_once_per_cell() -> TestResult {
    let (mut ui, table, controller) = controls(28.0, false)?;
    let per_row = |keys: &[&str]| -> Vec<Vec<String>> {
        let mut all: Vec<Vec<String>> = (0..3)
            .flat_map(|_| {
                [["field", "field"], ["checkbox", "check"], ["label", "wrap"]]
                    .map(|pair| pair.map(str::to_owned).to_vec())
            })
            .collect();
        if !keys.is_empty() {
            all.push(keys.iter().map(|key| (*key).to_owned()).collect());
        }
        all.sort();
        all
    };
    let before = exceeding(&ui);
    assert_eq!(before, per_row(&[]));
    // While the note is edited, its editor is reported against the editor's
    // host, and the host is not reported against the cell.
    open_note(&mut ui, &table, &controller)?;
    assert_eq!(exceeding(&ui), per_row(&["editor", "cell editor"]));
    Driver::new(&mut ui).key(KeyCode::Escape)?;
    assert!(ui.find(&table, &["r1", "note", "editor"])?.is_none());
    assert_eq!(exceeding(&ui), before);

    // Rows of ControlHeight hold the single-line controls and the editor; the
    // two lines of the wrapping label need 40 px.
    let (mut ui, table, controller) = controls(32.0, false)?;
    open_note(&mut ui, &table, &controller)?;
    let tall: Vec<Vec<String>> = (0..3)
        .map(|_| vec!["label".to_owned(), "wrap".to_owned()])
        .collect();
    assert_eq!(exceeding(&ui), tall);
    let (ui, _, _) = controls(40.0, false)?;
    assert!(exceeding(&ui).is_empty());
    // A label cut to one line with an ellipsis fits a row of a line's height.
    let (ui, _, _) = controls(28.0, true)?;
    assert!(
        !exceeding(&ui).iter().any(|keys| keys[0] == "label"),
        "{:?}",
        exceeding(&ui)
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn content_keyed_like_the_editors_host_is_reported_once_against_the_tables_wrapper() -> TestResult {
    let content = |_: &Runtime| -> CellFn {
        Rc::new(|_, _| Some(Element::new("cell editor", Button::new("Go", |_| Ok(())))))
    };
    let (ui, _, _) = built(28.0, vec![TableColumn::new("go", "Go")], content)?;
    let found = exceeding(&ui);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found.iter().all(|keys| keys == &["cell editor", "content"]),
        "{found:?}"
    );
    Ok(())
}
