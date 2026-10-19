//! Automatic validation, marker lifecycle and the release collection boundary.
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

#[test]
fn automatic_inspection_is_debug_only_and_idle_does_not_poll() -> TestResult {
    let mut runtime = Runtime::new();
    let component = Observed::default();
    component.details.borrow_mut().box_model.padding = Some([-1.0; 4]);
    runtime.mount("control", component.clone())?;
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
        let mut details = component.details.borrow_mut();
        details.box_model.border_box = Some([0.0, 0.0, 20.0, 20.0]);
        details
            .validation
            .shortcuts
            .push(ShortcutBinding::new(KeyCode::KeyA, "action"));
    }
    let a = runtime.mount("a", first)?;
    let b = runtime.mount("b", second.clone())?;
    runtime.flush(Duration::ZERO)?;
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
    runtime.set_visible(&b, false)?;
    assert!(runtime.validation_report().markers.is_empty());
    assert_eq!(runtime.next_deadline(), None);
    assert!(matches!(
        runtime.mount("a", Observed::default()),
        Err(RuntimeError::DuplicateKey)
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
        details.box_model.border_box = Some([0.0, 30.0, 20.0, 20.0]);
        details.box_model.margin = Some([-2.0; 4]);
        details.validation.clip = Some([0.0, 0.0, 20.0, 20.0]);
        details.validation.interactive = true;
    }
    let id = runtime.mount("scrolled", component.clone())?;
    runtime.flush(Duration::ZERO)?;
    assert!(runtime.validation_report().issues.is_empty());
    component.details.borrow_mut().box_model.border_box = Some([0.0; 4]);
    runtime.invalidate(&id, Invalidation::Layout)?;
    runtime.flush(Duration::ZERO)?;
    assert_eq!(
        runtime.validation_report().issues[0].code,
        ValidationCode::EmptyHitArea
    );
    Ok(())
}
