//! A fixed-height wrapper written outside the library, through the public
//! component API only: it forwards its child's measurement, reports
//! `ValidationHints::fixed_height` and refreshes its child's measurement in
//! `arrange`, as the table's own wrappers do.
use pert_ui::component::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;

/// Text that wraps to two lines at [`NARROW`] and fits one line at [`WIDE`].
const TEXT: &str = "alpha beta gamma";
const NARROW: f32 = 60.0;
const WIDE: f32 = 400.0;

/// Gives its one child exactly its own measurement and size, and asks that
/// the child fit the height it is given.
struct Band(Option<Element>);
impl Component for Band {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, space)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.refresh_forwarded(0)?;
        cx.place(0, Rect::from_size(size))
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.validation.fixed_height = true;
        details
    }
}

/// Measures its one child at the width `width` holds and a 20 px height, and
/// places it there, clipped.
struct Slot {
    child: Option<Element>,
    width: ReadSignal<f32>,
    at: f32,
}
impl Component for Slot {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.at = cx.read(&self.width)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(WIDE, 40.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        let measured = cx.measure(0, Constraints::new(Some(self.at), Some(20.0))?)?;
        let height = measured.height().min(20.0);
        let rect = Rect::new(0.0, 0.0, self.at, height)?;
        cx.place_clipped(0, rect, rect)
    }
}

/// Whether development validation reports the band's text as taller than
/// the band, after each width in turn.
fn reported_through(widths: [f32; 3]) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(WIDE, 300.0, 1.0)?)?;
    let width = ui.signal(WIDE);
    ui.mount_tree(Element::new(
        "slot",
        Slot {
            child: Some(Element::new(
                "band",
                Band(Some(Element::new("text", Label::new(TEXT)))),
            )),
            width: width.read_only(),
            at: WIDE,
        },
    ))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let mut seen = Vec::new();
    for at in widths {
        width.set(at)?;
        driver.settle()?;
        seen.push(
            driver
                .runtime()
                .validation_report()
                .issues
                .iter()
                .any(|issue| issue.code == ValidationCode::ExceedsFixedHeight),
        );
    }
    Ok(seen)
}

#[cfg(debug_assertions)]
#[test]
fn an_external_forwarding_wrapper_reports_its_childs_current_height_through_a_width_cycle()
-> Result<(), Box<dyn std::error::Error>> {
    // The band's last measurement in each cycle comes from the runtime's
    // retained results, which would leave the text's from the other width.
    assert_eq!(
        reported_through([WIDE, NARROW, WIDE])?,
        [false, true, false]
    );
    assert_eq!(
        reported_through([NARROW, WIDE, NARROW])?,
        [true, false, true]
    );
    Ok(())
}
