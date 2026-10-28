//! A grown child of a bounded row or column shrinks to the space left, so a
//! collection below a toolbar mounts only the rows it shows; and the
//! development diagnostic for a collection that was given its whole extent.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A leaf with a natural size and a minimum.
struct Leaf {
    natural: (f32, f32),
    minimum: (f32, f32),
}
impl Component for Leaf {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.natural.0, self.natural.1)?)
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::at_least(Size::new(self.minimum.0, self.minimum.1).unwrap_or(Size::ZERO))
    }
}
fn leaf(key: &str, height: f32, minimum: f32) -> Element {
    Leaf {
        natural: (50.0, height),
        minimum: (0.0, minimum),
    }
    .key(key)
}

/// A leaf whose propagated limits bound it on both sides.
struct Limited {
    natural: f32,
    minimum: f32,
    maximum: f32,
}
impl Component for Limited {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(50.0, self.natural)?)
    }
    fn measure_limits(&self, _: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        Ok(SizeLimits::new(
            Size::new(0.0, self.minimum)?,
            Constraints::new(None, Some(self.maximum))?,
        )?)
    }
}

struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("r{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Label::new(format!("row {index}")).key("text"))
    }
}
struct Cells(usize);
impl TableItems for Cells {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        Some(Label::new(format!("row {index}")).key("text"))
    }
}

fn mount(
    width: f32,
    height: f32,
    root: Element,
) -> Result<(Runtime, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
    let id = ui.mount_tree(root)?;
    // Settle: the first layout gives the lists their viewports.
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    Ok((ui, id))
}
fn height(ui: &Runtime, root: &WidgetId, key: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let id = ui.find_descendant(root, key)?.ok_or(format!("no {key}"))?;
    Ok(ui.bounds(&id)?.ok_or("laid out")?.height())
}
fn table(ui: &Runtime, rows: usize) -> Result<Element, Box<dyn std::error::Error>> {
    let controller = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        [TableColumn::new("a", "A")],
        TableSource::new(Cells(rows)),
    )?;
    Ok(Element::new("table", Table::new(&controller, 20.0)?))
}
fn list(ui: &Runtime, rows: usize, row: f32) -> Result<Element, Box<dyn std::error::Error>> {
    let items = ui.signal(ListSource::new(Rows(rows)));
    Ok(Element::new(
        "list",
        VirtualList::new(items.read_only(), row)?,
    ))
}
fn mounted_rows(ui: &Runtime) -> usize {
    ui.inspect()
        .components
        .iter()
        .filter(|component| {
            component.key.starts_with('r') && component.key[1..].parse::<usize>().is_ok()
        })
        .count()
}
#[cfg(debug_assertions)]
fn unbounded(ui: &Runtime) -> Vec<String> {
    ui.validation_report()
        .issues
        .into_iter()
        .filter(|issue| issue.code == ValidationCode::UnboundedCollection)
        .map(|issue| issue.message)
        .collect()
}

#[test]
fn a_grown_table_below_a_toolbar_fills_the_rest_and_mounts_a_screenful() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let table = {
        let controller = TableController::new(
            &ui,
            ListSelection::new(&ui, SelectionMode::Single),
            [TableColumn::new("a", "A")],
            TableSource::new(Cells(5000)),
        )?;
        Element::new("table", Table::new(&controller, 20.0)?)
    };
    let id = ui.mount_tree(
        Column::new([leaf("toolbar", 40.0, 40.0), table.with_grow(1)])
            .gap(Length::new(0.0)?)
            .align(Align::Stretch)
            .key("page"),
    )?;
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    assert_eq!(height(&ui, &id, "table")?, 560.0);
    // 28 rows show; overscan adds a few.
    let mounted = mounted_rows(&ui);
    assert!((28..60).contains(&mounted), "{mounted} rows mounted");
    #[cfg(debug_assertions)]
    assert!(unbounded(&ui).is_empty());
    Ok(())
}

#[test]
fn grown_children_shrink_by_weight_down_to_their_minimums() -> TestResult {
    let (ui, root) = mount(
        100.0,
        400.0,
        Column::new([
            leaf("fixed", 100.0, 0.0),
            leaf("one", 300.0, 0.0).with_grow(1),
            leaf("three", 300.0, 0.0).with_grow(3),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    // 700 natural in 400: the 300 deficit goes 1:3, 75 and 225.
    assert_eq!(height(&ui, &root, "fixed")?, 100.0);
    assert_eq!(height(&ui, &root, "one")?, 225.0);
    assert_eq!(height(&ui, &root, "three")?, 75.0);
    // A minimum stops one; the other gives the rest.
    let (ui, root) = mount(
        100.0,
        400.0,
        Column::new([
            leaf("fixed", 100.0, 0.0),
            leaf("one", 300.0, 0.0).with_grow(1),
            leaf("three", 300.0, 250.0).with_grow(3),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(height(&ui, &root, "three")?, 250.0);
    assert_eq!(height(&ui, &root, "one")?, 50.0);
    Ok(())
}

#[test]
fn the_floor_is_ungrown_natural_heights_plus_grown_minimums() -> TestResult {
    let (ui, root) = mount(
        100.0,
        100.0,
        Column::new([
            leaf("tall", 120.0, 0.0),
            leaf("grown", 80.0, 20.0).with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(height(&ui, &root, "tall")?, 120.0);
    assert_eq!(height(&ui, &root, "grown")?, 20.0);
    // The children overflow the 100-pixel column to the floor.
    let grown = ui.find_descendant(&root, "grown")?.ok_or("grown")?;
    let bounds = ui.bounds(&grown)?.ok_or("laid out")?;
    assert_eq!(
        bounds.y() + bounds.height(),
        140.0,
        "it overflows at the floor"
    );
    Ok(())
}

#[test]
fn a_huge_natural_height_shrinks_exactly_to_the_space_left() -> TestResult {
    // Twenty million 24-pixel rows: f32 arithmetic on the deficit would lose
    // the few hundred pixels that remain.
    let (ui, root) = mount(
        100.0,
        600.0,
        // A row measures its child against its bounded height and keeps
        // the height the child reports.
        Row::new([Element::new(
            "page",
            Column::new([
                leaf("toolbar", 40.0, 40.0),
                leaf("list", 480_000_000.0, 0.0).with_grow(1),
            ])
            .gap(Length::new(0.0)?),
        )])
        .align(Align::Start)
        .key("root"),
    )?;
    assert_eq!(height(&ui, &root, "list")?, 560.0);
    assert_eq!(height(&ui, &root, "page")?, 600.0);
    let list = ui.find_descendant(&root, "list")?.ok_or("list")?;
    let bounds = ui.bounds(&list)?.ok_or("laid out")?;
    assert_eq!(bounds.y() + bounds.height(), 600.0);
    Ok(())
}

#[test]
fn fractional_shares_fill_the_column_and_report_its_limit() -> TestResult {
    let (ui, root) = mount(
        100.0,
        100.0,
        // A row measures its child against its bounded height and keeps
        // the height the child reports.
        Row::new([Element::new(
            "page",
            Column::new([
                leaf("a", 100.0, 0.0).with_grow(1),
                leaf("b", 100.0, 0.0).with_grow(1),
                leaf("c", 100.0, 0.0).with_grow(1),
            ])
            .gap(Length::new(0.0)?),
        )])
        .align(Align::Start)
        .key("root"),
    )?;
    assert_eq!(height(&ui, &root, "page")?, 100.0);
    let c = ui.find_descendant(&root, "c")?.ok_or("c")?;
    let bounds = ui.bounds(&c)?.ok_or("laid out")?;
    assert!(
        (bounds.y() + bounds.height() - 100.0).abs() < 1e-3,
        "{bounds:?}"
    );
    for key in ["a", "b", "c"] {
        assert!((height(&ui, &root, key)? - 100.0 / 3.0).abs() < 1e-3);
    }
    Ok(())
}

#[test]
fn propagated_minimums_and_explicit_maximums_hold_while_shrinking() -> TestResult {
    let column = |root: &str| -> Result<Element, Box<dyn std::error::Error>> {
        Ok(Column::new([
            leaf("fixed", 100.0, 0.0),
            Limited {
                natural: 500.0,
                minimum: 150.0,
                maximum: 200.0,
            }
            .key("limited")
            .with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key(root))
    };
    // 200 high: the deficit would take it below its propagated minimum.
    let (ui, root) = mount(100.0, 200.0, column("page")?)?;
    assert_eq!(height(&ui, &root, "limited")?, 150.0);
    // 400 high: its natural height is held to its maximum, and growing
    // stops there.
    let (ui, root) = mount(100.0, 400.0, column("page")?)?;
    assert_eq!(height(&ui, &root, "limited")?, 200.0);
    Ok(())
}

#[test]
fn the_floor_counts_gaps_and_the_column_reports_it() -> TestResult {
    let (ui, root) = mount(
        100.0,
        100.0,
        // A row measures its child against its bounded height and keeps
        // the height the child reports.
        Row::new([Element::new(
            "page",
            Column::new([
                leaf("tall", 120.0, 0.0),
                leaf("grown", 80.0, 20.0).with_grow(1),
            ])
            .gap(Length::new(10.0)?),
        )])
        .align(Align::Start)
        .key("root"),
    )?;
    assert_eq!(height(&ui, &root, "grown")?, 20.0);
    assert_eq!(height(&ui, &root, "page")?, 150.0);
    let grown = ui.find_descendant(&root, "grown")?.ok_or("grown")?;
    let bounds = ui.bounds(&grown)?.ok_or("laid out")?;
    assert_eq!(bounds.y() + bounds.height(), 150.0);
    Ok(())
}

#[test]
fn stretch_and_space_between_place_shrunk_children() -> TestResult {
    let (ui, root) = mount(
        100.0,
        300.0,
        Column::new([
            leaf("fixed", 100.0, 0.0),
            leaf("grown", 400.0, 0.0).with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .align(Align::Stretch)
        .justify(Justify::SpaceBetween)
        .key("page"),
    )?;
    let grown = ui.find_descendant(&root, "grown")?.ok_or("grown")?;
    let bounds = ui.bounds(&grown)?.ok_or("laid out")?;
    assert_eq!(bounds.values(), [0.0, 100.0, 100.0, 200.0]);
    Ok(())
}

#[test]
fn a_grown_row_in_a_column_shrinks_in_height_only() -> TestResult {
    let (ui, root) = mount(
        300.0,
        300.0,
        Column::new([
            leaf("fixed", 100.0, 0.0),
            Row::new([leaf("inside", 400.0, 0.0)])
                .key("row")
                .with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(height(&ui, &root, "row")?, 200.0);
    let inside = ui.find_descendant(&root, "inside")?.ok_or("inside")?;
    assert_eq!(ui.bounds(&inside)?.ok_or("laid out")?.width(), 50.0);
    Ok(())
}

#[test]
fn a_column_grown_only_by_its_parent_keeps_its_childrens_heights() -> TestResult {
    let (ui, root) = mount(
        100.0,
        300.0,
        Column::new([
            leaf("top", 100.0, 0.0),
            Column::new([leaf("a", 150.0, 0.0), leaf("b", 150.0, 0.0)])
                .gap(Length::new(0.0)?)
                .key("nested")
                .with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    // The nested column shrinks to what is left; its own children are not
    // grown, so they keep their heights and overflow it.
    assert_eq!(height(&ui, &root, "nested")?, 200.0);
    assert_eq!(height(&ui, &root, "a")?, 150.0);
    assert_eq!(height(&ui, &root, "b")?, 150.0);
    Ok(())
}

#[test]
fn nested_columns_shrink_at_every_bounded_level() -> TestResult {
    let (ui, root) = mount(
        100.0,
        300.0,
        Column::new([
            leaf("top", 100.0, 0.0),
            Column::new([
                leaf("inner-top", 50.0, 0.0),
                leaf("inner", 400.0, 0.0).with_grow(1),
            ])
            .gap(Length::new(0.0)?)
            .key("nested")
            .with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(height(&ui, &root, "nested")?, 200.0);
    assert_eq!(height(&ui, &root, "inner")?, 150.0);
    Ok(())
}

fn width(ui: &Runtime, root: &WidgetId, key: &str) -> Result<f32, Box<dyn std::error::Error>> {
    let id = ui.find_descendant(root, key)?.ok_or(format!("no {key}"))?;
    Ok(ui.bounds(&id)?.ok_or("laid out")?.width())
}
/// A leaf 20 px high with a natural width and a minimum width.
fn wide(key: &str, width: f32, minimum: f32) -> Element {
    Leaf {
        natural: (width, 20.0),
        minimum: (minimum, 0.0),
    }
    .key(key)
}

#[test]
fn grown_children_of_a_bounded_row_shrink_by_weight_down_to_their_minimums() -> TestResult {
    // Two grown 300 px children of a 400 px row each give up half the deficit.
    let (ui, root) = mount(
        400.0,
        100.0,
        Row::new([
            wide("a", 300.0, 0.0).with_grow(1),
            wide("b", 300.0, 0.0).with_grow(1),
        ])
        .gap(Length::new(0.0)?)
        .key("row"),
    )?;
    assert_eq!(width(&ui, &root, "a")?, 200.0);
    assert_eq!(width(&ui, &root, "b")?, 200.0);
    // The column's shortfall, transposed: 700 natural in 400, the 300 deficit
    // goes 1:3, 75 and 225.
    let (ui, root) = mount(
        400.0,
        100.0,
        Row::new([
            wide("fixed", 100.0, 0.0),
            wide("one", 300.0, 0.0).with_grow(1),
            wide("three", 300.0, 0.0).with_grow(3),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(width(&ui, &root, "fixed")?, 100.0);
    assert_eq!(width(&ui, &root, "one")?, 225.0);
    assert_eq!(width(&ui, &root, "three")?, 75.0);
    // A minimum stops one; the other gives the rest.
    let (ui, root) = mount(
        400.0,
        100.0,
        Row::new([
            wide("fixed", 100.0, 0.0),
            wide("one", 300.0, 0.0).with_grow(1),
            wide("three", 300.0, 250.0).with_grow(3),
        ])
        .gap(Length::new(0.0)?)
        .key("page"),
    )?;
    assert_eq!(width(&ui, &root, "three")?, 250.0);
    assert_eq!(width(&ui, &root, "one")?, 50.0);
    Ok(())
}

#[test]
fn a_bounded_row_bounds_a_grown_horizontal_list() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Rows(200)));
    let list = VirtualList::new(items.read_only(), 24.0)?
        .orientation(Orientation::Horizontal)
        .key("list")
        .with_grow(1);
    let root = ui.mount_tree(
        Row::new([wide("side", 100.0, 0.0), list])
            .gap(Length::new(0.0)?)
            .key("row"),
    )?;
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    // Its natural width is its whole 4800 px extent; the row gives it what
    // the side leaves.
    assert_eq!(width(&ui, &root, "list")?, 200.0);
    let mounted = mounted_rows(&ui);
    assert!(mounted < 30, "{mounted} items mounted");
    #[cfg(debug_assertions)]
    assert!(unbounded(&ui).is_empty());
    Ok(())
}

#[test]
fn a_bounded_row_bounds_a_vertical_list_beside_other_children() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let list = list(&ui, 5000, 20.0)?;
    let root = ui.mount_tree(
        Row::new([leaf("side", 100.0, 0.0), list])
            .gap(Length::new(0.0)?)
            .key("row"),
    )?;
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    assert_eq!(height(&ui, &root, "list")?, 400.0);
    // 20 rows show; overscan adds a few.
    let mounted = mounted_rows(&ui);
    assert!((20..40).contains(&mounted), "{mounted} rows mounted");
    #[cfg(debug_assertions)]
    assert!(unbounded(&ui).is_empty());
    Ok(())
}

#[test]
fn a_short_list_in_a_scroll_area_shows_every_row_and_is_not_reported() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 300.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Rows(50)));
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "list",
            VirtualList::new(items.read_only(), 20.0)?,
        )),
    ))?;
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    assert_eq!(mounted_rows(&ui), 50);
    #[cfg(debug_assertions)]
    assert!(unbounded(&ui).is_empty());
    Ok(())
}

#[cfg(debug_assertions)]
mod diagnostic {
    use super::*;

    fn reported(
        build: impl FnOnce(&Runtime) -> Result<Element, Box<dyn std::error::Error>>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
        let root = build(&ui)?;
        ui.mount_tree(root)?;
        for step in 0..4 {
            ui.flush(Duration::from_millis(step))?;
        }
        Ok(unbounded(&ui))
    }

    #[test]
    fn a_table_in_a_scroll_area_or_a_default_tab_is_reported() -> TestResult {
        let scroll = reported(|ui| Ok(Element::new("scroll", ScrollArea::new(table(ui, 500)?))))?;
        assert_eq!(scroll.len(), 1, "{scroll:?}");
        assert!(
            scroll[0].contains("\"body\"") && scroll[0].contains("\"table\""),
            "{scroll:?}"
        );
        let tabs = reported(|ui| {
            Ok(Element::new(
                "tabs",
                Tabs::new([Tab::new("t", "T", table(ui, 500)?)]),
            ))
        })?;
        assert_eq!(tabs.len(), 1, "{tabs:?}");
        let region = reported(|ui| {
            Ok(Element::new(
                "region",
                RegionLayout::new().center(table(ui, 500)?),
            ))
        })?;
        assert_eq!(region.len(), 1, "{region:?}");
        Ok(())
    }

    #[test]
    fn bounded_tables_are_not_reported() -> TestResult {
        for build in [
            |ui: &Runtime| -> Result<Element, Box<dyn std::error::Error>> {
                Ok(Element::new(
                    "tabs",
                    Tabs::new([Tab::new("t", "T", table(ui, 500)?).fixed()]),
                ))
            },
            |ui: &Runtime| -> Result<Element, Box<dyn std::error::Error>> {
                Ok(Element::new(
                    "region",
                    RegionLayout::new().fixed().center(table(ui, 500)?),
                ))
            },
            |ui: &Runtime| -> Result<Element, Box<dyn std::error::Error>> {
                Ok(Element::new("view", View::new(table(ui, 500)?).fixed()))
            },
            |ui: &Runtime| -> Result<Element, Box<dyn std::error::Error>> {
                Ok(Element::new(
                    "split",
                    SplitPane::new(
                        SplitAxis::TopBottom,
                        table(ui, 500)?,
                        Element::new("detail", Label::new("detail")),
                    ),
                ))
            },
        ] {
            assert!(reported(build)?.is_empty());
        }
        Ok(())
    }

    /// Rows ten pixels high, whatever their estimate.
    struct Short(usize);
    impl ListItems for Short {
        fn len(&self) -> usize {
            self.0
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < self.0).then(|| format!("r{index}"))
        }
        fn build(&self, _: usize) -> Option<Element> {
            Some(leaf("row", 10.0, 0.0))
        }
    }

    #[test]
    fn an_estimated_list_is_judged_once_its_extents_settle() -> TestResult {
        // Estimated at 20 pixels, 100 rows are 2 000 pixels: more than the
        // 1 500-pixel frame, so the estimate alone says the list scrolls. Measured
        // at 10 pixels they are 1 000, all of them in the frame, which the window
        // shows less than half of.
        let found = reported(|ui| {
            let items = ui.signal(ListSource::new(Short(100)));
            Ok(Element::new(
                "page",
                Column::new([Element::new(
                    "frame",
                    Container::new(Element::new(
                        "list",
                        VirtualList::estimated(items.read_only(), 20.0)?,
                    ))
                    .preferred_size(Size::new(300.0, 1500.0)?),
                )]),
            ))
        })?;
        assert_eq!(found.len(), 1, "{found:?}");
        Ok(())
    }

    #[test]
    fn a_horizontal_list_in_a_horizontal_scroll_area_is_reported() -> TestResult {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
        let items = ui.signal(ListSource::new(Rows(100)));
        ui.mount_tree(Element::new(
            "scroll",
            ScrollArea::new(Element::new(
                "list",
                VirtualList::new(items.read_only(), 5.0)?.orientation(Orientation::Horizontal),
            ))
            .axes(ScrollAxes::Horizontal),
        ))?;
        for step in 0..4 {
            ui.flush(Duration::from_millis(step))?;
        }
        let found = unbounded(&ui);
        assert_eq!(found.len(), 1, "{found:?}");
        Ok(())
    }

    /// Shows its child at a fixed scale.
    struct Magnifier(ViewTransform, Vec<Element>);
    impl Component for Magnifier {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.1)
        }
        fn child_transform(&self) -> ViewTransform {
            self.0
        }
    }

    #[test]
    fn a_scaled_list_is_judged_by_its_visible_share() -> TestResult {
        // 100 five-pixel rows in a 600-pixel allocation, shown magnified in a
        // 600-pixel window: at 2× exactly half is visible.
        for (scale, expected) in [(2.5, 1), (2.0, 0), (1.6, 0)] {
            let found = reported(|ui| {
                let view = ViewTransform::new(ViewScale::new(scale)?, Point::new(0.0, 0.0)?);
                Ok(Element::new(
                    "magnifier",
                    Magnifier(view, vec![list(ui, 100, 5.0)?]),
                ))
            })?;
            assert_eq!(found.len(), expected, "at {scale}×: {found:?}");
        }
        Ok(())
    }

    #[test]
    fn the_report_follows_an_arrangement_that_changes_the_allocation() -> TestResult {
        // 100 five-pixel rows in a frame inside a 200-pixel clipping panel.
        // At 500 pixels the list was given its whole extent; at 495 it
        // scrolls itself, though overscan still mounts every row.
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
        let items = ui.signal(ListSource::new(Rows(100)));
        let frame = ui.signal(500.0f32);
        let built = items.clone();
        ui.mount_tree(Element::new(
            "panel",
            Container::new(Element::new(
                "column",
                Column::new([Element::new(
                    "dynamic",
                    Dynamic::new(frame.read_only(), move |_, height| {
                        let list = VirtualList::new(built.read_only(), 5.0).map_or_else(
                            |_| Element::new("list", Label::new("")),
                            |list| Element::new("list", list),
                        );
                        Element::new(
                            "frame",
                            Container::new(list)
                                .preferred_size(Size::new(300.0, *height).unwrap_or(Size::ZERO)),
                        )
                    }),
                )]),
            ))
            .preferred_size(Size::new(300.0, 200.0)?)
            .size_limits(SizeLimits::new(
                Size::ZERO,
                Constraints::new(Some(300.0), Some(200.0))?,
            )?),
        ))?;
        let settle = |ui: &mut Runtime, from: u64| -> TestResult {
            for step in from..from + 4 {
                ui.flush(Duration::from_millis(step))?;
            }
            Ok(())
        };
        settle(&mut ui, 0)?;
        assert_eq!(unbounded(&ui).len(), 1);
        frame.set(495.0)?;
        settle(&mut ui, 10)?;
        assert_eq!(mounted_rows(&ui), 100, "overscan mounts every row");
        assert!(unbounded(&ui).is_empty(), "{:?}", unbounded(&ui));
        frame.set(500.0)?;
        settle(&mut ui, 20)?;
        assert_eq!(unbounded(&ui).len(), 1);
        Ok(())
    }

    #[test]
    fn a_tall_list_that_scrolls_itself_is_not_reported() -> TestResult {
        // 1 200 pixels of a 100 000-pixel list in a 600-pixel window: it
        // scrolls itself, so it mounts only what its viewport covers.
        let found = reported(|ui| {
            Ok(Element::new(
                "page",
                Column::new([Element::new(
                    "frame",
                    Container::new(list(ui, 5000, 20.0)?).preferred_size(Size::new(300.0, 1200.0)?),
                )]),
            ))
        })?;
        assert!(found.is_empty(), "{found:?}");
        Ok(())
    }

    #[test]
    fn a_list_mostly_hidden_by_a_clipping_panel_is_reported() -> TestResult {
        // 100 rows of 5 pixels: 500 pixels, shorter than the window, all
        // mounted, in a 200-pixel panel that clips the rest.
        let found = reported(|ui| {
            Ok(Element::new(
                "panel",
                Container::new(Element::new("column", Column::new([list(ui, 100, 5.0)?])))
                    .preferred_size(Size::new(300.0, 200.0)?)
                    .size_limits(SizeLimits::new(
                        Size::ZERO,
                        Constraints::new(Some(300.0), Some(200.0))?,
                    )?),
            ))
        })?;
        assert_eq!(found.len(), 1, "{found:?}");
        Ok(())
    }
}
