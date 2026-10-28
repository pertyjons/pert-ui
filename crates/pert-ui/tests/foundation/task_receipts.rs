//! Which deferral receipts must be used: `TaskReceipt`, from
//! `EventContext::defer`, `EventContext::spawn` and `Runtime::defer`, may be
//! dropped, so a callback writes `cx.defer(...)?;` without binding it, while
//! `ShownTaskReceipt`, from `EventContext::defer_while_shown`, warns when it is
//! discarded (its `compile_fail` doctest under `#![deny(unused_must_use)]`).
//!
//! New-behavior test (round 2 fix plan P3, R2-4). The module denies
//! `unused_must_use`, so the named assertion is that it compiles. Mutation:
//! `#[must_use]` back on `TaskReceipt`; each statement below is then an error.
#![deny(unused_must_use)]

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn defer_and_spawn_receipts_may_be_dropped_after_the_question_mark() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 80.0, 1.0)?)?;
    let deferred = Rc::new(Cell::new(false));
    let counted = ui.signal(None::<usize>);
    let (flag, result) = (deferred.clone(), counted.clone());
    let button = ui.mount_tree(Element::new(
        "go",
        Button::new("Go", move |cx| {
            let flag = flag.clone();
            cx.defer(move |_ui| {
                flag.set(true);
                Ok(())
            })?;
            let result = result.clone();
            cx.spawn(|| 6 * 7, move |answer| result.set(Some(answer)))?;
            Ok(())
        }),
    ))?;
    let applied = Rc::new(Cell::new(false));
    let marker = applied.clone();
    ui.defer(move |_ui| {
        marker.set(true);
        Ok(())
    })?;
    ui.flush(Duration::ZERO)?;
    assert!(applied.get(), "Runtime::defer ran");
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    assert!(deferred.get(), "the callback's defer ran");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while counted.get()?.is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
        ui.flush(Duration::ZERO)?;
    }
    assert_eq!(counted.get()?, Some(42), "the callback's spawn delivered");
    Ok(())
}
