//! Automatic validation, marker lifecycle and the release collection boundary.
use pert_ui_runtime::testing::HostIntegration;
use pert_ui_runtime::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

#[derive(Clone, Default)]
struct Observed {
    details: Rc<RefCell<ComponentDetails>>,
    inspections: Rc<Cell<usize>>,
}
impl Component for Observed {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        self.inspections.set(self.inspections.get() + 1);
        self.details.borrow().clone()
    }
}
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Where a [`Place`] puts a child: the rectangle its shared cell holds, and
/// the clip it is placed with, if any.
type Placement = (Rc<Cell<Rect>>, Option<Rect>);

/// Places each visible child, by key, at its placement, so a fixture's
/// geometry is set by real layout rather than reported by the component. A
/// test moves a child by changing its cell and invalidating the container's
/// layout.
struct Place {
    children: Vec<Element>,
    rects: Vec<(&'static str, Placement)>,
}
impl Component for Place {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            let key = cx.measurement().key(index)?;
            let (_, (rect, clip)) = self
                .rects
                .iter()
                .find(|(placed, _)| *placed == key)
                .ok_or(LayoutError::InvalidChild)?;
            let rect = rect.get();
            match clip {
                Some(clip) => cx.place_clipped(index, rect, *clip)?,
                None => cx.place(index, rect)?,
            }
        }
        Ok(())
    }
}

#[test]
fn automatic_inspection_is_debug_only_and_idle_does_not_poll() -> TestResult {
    let mut runtime = Runtime::new();
    let component = Observed::default();
    component.details.borrow_mut().box_model.padding = Some([-1.0; 4]);
    runtime.mount_tree(Element::new("control", component.clone()))?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(
        component.inspections.get(),
        usize::from(cfg!(debug_assertions))
    );
    let report = runtime.validation_report();
    assert_eq!(report.enabled, cfg!(debug_assertions));
    assert_eq!(report.issues.len(), usize::from(cfg!(debug_assertions)));
    assert_eq!(report.markers.len(), report.issues.len());
    runtime.flush(Duration::from_secs(1))?;
    assert_eq!(
        component.inspections.get(),
        usize::from(cfg!(debug_assertions))
    );
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn overlap_and_shortcut_markers_follow_fixes_visibility_and_scope() -> TestResult {
    let mut runtime = Runtime::new();
    // Geometry is validated only in a window whose viewport has area.
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let first = Observed::default();
    let second = Observed::default();
    for component in [&first, &second] {
        component
            .details
            .borrow_mut()
            .validation
            .shortcuts
            .push(ShortcutBinding::new(KeyCode::KeyA, "action"));
    }
    // Both probes at the same rectangle, as peers under one container.
    let at = Rect::new(0.0, 0.0, 20.0, 20.0)?;
    let root = runtime.mount_tree(Element::new(
        "place",
        Place {
            children: vec![Element::new("a", first), Element::new("b", second.clone())],
            rects: vec![
                ("a", (Rc::new(Cell::new(at)), None)),
                ("b", (Rc::new(Cell::new(at)), None)),
            ],
        },
    ))?;
    runtime.flush(Duration::ZERO)?;
    let a = runtime.find(&root, &["a"])?.ok_or("a is mounted")?;
    let b = runtime.find(&root, &["b"])?.ok_or("b is mounted")?;
    let report = runtime.validation_report();
    assert_eq!(report.issues.len(), 2);
    assert_eq!(report.markers.len(), 2);
    assert!(
        report
            .markers
            .iter()
            .all(|m| m.bounds == Some([0.0, 0.0, 20.0, 20.0]))
    );
    {
        let mut details = second.details.borrow_mut();
        details.validation.overlap = OverlapPolicy::Allow;
        details.validation.shortcuts[0].scope = ShortcutScope::FocusedComponent;
    }
    runtime.invalidate(&b, Invalidation::Layout)?;
    runtime.flush(Duration::ZERO)?;
    assert!(runtime.validation_report().issues.is_empty());
    runtime.focus(&b)?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(
        runtime.validation_report().issues[0].code,
        ValidationCode::ShortcutConflict
    );
    // Hiding a child changes its parent's layout: the next flush lays it out
    // and leaves nothing scheduled.
    runtime.set_visible(&b, false)?;
    assert!(runtime.validation_report().markers.is_empty());
    assert_eq!(runtime.next_deadline(), Some(runtime.now()));
    runtime.flush(Duration::ZERO)?;
    assert_eq!(runtime.next_deadline(), None);
    assert!(runtime.validation_report().markers.is_empty());
    // So does removing one.
    runtime.unmount(&a)?;
    assert!(runtime.validation_report().markers.is_empty());
    assert_eq!(runtime.next_deadline(), Some(runtime.now()));
    runtime.flush(Duration::ZERO)?;
    assert_eq!(runtime.next_deadline(), None);
    assert!(runtime.validation_report().markers.is_empty());
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn a_refused_duplicate_root_is_marked_until_the_root_unmounts() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let a = runtime.mount_tree(Element::new("a", Observed::default()))?;
    runtime.flush(Duration::ZERO)?;
    assert!(matches!(
        runtime.mount_tree(Element::new("a", Observed::default())),
        Err(RuntimeError::DuplicateKey(key)) if key == "a"
    ));
    assert_eq!(
        runtime.validation_report().issues[0].code,
        ValidationCode::DuplicateKey
    );
    runtime.unmount(&a)?;
    assert!(runtime.validation_report().markers.is_empty());
    assert_eq!(runtime.next_deadline(), None);
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn clipping_is_valid_but_zero_sized_interactive_boxes_are_errors() -> TestResult {
    let mut runtime = Runtime::new();
    runtime.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let component = Observed::default();
    {
        let mut details = component.details.borrow_mut();
        details.box_model.margin = Some([-2.0; 4]);
        details.validation.interactive = true;
    }
    // Scrolled out of its clip: placed below the clipping boundary.
    let rect = Rc::new(Cell::new(Rect::new(0.0, 30.0, 20.0, 20.0)?));
    let root = runtime.mount_tree(Element::new(
        "place",
        Place {
            children: vec![Element::new("scrolled", component)],
            rects: vec![(
                "scrolled",
                (rect.clone(), Some(Rect::new(0.0, 0.0, 20.0, 20.0)?)),
            )],
        },
    ))?;
    runtime.flush(Duration::ZERO)?;
    assert!(runtime.validation_report().issues.is_empty());
    rect.set(Rect::new(0.0, 0.0, 0.0, 0.0)?);
    runtime.invalidate(&root, Invalidation::Layout)?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(
        runtime.validation_report().issues[0].code,
        ValidationCode::EmptyHitArea
    );
    Ok(())
}

/// A 200 × 20 parent that gives its child its own height, clipped: it
/// measures the child at the width `width` holds and places it at its own
/// rectangle, and reports `ValidationHints::fixed_height` while `fixed`
/// holds true.
struct Strip {
    child: Option<Element>,
    fixed: ReadSignal<bool>,
    width: ReadSignal<f32>,
    hint: bool,
    measure_at: f32,
}
impl Strip {
    fn element(key: &str, child: Element, fixed: &Signal<bool>, width: &Signal<f32>) -> Element {
        Element::new(
            key,
            Self {
                child: Some(child),
                fixed: fixed.read_only(),
                width: width.read_only(),
                hint: false,
                measure_at: 0.0,
            },
        )
    }
}
impl Component for Strip {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // The hint changes only what inspection reports; the width only
        // what the child is measured at.
        self.hint = cx.read_paint(&self.fixed)?;
        self.measure_at = cx.read(&self.width)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 20.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        cx.measure(0, Constraints::new(Some(self.measure_at), None)?)?;
        let own = Rect::from_size(size);
        cx.place_clipped(0, own, own)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.validation.fixed_height = self.hint;
        details
    }
}

/// A leaf that fills the offered width and fails to arrange while `fail`
/// holds true; a change of it lays the leaf out again.
struct Failing {
    fail: ReadSignal<bool>,
    failing: bool,
}
impl Failing {
    fn element(key: &str, fail: &Signal<bool>) -> Element {
        Element::new(
            key,
            Self {
                fail: fail.read_only(),
                failing: false,
            },
        )
    }
}
impl Component for Failing {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.failing = cx.read(&self.fail)?;
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
        if self.failing {
            Err(LayoutError::InvalidChild.into())
        } else {
            Ok(())
        }
    }
}

/// The `ExceedsFixedHeight` issues, as the keys of their components.
#[cfg(debug_assertions)]
fn exceeding(ui: &Runtime) -> Vec<Vec<String>> {
    let snapshot = ui.inspect();
    let key = |serial: &u64| {
        snapshot
            .components
            .iter()
            .find(|component| component.id == *serial)
            .map_or_else(String::new, |component| component.key.clone())
    };
    snapshot
        .validation
        .issues
        .iter()
        .filter(|issue| issue.code == ValidationCode::ExceedsFixedHeight)
        .map(|issue| issue.components.iter().map(key).collect())
        .collect()
}

/// Text that wraps to two lines at a narrow width and fits one at a wide one.
const TEXT: &str = "alpha beta gamma";
const NARROW: f32 = 60.0;
const WIDE: f32 = 400.0;

/// A mounted strip's runtime with the signals of its hint and its child's
/// measuring width.
type Mounted = (Runtime, Signal<bool>, Signal<f32>);

fn mounted_strip(child: Element, fixed: bool) -> Result<Mounted, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let hint = ui.signal(fixed);
    let width = ui.signal(WIDE);
    ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new("title", Label::new("Strip")),
            Strip::element("strip", child, &hint, &width),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok((ui, hint, width))
}

#[cfg(debug_assertions)]
#[test]
fn a_custom_fixed_height_parent_reports_a_child_laid_out_taller() -> TestResult {
    let button = || Element::new("button", Button::new("Go", |_| Ok(())));
    let (ui, _, _) = mounted_strip(button(), true)?;
    assert_eq!(exceeding(&ui), [["button", "strip"]]);
    let message = &ui.validation_report().issues[0].message;
    assert!(
        message.contains("measured 32 px tall in \"strip\"")
            && message.contains("a fixed 20 px height"),
        "{message}"
    );
    let (ui, _, _) = mounted_strip(button(), false)?;
    assert!(exceeding(&ui).is_empty());
    assert!(
        ui.validation_report().issues.is_empty(),
        "the clip permits the excess"
    );
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn the_fixed_height_hint_alone_revalidates() -> TestResult {
    let button = Element::new("button", Button::new("Go", |_| Ok(())));
    let (mut ui, hint, _) = mounted_strip(button, false)?;
    assert!(exceeding(&ui).is_empty());
    hint.set(true)?;
    ui.flush(ui.now())?;
    assert_eq!(exceeding(&ui), [["button", "strip"]]);
    hint.set(false)?;
    ui.flush(ui.now())?;
    assert!(exceeding(&ui).is_empty());
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn a_measurement_that_changes_alone_revalidates() -> TestResult {
    let label = Element::new("label", Label::new(TEXT));
    let (mut ui, _, width) = mounted_strip(label, true)?;
    let strip = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "label")
        .ok_or("the label")?;
    assert!(exceeding(&ui).is_empty(), "one line fits");
    // The child keeps its rectangle and is never arranged again; only what
    // its parent measured it at changes.
    for (at, reported) in [(NARROW, true), (WIDE, false), (NARROW, true)] {
        width.set(at)?;
        ui.flush(ui.now())?;
        assert_eq!(!exceeding(&ui).is_empty(), reported, "at {at}");
        let label = ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.key == "label")
            .ok_or("the label")?;
        assert_eq!(
            label.details.box_model.border_box,
            strip.details.box_model.border_box
        );
    }
    Ok(())
}

#[cfg(debug_assertions)]
#[test]
fn a_measurement_queued_before_a_move_is_recorded_only_in_the_window_that_lays_it_out() -> TestResult
{
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (outside, inside) = (ui.signal(false), ui.signal(false));
    let hint = ui.signal(true);
    let width = ui.signal(WIDE);
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            Element::new(
                "view",
                View::new(Element::new(
                    "content",
                    Column::new([
                        Strip::element(
                            "strip",
                            Element::new("label", Label::new(TEXT)),
                            &hint,
                            &width,
                        ),
                        Failing::element("inside", &inside),
                    ]),
                )),
            ),
            Failing::element("outside", &outside),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(exceeding(&ui).is_empty());
    let primary = ui.primary_window();
    let view = ui.find(&root, &["view"])?.ok_or("view")?;

    // Measured narrow in a primary layout that fails after it.
    width.set(NARROW)?;
    outside.set(true)?;
    assert!(ui.flush(ui.now()).is_err());
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("Strip")?)?;
    let detached = ui.commit_detach(prepared)?;
    // Measured again in the detached window, whose layout fails too.
    inside.set(true)?;
    assert!(ui.flush(ui.now()).is_err());
    ui.select_window(&primary)?;
    outside.set(false)?;
    let before = exceeding(&ui);
    ui.flush(ui.now())?;
    assert_eq!(
        exceeding(&ui),
        before,
        "the primary window commits nothing it moved"
    );

    ui.redock(&detached)?;
    inside.set(false)?;
    width.set(WIDE)?;
    ui.flush(ui.now())?;
    assert!(exceeding(&ui).is_empty());
    for (at, reported) in [(NARROW, true), (WIDE, false), (NARROW, true)] {
        width.set(at)?;
        ui.flush(ui.now())?;
        assert_eq!(!exceeding(&ui).is_empty(), reported, "at {at}");
    }
    Ok(())
}
