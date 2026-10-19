//! `ClickCounter`: series of presses by time, distance and button.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}
fn at(x: f32, y: f32) -> Point {
    Point::new(x, y).unwrap()
}
const PRIMARY: PointerButton = PointerButton::Primary;

#[test]
fn presses_count_up_within_the_interval_and_distance_of_the_previous_one() {
    let mut clicks = ClickCounter::new();
    assert_eq!(clicks.count(), 0);
    assert_eq!(clicks.press(ms(0), at(10.0, 10.0), PRIMARY), 1);
    // Exactly at the interval and at the distance on one axis.
    assert_eq!(clicks.press(ms(500), at(14.0, 10.0), PRIMARY), 2);
    // Diagonally at the distance on both axes: measured per axis.
    assert_eq!(clicks.press(ms(1000), at(18.0, 14.0), PRIMARY), 3);
    assert_eq!(clicks.count(), 3);
    // A series may drift: each press is measured from the previous one.
    assert_eq!(clicks.press(ms(1100), at(21.0, 14.0), PRIMARY), 4);
    // Just past the interval.
    assert_eq!(clicks.press(ms(1601), at(21.0, 14.0), PRIMARY), 1);
    // Just past the distance.
    assert_eq!(clicks.press(ms(1700), at(25.5, 14.0), PRIMARY), 1);
    // Another button starts over, and so does an earlier time.
    assert_eq!(
        clicks.press(ms(1800), at(25.5, 14.0), PointerButton::Secondary),
        1
    );
    assert_eq!(
        clicks.press(ms(1900), at(25.5, 14.0), PointerButton::Secondary),
        2
    );
    assert_eq!(
        clicks.press(ms(1850), at(25.5, 14.0), PointerButton::Secondary),
        1
    );
}

#[test]
fn reset_and_replaced_thresholds() {
    let mut clicks = ClickCounter::new();
    clicks.press(ms(0), at(0.0, 0.0), PRIMARY);
    assert_eq!(clicks.press(ms(100), at(0.0, 0.0), PRIMARY), 2);
    clicks.reset();
    assert_eq!(clicks.count(), 0);
    assert_eq!(clicks.press(ms(200), at(0.0, 0.0), PRIMARY), 1);

    // Zero interval: only presses at one instant continue a series.
    let mut instant = ClickCounter::new().with_interval(Duration::ZERO);
    instant.press(ms(0), at(0.0, 0.0), PRIMARY);
    assert_eq!(instant.press(ms(0), at(0.0, 0.0), PRIMARY), 2);
    assert_eq!(instant.press(ms(1), at(0.0, 0.0), PRIMARY), 1);

    // Zero distance: only the same position.
    let mut exact = ClickCounter::new().with_distance(Length::new(0.0).unwrap());
    exact.press(ms(0), at(5.0, 5.0), PRIMARY);
    assert_eq!(exact.press(ms(10), at(5.0, 5.0), PRIMARY), 2);
    assert_eq!(exact.press(ms(20), at(5.5, 5.0), PRIMARY), 1);

    // A wider tolerance.
    let mut loose = ClickCounter::new()
        .with_interval(ms(1000))
        .with_distance(Length::new(20.0).unwrap());
    loose.press(ms(0), at(0.0, 0.0), PRIMARY);
    assert_eq!(loose.press(ms(900), at(20.0, 20.0), PRIMARY), 2);
    assert_eq!(ClickCounter::default(), ClickCounter::new());
    assert_eq!(ClickCounter::DEFAULT_INTERVAL, ms(500));
    assert_eq!(ClickCounter::DEFAULT_DISTANCE.get(), 4.0);
}

/// Feeds every pointer event to a counter and records what it returned.
struct Area {
    clicks: ClickCounter,
    seen: Rc<RefCell<Vec<Option<u32>>>>,
}
impl Component for Area {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let counted = self.clicks.press_event(cx, event);
        self.seen.borrow_mut().push(counted);
        Ok(InputOutcome::Handled)
    }
}

#[test]
fn press_event_counts_presses_on_the_runtime_clock_and_ignores_the_rest() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "area",
        Area {
            clicks: ClickCounter::new(),
            seen: seen.clone(),
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let send = |ui: &mut Runtime, kind| {
        ui.pointer_input(PointerEvent::new(at(10.0, 10.0), kind))
            .unwrap();
    };
    send(&mut ui, PointerKind::Press(PRIMARY));
    send(&mut ui, PointerKind::Release(PRIMARY));
    send(&mut ui, PointerKind::Move);
    send(&mut ui, PointerKind::Press(PRIMARY));
    // The clock only moves through the host: 600 ms later is too late.
    ui.flush(ms(600)).unwrap();
    send(&mut ui, PointerKind::Press(PRIMARY));
    let presses: Vec<_> = seen.borrow().iter().flatten().copied().collect();
    assert_eq!(presses, [1, 2, 1]);
    assert_eq!(seen.borrow()[1], None, "a release is not counted");
    assert_eq!(seen.borrow()[2], None, "a move is not counted");
}
