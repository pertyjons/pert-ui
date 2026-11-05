//! Fixed slots: a parent that places a child with
//! `ArrangeContext::place_fixed_slot` declares that the child must fit the
//! slot's height, and development validation reports a child that does not.
//! The built-in fixed-extent `VirtualList` and `Tree` place their rows this
//! way; an external composite does the same through the public API.
use pert_ui::component::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::{cell::Cell, rc::Rc};

/// Shorter than a button, taller than a line of body text.
const ROW: f32 = 24.0;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Whether the issue reports `key`'s component as taller than its slot.
fn too_tall(ui: &Runtime, key: &str) -> Result<bool, RuntimeError> {
    let id = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .map(|component| component.id);
    Ok(ui.validation_report().issues.iter().any(|issue| {
        issue.code == ValidationCode::ExceedsFixedHeight
            && id.is_some_and(|id| issue.components.first() == Some(&id))
    }))
}

/// Rows keyed `row-N`: a button for each `true`, a label otherwise.
struct Rows(Vec<bool>);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0.len()).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        let key = self.key(index)?;
        Some(if *self.0.get(index)? {
            Button::new("Open", |_| Ok(())).key(key).into()
        } else {
            Label::new("Note").key(key).into()
        })
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_fixed_extent_list_reports_each_row_taller_than_its_row_height() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let rows = ui.signal(ListSource::new(Rows(vec![true, false, true])));
    ui.mount_tree(Element::new(
        "list",
        VirtualList::new(rows.read_only(), ROW)?,
    ))?;
    Driver::new(&mut ui).settle()?;
    assert!(
        too_tall(&ui, "row-0")?,
        "{:?}",
        ui.validation_report().issues
    );
    assert!(!too_tall(&ui, "row-1")?, "a label fits the row");
    assert!(too_tall(&ui, "row-2")?);
    // Exactly one report per offending row, also after another settle.
    for _ in 0..2 {
        let rows: Vec<Vec<u64>> = ui
            .validation_report()
            .issues
            .iter()
            .filter(|issue| issue.code == ValidationCode::ExceedsFixedHeight)
            .map(|issue| issue.components.clone())
            .collect();
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_ne!(rows[0][0], rows[1][0]);
        Driver::new(&mut ui).settle()?;
    }
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn an_estimated_list_sizes_its_rows_and_reports_nothing() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let rows = ui.signal(ListSource::new(Rows(vec![true, false, true])));
    ui.mount_tree(Element::new(
        "list",
        VirtualList::estimated(rows.read_only(), ROW)?,
    ))?;
    Driver::new(&mut ui).settle()?;
    assert!(!too_tall(&ui, "row-0")?);
    assert!(!too_tall(&ui, "row-2")?);
    Ok(())
}

/// One root holding a button.
struct Actions;
impl TreeItems for Actions {
    fn roots(&self) -> Vec<String> {
        vec!["open".into()]
    }
    fn children(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn text(&self, key: &str) -> Option<String> {
        Some(key.to_owned())
    }
    fn build(&self, key: &str) -> Option<RowItem> {
        Some(Button::new("Open", |_| Ok(())).key(key).into())
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_tree_reports_a_row_taller_than_its_row_height() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let tree = TreeController::new(&ui, selection, TreeSource::new(Actions))?;
    ui.mount_tree(Element::new("tree", Tree::new(&tree, ROW)?))?;
    Driver::new(&mut ui).settle()?;
    assert!(
        ui.validation_report()
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::ExceedsFixedHeight),
        "{:?}",
        ui.validation_report().issues
    );
    Ok(())
}

/// Counts how often the runtime asks it for a measurement.
struct Counted {
    measured: Rc<Cell<usize>>,
}
impl Component for Counted {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.measured.set(self.measured.get() + 1);
        Ok(Size::new(100.0, 40.0)?)
    }
}

/// Places its one child in a 24 px band at the top, declaring the band a
/// fixed slot while `fixed` holds, and arranging again whenever `revision`
/// changes.
struct Band {
    child: Option<Element>,
    fixed: ReadSignal<bool>,
    revision: ReadSignal<u32>,
    declared: bool,
}
impl Component for Band {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.declared = cx.read(&self.fixed)?;
        cx.read(&self.revision)?;
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let band = Rect::new(0.0, 0.0, size.width(), ROW)?;
        if self.declared {
            cx.place_fixed_slot(0, band, band, Orientation::Vertical)
        } else {
            cx.place_clipped(0, band, band)
        }
    }
}

struct BandFixture {
    ui: Runtime,
    fixed: Signal<bool>,
    revision: Signal<u32>,
    measured: Rc<Cell<usize>>,
}

fn band() -> Result<BandFixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let fixed = ui.signal(true);
    let revision = ui.signal(0_u32);
    let measured = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "band",
        Band {
            child: Some(Element::new(
                "content",
                Counted {
                    measured: measured.clone(),
                },
            )),
            fixed: fixed.read_only(),
            revision: revision.read_only(),
            declared: false,
        },
    ))?;
    Driver::new(&mut ui).settle()?;
    Ok(BandFixture {
        ui,
        fixed,
        revision,
        measured,
    })
}

#[cfg(debug_assertions)]
#[test]
fn an_external_composite_declares_and_clears_a_fixed_slot() -> TestResult {
    let mut f = band()?;
    assert!(too_tall(&f.ui, "content")?, "40 px content in a 24 px slot");
    // The same rectangle placed without the declaration clears the report.
    f.fixed.set(false)?;
    Driver::new(&mut f.ui).settle()?;
    assert!(!too_tall(&f.ui, "content")?);
    f.fixed.set(true)?;
    Driver::new(&mut f.ui).settle()?;
    assert!(
        too_tall(&f.ui, "content")?,
        "declaring it again reports again"
    );
    Ok(())
}

#[test]
fn a_fixed_slot_measures_its_child_once_and_reuses_the_measurement() -> TestResult {
    let mut f = band()?;
    // The band's default measurement and the slot's, at different
    // constraints.
    let first = f.measured.get();
    assert!(first >= 1, "measured at the slot's width");
    // Arranging again at the same width asks for the same measurement, which
    // the runtime answers from the child's retained results.
    for revision in 1..4 {
        f.revision.set(revision)?;
        Driver::new(&mut f.ui).settle()?;
    }
    assert_eq!(
        f.measured.get(),
        first,
        "an unchanged child is not measured again"
    );
    // A different slot width is a different measurement.
    f.ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    Driver::new(&mut f.ui).settle()?;
    assert!(f.measured.get() > first);
    Ok(())
}

/// Whether any issue has `code`.
fn reports(ui: &Runtime, code: ValidationCode) -> bool {
    ui.validation_report()
        .issues
        .iter()
        .any(|issue| issue.code == code)
}

#[cfg(debug_assertions)]
#[test]
fn a_horizontal_list_reports_an_item_wider_than_its_extent() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 60.0, 1.0)?)?;
    // A button is wider than 20 px; a 200 px extent fits it.
    for (extent, reported) in [(20.0, true), (200.0, false)] {
        let rows = ui.signal(ListSource::new(Rows(vec![true])));
        let list = ui.mount_tree(Element::new(
            "list",
            VirtualList::new(rows.read_only(), extent)?.orientation(Orientation::Horizontal),
        ))?;
        Driver::new(&mut ui).settle()?;
        assert_eq!(
            reports(&ui, ValidationCode::ExceedsFixedWidth),
            reported,
            "extent {extent}: {:?}",
            ui.validation_report().issues
        );
        assert!(!reports(&ui, ValidationCode::ExceedsFixedHeight));
        ui.unmount(&list)?;
    }
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn a_fixed_slot_compares_in_window_pixels_at_any_scale() -> TestResult {
    for scale in [1.0, 1.5, 2.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, scale)?)?;
        // A 36 px row fits a padded label at every scale, a 24 px row never
        // fits a button.
        for (extent, button, reported) in [(36.0, false, false), (24.0, true, true)] {
            let rows = ui.signal(ListSource::new(Rows(vec![button])));
            let list = ui.mount_tree(Element::new(
                "list",
                VirtualList::new(rows.read_only(), extent)?,
            ))?;
            Driver::new(&mut ui).settle()?;
            assert_eq!(
                reports(&ui, ValidationCode::ExceedsFixedHeight),
                reported,
                "scale {scale}, extent {extent}: {:?}",
                ui.validation_report().issues
            );
            ui.unmount(&list)?;
            Driver::new(&mut ui).settle()?;
            assert!(
                !reports(&ui, ValidationCode::ExceedsFixedHeight),
                "unmounting removes the report"
            );
        }
    }
    Ok(())
}

/// Shows its one child at the scale `zoom` holds.
struct Zoom {
    child: Option<Element>,
    zoom: ReadSignal<f32>,
    view: ViewTransform,
}
impl Component for Zoom {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.view =
            ViewTransform::new(ViewScale::new(cx.read(&self.zoom)?)?, Point::new(0.0, 0.0)?);
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        self.view
    }
}

/// Always measures `height` tall.
struct Tall(f32);
impl Component for Tall {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, self.0)?)
    }
}

#[cfg(debug_assertions)]
#[test]
fn zooming_keeps_a_slot_in_the_scale_its_child_is_shown_at() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let zoom = ui.signal(1.0_f32);
    let fixed = ui.signal(true);
    let revision = ui.signal(0_u32);
    ui.mount_tree(Element::new(
        "zoom",
        Zoom {
            child: Some(Element::new(
                "band",
                Band {
                    // 22 px content in the band's 24 px slot: a slot left at
                    // an earlier, smaller scale would be exceeded at once.
                    child: Some(Element::new("content", Tall(22.0))),
                    fixed: fixed.read_only(),
                    revision: revision.read_only(),
                    declared: false,
                },
            )),
            zoom: zoom.read_only(),
            view: ViewTransform::IDENTITY,
        },
    ))?;
    Driver::new(&mut ui).settle()?;
    // Steps within the continuous-zoom interval reuse the band's arrangement
    // while the scale its child is shown at changes.
    for step in [1.25_f32, 1.5, 1.75, 2.0, 1.5, 1.0] {
        zoom.set(step)?;
        Driver::new(&mut ui).settle()?;
        assert!(
            !reports(&ui, ValidationCode::ExceedsFixedHeight),
            "zoom {step}: {:?}",
            ui.validation_report().issues
        );
    }
    Ok(())
}

/// Measures 40 px tall, but fails the slot's measurement (an unbounded
/// height) while `fail` holds; a bounded measurement always succeeds.
struct Fallible {
    fail: Rc<Cell<bool>>,
}
impl Component for Fallible {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        if self.fail.get() && space.height().is_none() {
            return Err(pert_ui::LayoutError::InvalidChild.into());
        }
        Ok(Size::new(100.0, 40.0)?)
    }
}

#[test]
fn a_failing_slot_measurement_fails_the_layout_and_adopts_nothing() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let fixed = ui.signal(true);
    let revision = ui.signal(0_u32);
    let fail = Rc::new(Cell::new(false));
    ui.mount_tree(Element::new(
        "band",
        Band {
            child: Some(Element::new("content", Fallible { fail: fail.clone() })),
            fixed: fixed.read_only(),
            revision: revision.read_only(),
            declared: false,
        },
    ))?;
    Driver::new(&mut ui).settle()?;
    #[cfg(debug_assertions)]
    assert!(too_tall(&ui, "content")?, "40 px content in the 24 px slot");
    // A new width makes the slot measure again (its retained result is for
    // the old width), and only that measurement fails: the layout fails, and
    // the slot the last successful layout committed stays.
    fail.set(true);
    ui.set_viewport(Viewport::new(250.0, 200.0, 1.0)?)?;
    assert!(
        ui.flush(ui.now()).is_err(),
        "the slot measurement's error fails the flush"
    );
    // Validation waits for a successful layout: nothing from the failed pass
    // is reported or adopted.
    #[cfg(debug_assertions)]
    {
        let report = ui.validation_report();
        assert!(report.pending && report.issues.is_empty(), "{report:?}");
    }
    fail.set(false);
    revision.set(2)?;
    Driver::new(&mut ui).settle()?;
    #[cfg(debug_assertions)]
    assert!(too_tall(&ui, "content")?);
    Ok(())
}
