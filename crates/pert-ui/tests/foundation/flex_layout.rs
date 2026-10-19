//! Flex layout of rows and columns through the facade: growth weights share
//! spare main-axis space up to each child's maximum, `Justify` places what is
//! left, `Align` places children across, and the defaults keep natural sizes.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
    let b = ui.find_path(&root, &["b"])?.ok_or("mounted")?;
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
