//! Signal writes through the facade: `set`, `set_changed`, `update` and
//! `toggle`, their phase checks and writes made inside an update closure.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn set_changed_reports_a_change_and_set_does_not() -> TestResult {
    let ui = Runtime::new();
    let value = ui.signal(1);
    assert!(!value.set_changed(1)?);
    assert!(value.set_changed(2)?);
    value.set(3)?;
    assert_eq!(value.get()?, 3);
    let flag = ui.signal(false);
    flag.toggle()?;
    assert!(flag.get()?);
    Ok(())
}

#[test]
fn update_writes_the_changed_copy_and_an_unchanged_copy_notifies_nobody() -> TestResult {
    let ui = Runtime::new();
    let items = ui.signal(vec![1, 2]);
    let revision = items.revision();
    items.update(|items| items.retain(|item| *item > 5))?;
    assert!(items.get()?.is_empty());
    let after = items.revision();
    assert_ne!(after, revision);
    items.update(|_| {})?;
    assert_eq!(items.revision(), after, "an unchanged copy is no write");
    Ok(())
}

/// Tries to update a signal from its own update, which is a forbidden phase.
struct Writer {
    target: Signal<i32>,
    ran: Rc<Cell<bool>>,
    outcome: Rc<Cell<Option<bool>>>,
}
impl Component for Writer {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        let ran = self.ran.clone();
        let result = self.target.update(move |value| {
            ran.set(true);
            *value += 1;
        });
        self.outcome
            .set(Some(result == Err(RuntimeError::WriteDuringUpdate)));
        Ok(())
    }
}

#[test]
fn update_in_a_forbidden_phase_never_runs_its_closure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    let target = ui.signal(0);
    let (ran, outcome) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(None)));
    ui.mount_tree(
        Writer {
            target: target.clone(),
            ran: ran.clone(),
            outcome: outcome.clone(),
        }
        .key("writer"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(outcome.get(), Some(true));
    assert!(!ran.get());
    assert_eq!(target.get()?, 0);
    Ok(())
}

#[test]
fn a_write_inside_update_takes_effect_and_the_changed_copy_replaces_it() -> TestResult {
    let ui = Runtime::new();
    let value = ui.signal(0);
    let inner = value.clone();
    let revision = value.revision();
    value.update(|copy| {
        inner.set(5).unwrap();
        *copy += 1;
    })?;
    assert_eq!(
        value.get()?,
        1,
        "the copy, taken before the inner write, wins"
    );
    assert_eq!(
        value.revision(),
        revision.wrapping_add(2),
        "both writes happened"
    );
    Ok(())
}

#[test]
fn a_panicking_update_makes_no_write_of_its_own() -> TestResult {
    let ui = Runtime::new();
    let value = ui.signal(1);
    let revision = value.revision();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = value.update(|copy| {
            *copy = 2;
            panic!("the change failed");
        });
    }));
    assert!(panicked.is_err());
    assert_eq!(value.get()?, 1);
    assert_eq!(value.revision(), revision, "nobody was notified");
    // A write the change made itself before panicking stays made.
    let inner = value.clone();
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = value.update(|copy| {
            inner.set(7).unwrap();
            *copy = 9;
            panic!("the change failed after a write");
        });
    }));
    assert!(panicked.is_err());
    assert_eq!(value.get()?, 7);
    // The signal is still usable.
    value.update(|copy| *copy += 1)?;
    assert_eq!(value.get()?, 8);
    Ok(())
}
