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
