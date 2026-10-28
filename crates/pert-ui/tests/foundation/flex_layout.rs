//! Flex layout of rows and columns through the facade: growth weights share
//! spare main-axis space up to each child's maximum, `Justify` places what is
//! left, `Align` places children across, and the defaults keep natural sizes;
//! a bounded row or column takes a shortfall from its grown children, and a
//! row measures a child it shrank again at its final width.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
/// A mounted tree, its root and a probe's shared cell.
type Probed<T> = Result<(Runtime, WidgetId, Rc<Cell<T>>), Box<dyn std::error::Error>>;

/// A leaf with a fixed natural size.
struct Fixed(f32, f32);
impl Component for Fixed {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.0, self.1)?)
    }
}

fn fixed(key: &str, width: f32, height: f32) -> Element {
    Fixed(width, height).key(key)
}

/// Mounts `root` in a window of `width` × `height` and returns the bounds of
/// each child of the root, by key.
fn layout(
    width: f32,
    height: f32,
    root: Element,
) -> Result<(Runtime, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
    let id = ui.mount_tree(root)?;
    ui.flush(Duration::ZERO)?;
    Ok((ui, id))
}

fn bounds(
    ui: &Runtime,
    root: &WidgetId,
    path: &[&str],
) -> Result<[f32; 4], Box<dyn std::error::Error>> {
    let id = ui.find(root, path)?.ok_or("mounted")?;
    let rect = ui.bounds(&id)?.ok_or("laid out")?;
    Ok([rect.x(), rect.y(), rect.width(), rect.height()])
}

fn no_gap() -> Result<Length, GeometryError> {
    Length::new(0.0)
}

#[test]
fn growing_children_share_the_spare_height_by_weight() -> TestResult {
    let (ui, root) = layout(
        100.0,
        400.0,
        Column::new([
            fixed("top", 50.0, 100.0),
            fixed("one", 50.0, 0.0).with_grow(1),
            fixed("two", 50.0, 0.0).with_grow(2),
        ])
        .gap(no_gap()?)
        .key("column"),
    )?;
    assert_eq!(bounds(&ui, &root, &["top"])?, [0.0, 0.0, 50.0, 100.0]);
    assert_eq!(bounds(&ui, &root, &["one"])?, [0.0, 100.0, 50.0, 100.0]);
    assert_eq!(bounds(&ui, &root, &["two"])?, [0.0, 200.0, 50.0, 200.0]);
    Ok(())
}

#[test]
fn a_child_that_reaches_its_maximum_leaves_the_rest_to_the_others() -> TestResult {
    let capped = Container::new(fixed("leaf", 10.0, 10.0))
        .size_limits(SizeLimits::new(
            Size::ZERO,
            Constraints::new(Some(40.0), None)?,
        )?)
        .key("capped")
        .with_grow(1);
    let (ui, root) = layout(
        300.0,
        50.0,
        Row::new([capped, fixed("free", 0.0, 10.0).with_grow(1)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    assert_eq!(bounds(&ui, &root, &["capped"])?[2], 40.0);
    let free = bounds(&ui, &root, &["free"])?;
    assert_eq!((free[0], free[2]), (40.0, 260.0));
    Ok(())
}

#[test]
fn in_a_scroll_area_growth_fills_only_the_visible_space_the_content_leaves() -> TestResult {
    let column = |top: f32| -> Result<Element, GeometryError> {
        Ok(ScrollArea::new(
            Column::new([fixed("a", 50.0, top), fixed("b", 50.0, 20.0).with_grow(1)])
                .gap(no_gap()?)
                .key("column"),
        )
        .key("scroll"))
    };
    // Shorter content: the column is given the visible height, and the
    // growing child takes what the content leaves of it.
    let (ui, root) = layout(200.0, 300.0, column(20.0)?)?;
    assert_eq!(bounds(&ui, &root, &["column", "b"])?[3], 280.0);
    // Taller content scrolls, and there is no spare space to grow into.
    let (ui, root) = layout(200.0, 300.0, column(400.0)?)?;
    assert_eq!(bounds(&ui, &root, &["column", "b"])?[3], 20.0);
    Ok(())
}

#[test]
fn justify_places_the_space_no_child_takes() -> TestResult {
    let row = |justify| -> Result<Element, GeometryError> {
        Ok(Row::new([fixed("a", 20.0, 10.0), fixed("b", 20.0, 10.0)])
            .gap(no_gap()?)
            .justify(justify)
            .key("row"))
    };
    let x = |justify| -> Result<(f32, f32), Box<dyn std::error::Error>> {
        let (ui, root) = layout(100.0, 50.0, row(justify)?)?;
        Ok((
            bounds(&ui, &root, &["a"])?[0],
            bounds(&ui, &root, &["b"])?[0],
        ))
    };
    assert_eq!(x(Justify::Start)?, (0.0, 20.0));
    assert_eq!(x(Justify::Center)?, (30.0, 50.0));
    assert_eq!(x(Justify::End)?, (60.0, 80.0));
    assert_eq!(x(Justify::SpaceBetween)?, (0.0, 80.0));
    Ok(())
}

#[test]
fn a_spacer_pushes_the_following_children_to_the_far_end() -> TestResult {
    let (ui, root) = layout(
        200.0,
        50.0,
        Row::new([
            fixed("left", 30.0, 10.0),
            Element::spacer("gap"),
            fixed("right", 30.0, 10.0),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    assert_eq!(bounds(&ui, &root, &["right"])?[0], 170.0);
    Ok(())
}

#[test]
fn align_places_children_across_a_column_and_a_row() -> TestResult {
    let column = |align| -> Result<Element, GeometryError> {
        Ok(Column::new([fixed("child", 20.0, 10.0)])
            .align(align)
            .key("column"))
    };
    let across = |align| -> Result<[f32; 4], Box<dyn std::error::Error>> {
        let (ui, root) = layout(100.0, 50.0, column(align)?)?;
        bounds(&ui, &root, &["child"])
    };
    assert_eq!(across(Align::Start)?[0], 0.0);
    assert_eq!(across(Align::Center)?[0], 40.0);
    assert_eq!(across(Align::End)?[0], 80.0);
    assert_eq!(across(Align::Stretch)?[2], 100.0);
    // A row aligns within its tallest child, centered by default.
    let row = |align: Option<Align>| -> Result<[f32; 4], Box<dyn std::error::Error>> {
        let mut row = Row::new([fixed("tall", 10.0, 40.0), fixed("short", 10.0, 10.0)]);
        if let Some(align) = align {
            row = row.align(align);
        }
        let (ui, root) = layout(100.0, 200.0, row.key("row"))?;
        bounds(&ui, &root, &["short"])
    };
    assert_eq!(row(None)?[1], 15.0);
    assert_eq!(row(Some(Align::Start))?[1], 0.0);
    assert_eq!(row(Some(Align::End))?[1], 30.0);
    // Stretching fills the height the row is given, here the whole window.
    assert_eq!(row(Some(Align::Stretch))?[3], 200.0);
    Ok(())
}

#[test]
fn a_hidden_child_takes_no_share() -> TestResult {
    let (mut ui, root) = layout(
        100.0,
        100.0,
        Column::new([
            fixed("a", 10.0, 0.0).with_grow(1),
            fixed("b", 10.0, 0.0).with_grow(1),
        ])
        .gap(no_gap()?)
        .key("column"),
    )?;
    assert_eq!(bounds(&ui, &root, &["a"])?[3], 50.0);
    let b = support::key_path(&ui, &root, &["b"]).ok_or("mounted")?;
    ui.set_visible(&b, false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, &["a"])?[3], 100.0);
    Ok(())
}

#[test]
fn a_new_description_changes_the_weights_and_choices() -> TestResult {
    let describe = |weight: u16, justify| -> Result<Vec<Element>, GeometryError> {
        Ok(vec![
            Row::new([
                fixed("a", 10.0, 10.0).with_grow(weight),
                fixed("b", 10.0, 10.0),
            ])
            .gap(no_gap()?)
            .justify(justify)
            .key("row"),
        ])
    };
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 50.0, 1.0)?)?;
    let root = ui.mount_tree(Column::new(describe(1, Justify::Start)?).key("root"))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, &["row", "a"])?[2], 90.0);
    ui.reconcile_children(&root, describe(0, Justify::End)?)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, &["row", "a"])?, [80.0, 0.0, 10.0, 10.0]);
    Ok(())
}

#[test]
fn wrap_ignores_growth() -> TestResult {
    let (ui, root) = layout(
        200.0,
        50.0,
        Wrap::new([fixed("a", 20.0, 10.0).with_grow(1)]).key("wrap"),
    )?;
    assert_eq!(bounds(&ui, &root, &["a"])?[2], 20.0);
    Ok(())
}

struct Rows;
impl ListItems for Rows {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Label::new(format!("Row {index}")).key("row"))
    }
}

#[test]
fn a_growing_list_fills_the_rest_of_a_row() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Rows));
    let root = ui.mount_tree(
        Row::new([
            fixed("sidebar", 100.0, 200.0),
            VirtualList::new(&items, 24.0)?.key("list").with_grow(1),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, &["list"])?[2], 200.0);
    Ok(())
}

#[test]
fn a_huge_spare_space_is_shared_without_overflow() -> TestResult {
    let (ui, root) = layout(
        1.0e34,
        50.0,
        Row::new([
            fixed("a", 0.0, 10.0).with_grow(u16::MAX),
            fixed("b", 0.0, 10.0).with_grow(u16::MAX),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    let (a, b) = (bounds(&ui, &root, &["a"])?, bounds(&ui, &root, &["b"])?);
    assert!(a[2].is_finite() && a[2] > 0.0);
    assert_eq!(a[2], b[2]);
    Ok(())
}

#[test]
fn hard_minimums_that_do_not_fit_leave_no_spare_space() -> TestResult {
    let at_least = Container::new(fixed("leaf", 0.0, 10.0))
        .size_limits(SizeLimits::at_least(Size::new(50.0, 0.0)?))
        .key("minimum")
        .with_grow(1);
    let (ui, root) = layout(
        100.0,
        50.0,
        Row::new([
            fixed("first", 80.0, 10.0),
            at_least,
            fixed("last", 10.0, 10.0),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    let minimum = bounds(&ui, &root, &["minimum"])?;
    assert_eq!(
        (minimum[0], minimum[2]),
        (80.0, 50.0),
        "the minimum survives"
    );
    assert_eq!(
        bounds(&ui, &root, &["last"])?[0],
        130.0,
        "later siblings keep their place"
    );
    Ok(())
}

/// A leaf with a natural size and a minimum width that counts its
/// measurements; it reports its natural size at every constraint.
struct Counting {
    natural: (f32, f32),
    minimum: f32,
    count: Rc<Cell<u32>>,
}
impl Component for Counting {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.count.set(self.count.get() + 1);
        Ok(Size::new(self.natural.0, self.natural.1)?)
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::at_least(Size::new(self.minimum, 0.0).unwrap_or(Size::ZERO))
    }
}
fn counting(key: &str, width: f32, height: f32) -> (Element, Rc<Cell<u32>>) {
    let count = Rc::new(Cell::new(0));
    let element = Counting {
        natural: (width, height),
        minimum: 0.0,
        count: count.clone(),
    }
    .key(key);
    (element, count)
}

/// Text of a fixed area that wraps: 300 wide and `area / 300` high without a
/// width, and `area / w` high at a width `w` up to 300.
struct Wrapping(f32);
impl Component for Wrapping {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let width = available
            .width()
            .map_or(300.0, |width| width.clamp(1.0, 300.0));
        Ok(Size::new(width, self.0 / width)?)
    }
}

/// A row-height filler whose width depends on whether it is offered a height.
struct Filler {
    unbounded: (f32, f32),
    filled: f32,
    minimum: f32,
}
impl Component for Filler {
    fn fills_row_height(&self) -> bool {
        true
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(match available.height() {
            None => Size::new(self.unbounded.0, self.unbounded.1)?,
            Some(height) => Size::new(self.filled, height)?,
        })
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::at_least(Size::new(self.minimum, 0.0).unwrap_or(Size::ZERO))
    }
}
/// 100 × 10 without a height, 300 wide with one.
fn wide_fill(key: &str) -> Element {
    Filler {
        unbounded: (100.0, 10.0),
        filled: 300.0,
        minimum: 0.0,
    }
    .key(key)
}

/// Measures its one child at a 250 px width without a height, records the
/// width the child reports and arranges it at 250 wide without clipping it.
struct Reporter {
    reported: Rc<Cell<f32>>,
    child: Vec<Element>,
}
impl Component for Reporter {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.child)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let child = cx.measure(0, Constraints::new(Some(250.0), None)?)?;
        self.reported.set(child.width());
        Ok(Size::new(250.0, child.height())?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(0.0, 0.0, 250.0, size.height())?)
    }
}
/// Mounts `row` in a [`Reporter`] inside a 290 px container with 20 px padding,
/// in a 400 × 100 window, and returns the width the row reported.
fn in_reporter(row: Element) -> Probed<f32> {
    let reported = Rc::new(Cell::new(0.0));
    let reporter = Element::new(
        "reporter",
        Reporter {
            reported: reported.clone(),
            child: vec![row],
        },
    );
    let (ui, root) = layout(
        400.0,
        100.0,
        Column::new([Container::new(reporter)
            .padding(Length::from(20))
            .preferred_size(Size::from([290, 60]))
            .key("frame")])
        .key("root"),
    )?;
    Ok((ui, root, reported))
}

fn bounds_of(
    ui: &Runtime,
    root: &WidgetId,
    key: &str,
) -> Result<[f32; 4], Box<dyn std::error::Error>> {
    let id = ui.find_descendant(root, key)?.ok_or(format!("no {key}"))?;
    let rect = ui.bounds(&id)?.ok_or("laid out")?;
    Ok([rect.x(), rect.y(), rect.width(), rect.height()])
}
#[cfg(debug_assertions)]
fn escapes(ui: &Runtime, root: &WidgetId, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let id = ui.find_descendant(root, key)?.ok_or(format!("no {key}"))?;
    let id = ui.inspection_id(&id)?;
    Ok(ui
        .validation_report()
        .issues
        .iter()
        .any(|issue| issue.code == ValidationCode::EscapesParent && issue.components.contains(&id)))
}

/// A table of one 167 px column whose cell is a growing bar beside its
/// percentage.
struct Progress;
impl TableItems for Progress {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "job".to_owned())
    }
    fn cell(&self, _: usize, _: &str) -> Option<Element> {
        Some(
            Row::new([
                ProgressBar::new(ProgressStatus::of(42, 100))
                    .key("bar")
                    .with_grow(1),
                Label::new("42%").key("percent"),
            ])
            .key("cell"),
        )
    }
}

#[test]
fn a_growing_bar_beside_its_percentage_fits_a_narrow_cell() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let controller = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("progress", "Progress").width(Length::from(167))],
        TableSource::new(Progress),
    )?;
    let root = ui.mount_tree(Element::new("table", Table::new(&controller, 28.0)?))?;
    for step in 0..3 {
        ui.flush(Duration::from_millis(step))?;
    }
    let (cell, bar, percent) = (
        bounds_of(&ui, &root, "cell")?,
        bounds_of(&ui, &root, "bar")?,
        bounds_of(&ui, &root, "percent")?,
    );
    // The cell's content is 167 less twice the 12 px control padding.
    assert_eq!(cell[2], 143.0);
    assert!(
        percent[0] + percent[2] <= cell[0] + cell[2],
        "{percent:?} in {cell:?}"
    );
    assert!(
        (bar[2] + 8.0 + percent[2] - cell[2]).abs() < 0.01,
        "{bar:?} {percent:?}"
    );
    #[cfg(debug_assertions)]
    {
        let issues = ui.validation_report().issues;
        assert!(
            !issues
                .iter()
                .any(|issue| issue.code == ValidationCode::EscapesParent),
            "{issues:?}"
        );
    }
    Ok(())
}

#[test]
fn a_shrunk_child_is_measured_again_at_its_final_width() -> TestResult {
    // 350 natural in 200: the text gives up 150 and wraps to twice its height.
    let row = |area: f32| -> Result<Element, GeometryError> {
        Ok(Row::new([
            Wrapping(area).key("text").with_grow(1),
            fixed("side", 50.0, 10.0),
        ])
        .gap(no_gap()?)
        .key("row"))
    };
    let (ui, root) = layout(200.0, 100.0, Column::new([row(3000.0)?]).key("column"))?;
    assert_eq!(bounds_of(&ui, &root, "text")?, [0.0, 0.0, 150.0, 20.0]);
    assert_eq!(bounds_of(&ui, &root, "row")?[3], 20.0);
    assert_eq!(bounds_of(&ui, &root, "side")?[1], 5.0);
    // The row is as tall as its tallest final child, past the offered 30 px.
    let (ui, root) = layout(200.0, 30.0, row(6000.0)?)?;
    assert_eq!(bounds_of(&ui, &root, "text")?, [0.0, 0.0, 150.0, 40.0]);
    assert_eq!(
        bounds_of(&ui, &root, "side")?[1],
        15.0,
        "centered against 40"
    );
    Ok(())
}

#[test]
fn a_fill_child_follows_the_height_a_shrunk_child_wraps_to() -> TestResult {
    let (ui, root) = layout(
        140.0,
        200.0,
        Column::new([Row::new([
            Wrapping(3000.0).key("text").with_grow(1),
            Divider::new().vertical().key("divider"),
            fixed("side", 50.0, 10.0),
        ])
        .gap(no_gap()?)
        .key("row")])
        .key("column"),
    )?;
    let (text, divider, row) = (
        bounds_of(&ui, &root, "text")?,
        bounds_of(&ui, &root, "divider")?,
        bounds_of(&ui, &root, "row")?,
    );
    // 375 natural in 140: the text is 65 wide and about 46 high.
    assert_eq!(text[2], 65.0);
    assert!(text[3] > 32.0, "{text:?}");
    assert_eq!(divider[3], row[3]);
    assert_eq!(row[3], text[3]);
    Ok(())
}

#[test]
fn a_growing_ellipsis_label_truncates_in_a_narrow_row() -> TestResult {
    let (ui, root) = layout(
        160.0,
        100.0,
        Column::new([Row::new([
            Label::new("A status message much longer than its row")
                .overflow(TextOverflow::Ellipsis)
                .key("message")
                .with_grow(1),
            Label::new("100%").key("percent"),
        ])
        .gap(no_gap()?)
        .key("row")])
        .key("column"),
    )?;
    let message = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "message")
        .ok_or("inspected")?;
    assert_eq!(
        message.details.properties.get("truncated"),
        Some(&InspectValue::Bool(true))
    );
    let percent = bounds_of(&ui, &root, "percent")?;
    assert!(percent[0] + percent[2] <= 160.0, "{percent:?}");
    Ok(())
}

#[test]
fn residual_overflow_stays_and_is_reported() -> TestResult {
    // A 200 px row inside a container's 20 px padding, which leaves room
    // inside the container's clip to see a child leave the row.
    let framed = |row: Element| {
        Column::new([Container::new(row)
            .padding(Length::from(20))
            .preferred_size(Size::from([240, 60]))
            .key("frame")])
        .key("root")
    };
    // Children without a weight never give up width.
    let (ui, root) = layout(
        400.0,
        100.0,
        framed(
            Row::new([
                fixed("a", 150.0, 10.0),
                fixed("b", 100.0, 10.0),
                fixed("c", 40.0, 10.0).with_grow(1),
            ])
            .gap(no_gap()?)
            .key("row"),
        ),
    )?;
    let row = bounds_of(&ui, &root, "row")?;
    assert_eq!(row[2], 200.0);
    assert_eq!(bounds_of(&ui, &root, "c")?[2], 0.0);
    let b = bounds_of(&ui, &root, "b")?;
    assert_eq!(b[0] + b[2] - row[0], 250.0);
    #[cfg(debug_assertions)]
    assert!(escapes(&ui, &root, "b")?, "{:?}", ui.validation_report());
    // A grown child stops at its minimum, and the child after it overflows.
    let minimum = Counting {
        natural: (200.0, 10.0),
        minimum: 150.0,
        count: Rc::default(),
    };
    let (ui, root) = layout(
        400.0,
        100.0,
        framed(
            Row::new([
                fixed("a", 50.0, 10.0),
                minimum.key("b").with_grow(1),
                fixed("d", 20.0, 10.0),
            ])
            .gap(no_gap()?)
            .key("row"),
        ),
    )?;
    let row = bounds_of(&ui, &root, "row")?;
    assert_eq!(bounds_of(&ui, &root, "b")?[2], 150.0);
    assert_eq!(bounds_of(&ui, &root, "d")?[0] - row[0], 200.0);
    #[cfg(debug_assertions)]
    assert!(escapes(&ui, &root, "d")?, "{:?}", ui.validation_report());
    Ok(())
}

#[test]
fn nested_shrinking_rows_measure_each_child_once_per_width() -> TestResult {
    let (leaf, count) = counting("leaf", 500.0, 10.0);
    let stretched = |children: Vec<Element>, key: &str| -> Result<Element, GeometryError> {
        Ok(Row::new(children)
            .gap(no_gap()?)
            .align(Align::Stretch)
            .key(key))
    };
    let inner = stretched(vec![fixed("c", 50.0, 10.0), leaf.with_grow(1)], "r2")?;
    let middle = stretched(vec![fixed("b", 50.0, 10.0), inner.with_grow(1)], "r1")?;
    let (mut ui, root) = layout(
        300.0,
        40.0,
        stretched(vec![fixed("a", 100.0, 10.0), middle.with_grow(1)], "row")?,
    )?;
    assert_eq!(bounds_of(&ui, &root, "r1")?[2], 200.0);
    assert_eq!(bounds_of(&ui, &root, "r2")?[2], 150.0);
    assert_eq!(bounds_of(&ui, &root, "leaf")?[2], 100.0);
    // Its natural width and its final one.
    assert_eq!(count.get(), 2);
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(count.get(), 2, "a settled row measures nothing");
    Ok(())
}

#[test]
fn a_row_that_fits_measures_its_growing_child_once() -> TestResult {
    let (child, count) = counting("grows", 50.0, 10.0);
    let (ui, root) = layout(
        200.0,
        40.0,
        Row::new([child.with_grow(1), fixed("fixed", 50.0, 10.0)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    assert_eq!(bounds_of(&ui, &root, "grows")?[2], 150.0);
    assert_eq!(count.get(), 1);
    Ok(())
}

/// A row-height filler 300 wide and 20 high without constraints, that counts
/// its measurements; with `child`, it measures that child at the constraints
/// it receives instead.
struct FillProbe {
    count: Rc<Cell<u32>>,
    composite: bool,
    child: Vec<Element>,
}
impl Component for FillProbe {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.child)
    }
    fn fills_row_height(&self) -> bool {
        true
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.count.set(self.count.get() + 1);
        if self.composite {
            return cx.measure(0, available);
        }
        Ok(Size::new(300.0, available.height().unwrap_or(20.0))?)
    }
}

#[test]
fn a_shrunk_fill_child_is_asked_for_two_constraints() -> TestResult {
    let count = Rc::new(Cell::new(0));
    let probe = FillProbe {
        count: count.clone(),
        composite: false,
        child: Vec::new(),
    };
    let (mut ui, root) = layout(
        200.0,
        40.0,
        Row::new([probe.key("probe").with_grow(1), fixed("fixed", 50.0, 10.0)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    let probe = bounds_of(&ui, &root, "probe")?;
    assert_eq!((probe[2], probe[3]), (150.0, 20.0));
    assert_eq!(count.get(), 2, "unbounded, then its fill measurement");
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(count.get(), 2);
    // A composite fill child asks its own child for what it is asked.
    let (child, child_count) = counting("inner", 300.0, 20.0);
    let composite = Element::new(
        "probe",
        FillProbe {
            count: Rc::default(),
            composite: true,
            child: vec![child],
        },
    );
    let (mut ui, _) = layout(
        200.0,
        40.0,
        Row::new([composite.with_grow(1), fixed("fixed", 50.0, 10.0)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    assert_eq!(child_count.get(), 2);
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(child_count.get(), 2);
    Ok(())
}

#[test]
fn a_row_in_a_column_reuses_its_childs_four_measurements() -> TestResult {
    let (child, count) = counting("grows", 300.0, 10.0);
    let row = |child: Element, justify| -> Result<Element, GeometryError> {
        Ok(Row::new([child.with_grow(1), fixed("fixed", 50.0, 10.0)])
            .gap(no_gap()?)
            .justify(justify)
            .key("row"))
    };
    let (mut ui, root) = layout(
        200.0,
        100.0,
        Column::new([row(child, Justify::Start)?]).key("column"),
    )?;
    // Measured with an unbounded height and arranged at 10: two widths at
    // two heights.
    assert_eq!(count.get(), 4);
    assert_eq!(bounds_of(&ui, &root, "grows")?[2], 150.0);
    // A new `Justify` invalidates the row's layout, not its children's.
    let (again, _) = counting("grows", 300.0, 10.0);
    ui.reconcile_children(&root, vec![row(again, Justify::End)?])?;
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(count.get(), 4);
    assert_eq!(bounds_of(&ui, &root, "grows")?[2], 150.0);
    Ok(())
}

/// Measures 4 × `ControlWidth` by 10 without a width, and its width by
/// `IconSize` with one.
struct MetricProbe;
impl Component for MetricProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(match available.width() {
            None => Size::new(4.0 * cx.metric(MetricRole::ControlWidth), 10.0)?,
            Some(width) => Size::new(width, cx.metric(MetricRole::IconSize))?,
        })
    }
}
fn control_width(ui: &Runtime, width: u16) -> TestResult {
    ui.theme().set_metrics(
        Density::Standard,
        ThemeMetrics::for_density(Density::Standard)
            .with_metric(MetricRole::ControlWidth, Length::from(width))?,
    )?;
    Ok(())
}

#[test]
fn a_metric_only_the_natural_measurement_read_still_invalidates() -> TestResult {
    let (mut ui, root) = layout(
        200.0,
        100.0,
        Column::new([Row::new([
            MetricProbe.key("probe").with_grow(1),
            fixed("q", 200.0, 10.0).with_grow(1),
        ])
        .gap(no_gap()?)
        .key("row")])
        .key("column"),
    )?;
    // 256 + 200 natural in 200: each gives up 128.
    assert_eq!(bounds_of(&ui, &root, "probe")?[2], 128.0);
    control_width(&ui, 72)?;
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(bounds_of(&ui, &root, "probe")?[2], 144.0);
    assert_eq!(bounds_of(&ui, &root, "q")?[2], 56.0);
    Ok(())
}

/// Measures `natural` without a width, reading `ControlWidth` (or the body
/// font, preparing text) to do so and counting those measurements, and its
/// width by 10 with one, reading nothing.
struct Natural {
    font: bool,
    unbounded: Rc<Cell<u32>>,
}
impl Component for Natural {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if let Some(width) = available.width() {
            return Ok(Size::new(width, 10.0)?);
        }
        self.unbounded.set(self.unbounded.get() + 1);
        let width = if self.font {
            cx.text("iiii WWWW", None, TextStyle::Body)?.size().width()
        } else {
            2.0 * cx.metric(MetricRole::ControlWidth)
        };
        Ok(Size::new(width, 10.0)?)
    }
}
/// A leaf as wide as its description says, invalidating layout when that
/// changes.
struct Sized(f32);
impl Component for Sized {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let changed = self.0 != next.0;
        self.0 = next.0;
        changed.then_some(Invalidation::Layout)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(self.0, 10.0)?)
    }
}
/// A 300 × 40 row of a growing column holding a [`Natural`] probe and a
/// growing [`Sized`] leaf, redescribed at four widths so the probe's
/// unbounded measurement is evicted while the column's natural size, computed
/// from it, stays cached.
fn evicted(font: bool) -> Probed<u32> {
    let unbounded = Rc::new(Cell::new(0));
    let describe = |width: f32| -> Result<Vec<Element>, GeometryError> {
        Ok(vec![
            Column::new([Natural {
                font,
                unbounded: unbounded.clone(),
            }
            .key("probe")])
            .key("column")
            .with_grow(1),
            Sized(width).key("sized").with_grow(1),
        ])
    };
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 40.0, 1.0)?)?;
    let root = ui.mount_tree(Row::new(describe(200.0)?).gap(no_gap()?).key("row"))?;
    ui.flush(Duration::ZERO)?;
    for (step, width) in [210.0, 220.0, 230.0].into_iter().enumerate() {
        ui.reconcile_children(&root, describe(width)?)?;
        ui.flush(Duration::from_millis(step as u64 + 1))?;
    }
    Ok((ui, root, unbounded))
}

#[test]
fn an_evicted_measurement_keeps_its_metric_dependency() -> TestResult {
    let (mut ui, root, _) = evicted(false)?;
    // 128 + 230 natural in 300: each gives up 29.
    assert_eq!(bounds_of(&ui, &root, "column")?[2], 99.0);
    control_width(&ui, 72)?;
    ui.flush(Duration::from_millis(10))?;
    // 144 + 230 natural: each gives up 37.
    assert_eq!(bounds_of(&ui, &root, "column")?[2], 107.0);
    Ok(())
}

#[test]
fn an_evicted_measurement_keeps_its_font_dependency() -> TestResult {
    let (mut ui, _, unbounded) = evicted(true)?;
    let before = unbounded.get();
    let fonts = ui.fonts();
    let mono = fonts.register(FontData::bundled(FontRole::Monospace))?;
    fonts.set_default(FontRole::Body, mono)?;
    ui.flush(Duration::from_millis(10))?;
    assert_eq!(
        unbounded.get(),
        before + 1,
        "measured again with the new font"
    );
    Ok(())
}

#[test]
fn a_divider_counts_with_its_fill_height() -> TestResult {
    let (ui, root) = layout(
        200.0,
        20.0,
        Row::new([
            Wrapping(1500.0).key("text").with_grow(1),
            Divider::new().vertical().key("divider"),
            fixed("side", 50.0, 10.0),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    // 375 natural in 200: the text gives up 175 and is 125 by 12; the
    // divider fills min(32, 20).
    assert_eq!(bounds_of(&ui, &root, "text")?, [0.0, 4.0, 125.0, 12.0]);
    assert_eq!(bounds_of(&ui, &root, "divider")?[3], 20.0);
    assert_eq!(bounds_of(&ui, &root, "side")?[1], 5.0);
    Ok(())
}

#[test]
fn a_fill_child_wider_at_its_fill_height_without_a_shortfall_is_laid_out_as_before() -> TestResult {
    let (child, count) = counting("grows", 50.0, 10.0);
    let (ui, root) = layout(
        250.0,
        40.0,
        Row::new([
            wide_fill("fill"),
            fixed("fixed", 50.0, 10.0),
            child.with_grow(1),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    assert_eq!(bounds_of(&ui, &root, "fill")?[2], 300.0);
    let grows = bounds_of(&ui, &root, "grows")?;
    assert_eq!((grows[0], grows[2]), (350.0, 50.0));
    assert_eq!(count.get(), 1);
    Ok(())
}

/// Measures its one [`Echo`] child at each width in `widths`, twice over,
/// and counts results that differ from `(w, 1000 / w)`.
struct Cycler {
    widths: Vec<f32>,
    wrong: Rc<Cell<u32>>,
    child: Vec<Element>,
}
impl Component for Cycler {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.child)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        for _ in 0..2 {
            for width in &self.widths {
                let size = cx.measure(0, Constraints::new(Some(*width), None)?)?;
                if size != Size::new(*width, 1000.0 / width)? {
                    self.wrong.set(self.wrong.get() + 1);
                }
            }
        }
        Ok(Size::new(100.0, 100.0)?)
    }
}
/// Answers `(w, 1000 / w)` at a width `w`, counting its measurements.
struct Echo(Rc<Cell<u32>>);
impl Component for Echo {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.0.set(self.0.get() + 1);
        let width = available.width().unwrap_or(1000.0);
        Ok(Size::new(width, 1000.0 / width)?)
    }
}

#[test]
fn more_than_four_constraints_are_measured_again_with_the_same_results() -> TestResult {
    for (widths, expected) in [
        (vec![10.0, 20.0, 30.0, 40.0], 4),
        (vec![10.0, 20.0, 30.0, 40.0, 50.0], 10),
    ] {
        let (count, wrong) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let cycler = Element::new(
            "cycler",
            Cycler {
                widths,
                wrong: wrong.clone(),
                child: vec![Element::new("echo", Echo(count.clone()))],
            },
        );
        let (ui, _) = layout(200.0, 200.0, Column::new([cycler]).key("column"))?;
        drop(ui);
        assert_eq!(count.get(), expected);
        assert_eq!(wrong.get(), 0, "every result matches its constraint");
    }
    Ok(())
}

#[test]
fn a_fill_child_wider_at_its_fill_height_counts_with_that_width_after_a_shortfall() -> TestResult {
    let (child, _) = counting("grows", 300.0, 10.0);
    let (ui, root, reported) = in_reporter(
        Row::new([wide_fill("fill"), child.with_grow(1)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    let (row, grows) = (
        bounds_of(&ui, &root, "row")?,
        bounds_of(&ui, &root, "grows")?,
    );
    // The natural 150 deficit is taken from the grower; the filler then
    // takes 300.
    assert_eq!((grows[0] - row[0], grows[2]), (300.0, 150.0));
    assert_eq!(bounds_of(&ui, &root, "fill")?[2], 300.0);
    assert_eq!(reported.get(), 450.0);
    #[cfg(debug_assertions)]
    assert!(escapes(&ui, &root, "fill")?, "{:?}", ui.validation_report());
    // A growing filler at its minimum gives nothing, and counts with its
    // width at its fill height, not its minimum.
    let (child, _) = counting("grows", 300.0, 10.0);
    let fill = Filler {
        unbounded: (100.0, 10.0),
        filled: 300.0,
        minimum: 100.0,
    };
    let (ui, root, reported) = in_reporter(
        Row::new([fill.key("fill").with_grow(1), child.with_grow(1)])
            .gap(no_gap()?)
            .key("row"),
    )?;
    assert_eq!(bounds_of(&ui, &root, "fill")?[2], 300.0);
    assert_eq!(bounds_of(&ui, &root, "grows")?[2], 150.0);
    assert_eq!(reported.get(), 450.0);
    Ok(())
}

#[test]
fn a_fill_child_narrower_at_its_fill_height_leaves_space_that_justify_places() -> TestResult {
    let (child, _) = counting("grows", 100.0, 10.0);
    let narrow = Filler {
        unbounded: (300.0, 10.0),
        filled: 100.0,
        minimum: 0.0,
    };
    let (ui, root, reported) = in_reporter(
        Row::new([narrow.key("fill"), child.with_grow(1)])
            .gap(no_gap()?)
            .justify(Justify::Center)
            .key("row"),
    )?;
    let (row, fill) = (
        bounds_of(&ui, &root, "row")?,
        bounds_of(&ui, &root, "fill")?,
    );
    // 400 natural in 250: the grower gives all its 100 and does not grow
    // back; the filler takes 100 and the 150 left over is centered.
    assert_eq!(bounds_of(&ui, &root, "grows")?[2], 0.0);
    assert_eq!((fill[0] - row[0], fill[2]), (75.0, 100.0));
    assert_eq!(reported.get(), 250.0);
    #[cfg(debug_assertions)]
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report()
    );
    Ok(())
}

#[test]
fn a_shrinking_rows_spare_space_is_its_final_widths_total_in_f64() -> TestResult {
    // A filler 200,000,000 wide by nature fills at 100,000,000; with three
    // 1 px siblings the final total is 100,000,003, so a 100,000,016 px row
    // centers 13 px of spare space. Summed in f32, the ones vanish.
    let filler = Filler {
        unbounded: (200_000_000.0, 10.0),
        filled: 100_000_000.0,
        minimum: 0.0,
    };
    let (ui, root) = layout(
        100_000_016.0,
        40.0,
        Row::new([
            filler.key("fill"),
            fixed("a", 1.0, 10.0),
            fixed("b", 1.0, 10.0),
            fixed("c", 1.0, 10.0),
            fixed("grows", 0.0, 10.0).with_grow(1),
        ])
        .gap(no_gap()?)
        .justify(Justify::Center)
        .key("row"),
    )?;
    assert_eq!(bounds_of(&ui, &root, "fill")?[0], 6.5);
    Ok(())
}

#[test]
fn a_shrinking_rows_reported_width_is_its_final_widths_total_in_f64() -> TestResult {
    // Five 1 px siblings of a filler at 100,000,000 and a grower with no
    // room: 100,000,005, which f32 holds as 100,000,008.
    let filler = Filler {
        unbounded: (200_000_000.0, 10.0),
        filled: 100_000_000.0,
        minimum: 0.0,
    };
    let mut children = vec![filler.key("fill")];
    children.extend((0..5).map(|index| fixed(&format!("one-{index}"), 1.0, 10.0)));
    children.push(fixed("grows", 0.0, 10.0).with_grow(1));
    let (_ui, _root, reported) = in_reporter(Row::new(children).gap(no_gap()?).key("row"))?;
    assert_eq!(reported.get(), 100_000_008.0);
    Ok(())
}

/// A leaf `width` wide that records every constraint it is measured at; as
/// a row-height filler it is as high as it is offered.
struct Recorder {
    width: f32,
    fill: bool,
    seen: Rc<std::cell::RefCell<Vec<Constraints>>>,
}
impl Component for Recorder {
    fn fills_row_height(&self) -> bool {
        self.fill
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.seen.borrow_mut().push(available);
        let height = if self.fill {
            available.height().unwrap_or(10.0)
        } else {
            10.0
        };
        Ok(Size::new(self.width, height)?)
    }
}

/// Lays out a 1099 px row of a 1000 px [`Recorder`] with weight 1 and a
/// 100 px child with weight 65,535: 1100 natural, so the recorder's share of
/// the 1 px deficit is 1/65,536, which does not change its width as f32.
fn rounded_away(fill: bool) -> TestResult {
    let seen = Rc::default();
    let recorder = Recorder {
        width: 1000.0,
        fill,
        seen: Rc::clone(&seen),
    };
    let (ui, root) = layout(
        1099.0,
        40.0,
        Row::new([
            recorder.key("first").with_grow(1),
            fixed("second", 100.0, 10.0).with_grow(u16::MAX),
        ])
        .gap(no_gap()?)
        .key("row"),
    )?;
    assert_eq!(bounds_of(&ui, &root, "first")?[2], 1000.0);
    // Unshrunk, it is never measured at a width.
    let seen = seen.borrow();
    assert!(
        seen.iter().all(|constraints| constraints.width().is_none()),
        "{seen:?}"
    );
    Ok(())
}

#[test]
fn a_share_that_rounds_away_leaves_a_child_unshrunk() -> TestResult {
    rounded_away(false)
}

#[test]
fn a_share_that_rounds_away_leaves_a_fill_child_unshrunk() -> TestResult {
    rounded_away(true)
}
