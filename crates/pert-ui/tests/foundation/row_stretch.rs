//! A row with `Align::Stretch` fills the height it is allocated, while its
//! measured height, the default alignments and child limits stay as they were.
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

fn no_gap() -> Result<Length, GeometryError> {
    Length::new(0.0)
}

fn mount(
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
    key: &str,
) -> Result<[f32; 4], Box<dyn std::error::Error>> {
    let id = ui.find_descendant(root, key)?.ok_or("mounted")?;
    let rect = ui.bounds(&id)?.ok_or("laid out")?;
    Ok([rect.x(), rect.y(), rect.width(), rect.height()])
}

/// A column of a 100-pixel header and a row that grows into the rest.
fn grown(row: Row) -> Result<Element, GeometryError> {
    Ok(Column::new([
        fixed("header", 50.0, 100.0),
        row.gap(no_gap()?).key("row").with_grow(1),
    ])
    .gap(no_gap()?)
    .key("column"))
}

#[test]
fn a_grown_row_stretches_its_children_to_its_height() -> TestResult {
    let row =
        Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)]).align(Align::Stretch);
    let (ui, root) = mount(200.0, 400.0, grown(row)?)?;
    assert_eq!(bounds(&ui, &root, "row")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "tall")?, [0.0, 100.0, 10.0, 300.0]);
    assert_eq!(bounds(&ui, &root, "short")?, [10.0, 100.0, 10.0, 300.0]);
    Ok(())
}

#[test]
fn a_new_allocation_alone_restretches_the_children() -> TestResult {
    // Resizing the window changes only the row's allocation: its measured
    // height, its children and its description stay the same.
    let row =
        Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)]).align(Align::Stretch);
    let (mut ui, root) = mount(200.0, 400.0, grown(row)?)?;
    assert_eq!(bounds(&ui, &root, "short")?[3], 300.0);
    ui.set_viewport(Viewport::new(200.0, 600.0, 1.0)?)?;
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(bounds(&ui, &root, "short")?[3], 500.0);
    Ok(())
}

#[test]
fn the_other_alignments_keep_placing_against_the_tallest_child() -> TestResult {
    for (align, y) in [
        (None, 115.0),
        (Some(Align::Start), 100.0),
        (Some(Align::End), 130.0),
    ] {
        let mut row = Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)]);
        if let Some(align) = align {
            row = row.align(align);
        }
        let (ui, root) = mount(200.0, 400.0, grown(row)?)?;
        assert_eq!(
            bounds(&ui, &root, "short")?,
            [10.0, y, 10.0, 20.0],
            "{align:?}"
        );
    }
    Ok(())
}

#[test]
fn a_row_given_only_its_natural_height_stretches_to_its_tallest_child() -> TestResult {
    // An ungrown row in a column is allocated its measured height.
    let (ui, root) = mount(
        200.0,
        400.0,
        Column::new([
            Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)])
                .align(Align::Stretch)
                .key("row"),
        ])
        .key("column"),
    )?;
    assert_eq!(bounds(&ui, &root, "row")?[3], 50.0);
    assert_eq!(bounds(&ui, &root, "short")?[3], 50.0);
    Ok(())
}

#[test]
fn stretching_respects_child_maximum_and_minimum_heights() -> TestResult {
    let capped = Container::new(fixed("leaf", 10.0, 10.0))
        .size_limits(SizeLimits::new(
            Size::ZERO,
            Constraints::new(None, Some(120.0))?,
        )?)
        .key("capped");
    let row = Row::new([capped, fixed("free", 10.0, 10.0)]).align(Align::Stretch);
    let (ui, root) = mount(200.0, 400.0, grown(row)?)?;
    assert_eq!(bounds(&ui, &root, "capped")?[3], 120.0);
    assert_eq!(bounds(&ui, &root, "free")?[3], 300.0);

    // A child whose minimum exceeds the row's allocation keeps its minimum.
    let floor = Container::new(fixed("inner", 10.0, 10.0))
        .size_limits(SizeLimits::at_least(Size::new(0.0, 60.0)?))
        .key("floor");
    let row = Row::new([floor]).align(Align::Stretch).key("row");
    let clipped = Container::new(row)
        .size_limits(SizeLimits::new(
            Size::ZERO,
            Constraints::new(None, Some(30.0))?,
        )?)
        .key("clip");
    let (ui, root) = mount(200.0, 400.0, Column::new([clipped]).key("column"))?;
    assert_eq!(bounds(&ui, &root, "floor")?[3], 60.0);
    Ok(())
}

#[test]
fn nested_rows_pass_the_allocated_height_down() -> TestResult {
    let inner = Row::new([fixed("leaf", 10.0, 20.0)])
        .align(Align::Stretch)
        .key("inner");
    let row = Row::new([inner, fixed("sibling", 10.0, 40.0)]).align(Align::Stretch);
    let (ui, root) = mount(200.0, 400.0, grown(row)?)?;
    assert_eq!(bounds(&ui, &root, "inner")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "leaf")?[3], 300.0);
    Ok(())
}

#[test]
fn a_vertical_divider_and_hidden_children_follow_the_stretched_height() -> TestResult {
    let row = Row::new([
        fixed("left", 10.0, 20.0),
        Divider::new().vertical().key("divider"),
        fixed("hidden", 10.0, 380.0),
    ])
    .align(Align::Stretch);
    let (mut ui, root) = mount(200.0, 400.0, grown(row)?)?;
    // The hidden child is still shown and makes the row taller than the space
    // the column has left. Since 0.3.25 a grown child of a bounded column
    // shrinks to the space left (it had its natural 380 before), and the
    // divider stretches to that.
    assert_eq!(bounds(&ui, &root, "row")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "divider")?[3], 300.0);
    // A hidden child neither sets the row's height nor takes the stretch.
    let hidden = ui.find_descendant(&root, "hidden")?.ok_or("mounted")?;
    ui.set_visible(&hidden, false)?;
    ui.flush(Duration::from_millis(1))?;
    assert_eq!(bounds(&ui, &root, "row")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "left")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "divider")?[3], 300.0);
    Ok(())
}

#[test]
fn in_a_scroll_area_a_stretched_row_fills_the_visible_height() -> TestResult {
    let row = Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)])
        .align(Align::Stretch)
        .key("row");
    let (ui, root) = mount(200.0, 300.0, ScrollArea::new(row).key("scroll"))?;
    let visible = ui.bounds(&root)?.ok_or("laid out")?.height();
    assert_eq!(bounds(&ui, &root, "short")?[3], visible);
    Ok(())
}

#[test]
fn a_new_description_switches_between_centering_and_stretching() -> TestResult {
    let describe = |align: Align| -> Result<Vec<Element>, GeometryError> {
        Ok(vec![
            fixed("header", 50.0, 100.0),
            Row::new([fixed("tall", 10.0, 50.0), fixed("short", 10.0, 20.0)])
                .align(align)
                .gap(no_gap()?)
                .key("row")
                .with_grow(1),
        ])
    };
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 400.0, 1.0)?)?;
    let root = ui.mount_tree(
        Column::new(describe(Align::Center)?)
            .gap(no_gap()?)
            .key("column"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, "short")?[3], 20.0);
    ui.reconcile_children(&root, describe(Align::Stretch)?)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, "short")?[3], 300.0);
    ui.reconcile_children(&root, describe(Align::Center)?)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(bounds(&ui, &root, "short")?, [10.0, 115.0, 10.0, 20.0]);
    Ok(())
}

#[test]
fn a_stretched_vertical_slider_and_meter_use_the_whole_height() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 400.0, 1.0)?)?;
    let (gain, _host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let (_endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(8)?));
    let source = ui.series_source(consumer, Delivery::Polled);
    let meter = LevelMeter::new(source, ParameterSpec::new("Level", 0.0, 1.0, 0.0)?)
        .axis(MeterAxis::Vertical);
    let row = Row::new([
        Element::new(
            "fader",
            Slider::new(gain.clone()).orientation(Orientation::Vertical),
        ),
        Element::new("meter", meter),
    ])
    .align(Align::Stretch);
    let root = ui.mount_tree(grown(row)?)?;
    // The first update subscribes the meter; the runtime then activates the source.
    while ui.next_deadline() == Some(Duration::ZERO) {
        ui.flush(Duration::ZERO)?;
    }
    assert_eq!(bounds(&ui, &root, "fader")?[3], 300.0);
    assert_eq!(bounds(&ui, &root, "meter")?[3], 300.0);
    // The track spans the stretched height: a press low in the strip, below
    // where the natural-height fader would end, sets a low value.
    let fader = bounds(&ui, &root, "fader")?;
    let (x, y) = (fader[0] + fader[2] / 2.0, fader[1] + 230.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(x, y)?,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    let preview = gain.snapshot().get()?.preview.ok_or("a gesture began")?;
    assert!(preview > 0.0 && preview < 0.3, "{preview}");
    ui.pointer_input(PointerEvent::new(
        Point::new(x, y)?,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::from_millis(1))?;
    // The meter's track is drawn over the whole height.
    let meter = bounds(&ui, &root, "meter")?;
    let scene = ui.paint()?;
    let tallest = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, .. }
                if rect.x() >= meter[0] && rect.x() < meter[0] + meter[2] =>
            {
                Some(rect.height())
            }
            _ => None,
        })
        .fold(0.0f32, f32::max);
    assert_eq!(tallest, 300.0);
    Ok(())
}
