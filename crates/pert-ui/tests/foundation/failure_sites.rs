//! Failure sites through the facade: a failing flush or paint names the
//! component where it arose, and the next one starts without a site.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Fails measurement or painting while its switch reads true.
struct Faulty {
    measure: Rc<Cell<bool>>,
    paint: Rc<Cell<bool>>,
}
impl Component for Faulty {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        if self.measure.get() {
            return Err(LayoutError::InvalidChild.into());
        }
        Ok(Size::new(10.0, 10.0)?)
    }
    fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
        if self.paint.get() {
            return Err(DrawError::Runtime(RuntimeError::Exhausted));
        }
        Ok(())
    }
}

#[test]
fn a_failure_is_named_where_it_arose_and_cleared_by_the_next_success() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (measure, paint) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(false)));
    let root = ui.mount_tree(
        Column::new([Container::new(
            Faulty {
                measure: measure.clone(),
                paint: paint.clone(),
            }
            .key("leaf"),
        )
        .key("frame")])
        .key("form"),
    )?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let site = ui.last_failure().ok_or("a component failed")?;
    assert_eq!(site.path, ["form", "frame", "leaf"]);
    assert_eq!(site.phase, FailurePhase::Measure);
    measure.set(false);
    let leaf = ui.find_path(&root, &["frame", "leaf"])?.ok_or("mounted")?;
    ui.invalidate(&leaf, Invalidation::Layout)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    paint.set(true);
    ui.invalidate(&leaf, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.paint().is_err());
    let site = ui.last_failure().ok_or("painting failed")?;
    assert_eq!(site.phase, FailurePhase::Paint);
    assert_eq!(site.to_string(), "painting of form/frame/leaf");
    paint.set(false);
    ui.invalidate(&leaf, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(ui.last_failure(), None);
    Ok(())
}

/// Fails its update while its switch reads true.
struct Switch(Rc<Cell<bool>>);
impl Component for Switch {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.0.get() {
            return Err(RuntimeError::Exhausted);
        }
        Ok(())
    }
}

#[test]
fn flushing_every_window_names_its_own_failure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (a, b) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(false)));
    let root = ui.mount_tree(
        Column::new([Switch(a.clone()).key("a"), Switch(b.clone()).key("b")]).key("root"),
    )?;
    assert!(ui.flush_windows(Duration::ZERO).is_err());
    assert_eq!(
        ui.last_failure().map(|site| site.path),
        Some(vec!["root".to_owned(), "a".to_owned()])
    );
    a.set(false);
    for key in ["a", "b"] {
        let id = ui.find_path(&root, &[key])?.ok_or("mounted")?;
        ui.invalidate(&id, Invalidation::Paint)?;
    }
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    b.set(true);
    let id = ui.find_path(&root, &["b"])?.ok_or("mounted")?;
    ui.invalidate(&id, Invalidation::Paint)?;
    assert!(ui.flush_windows(Duration::ZERO).is_err());
    assert_eq!(
        ui.last_failure().map(|site| site.path),
        Some(vec!["root".to_owned(), "b".to_owned()])
    );
    Ok(())
}

/// A container that places none of its children.
struct Forgetful;
impl Component for Forgetful {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn a_child_left_unplaced_is_the_containers_arrangement_failure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    ui.mount_tree(
        Element::new("forgetful", Forgetful).children([Label::new("child").key("child")]),
    )?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let site = ui.last_failure().ok_or("a component failed")?;
    assert_eq!(site.phase, FailurePhase::Arrange);
    assert_eq!(site.path.last().map(String::as_str), Some("forgetful"));
    Ok(())
}
