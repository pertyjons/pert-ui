//! Recognizing double and triple clicks for a component.
use crate::{EventContext, Length, Point, PointerButton, PointerEvent, PointerKind};
use std::time::Duration;

/// The previous press of a series.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    at: Duration,
    position: Point,
    button: PointerButton,
}

/// Counts successive presses as single, double, triple and longer clicks,
/// for a component that reacts to them, as the built-in text editors, knobs
/// and file browser rows do.
///
/// A component owns one counter and feeds it the presses it accepts with
/// [`Self::press_event`] (or [`Self::press`]); each press returns its place in
/// the current series: 1 for a single click, 2 for the second press of a
/// double click, and so on. A press continues the series when the previous
/// press was of the same button, at most [`Self::DEFAULT_INTERVAL`] earlier,
/// and within [`Self::DEFAULT_DISTANCE`] of it on each axis, in window
/// coordinates; otherwise it starts a new series at 1. Both thresholds can be
/// replaced. A press timed before the previous one starts a new series. The
/// count saturates at [`u32::MAX`].
///
/// The counter never resets itself on other events. The component decides
/// what breaks a series, since only it knows what it accepted and consumed,
/// and calls [`Self::reset`]: when its input is cancelled or it loses focus,
/// when a press became a drag instead of a click, and when it pressed one of
/// its own parts (a button inside it) instead. Releasing the pointer capture
/// is no reason.
///
/// What follows a recognized click depends on the action. A component whose
/// double click does one thing, such as opening an item, resets after it, so
/// that four quick presses are two double clicks rather than a double and a
/// quadruple one. A component that steps through actions with each press of
/// one series, as a text editor selects a word on the second press and all
/// text on the third, keeps the series and its own bounded position in the
/// cycle; resetting there would make every third press a single click.
///
/// Time is the runtime's ([`EventContext::now`]), so a headless host decides
/// how much passes between presses by flushing; the native application
/// advances it as input arrives. The interval is the counter's own, not a
/// platform setting.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::time::Duration;
///
/// let mut clicks = ClickCounter::new();
/// let at = Point::new(10.0, 10.0)?;
/// let ms = Duration::from_millis;
/// assert_eq!(clicks.press(ms(0), at, PointerButton::Primary), 1);
/// assert_eq!(clicks.press(ms(200), at, PointerButton::Primary), 2);
/// assert_eq!(clicks.press(ms(400), Point::new(13.0, 12.0)?, PointerButton::Primary), 3);
/// // Too late: a new series.
/// assert_eq!(clicks.press(ms(1000), at, PointerButton::Primary), 1);
/// // Acting on a double click and starting over.
/// assert_eq!(clicks.press(ms(1100), at, PointerButton::Primary), 2);
/// clicks.reset();
/// assert_eq!(clicks.press(ms(1200), at, PointerButton::Primary), 1);
/// # Ok::<(), GeometryError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ClickCounter {
    interval: Duration,
    distance: Length,
    last: Option<Press>,
    count: u32,
}

impl Default for ClickCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickCounter {
    /// The longest time between two presses of one series: 500 ms.
    pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(500);
    /// The farthest a press may be from the previous one, on each axis, and
    /// continue its series: four logical pixels.
    pub const DEFAULT_DISTANCE: Length = Length::trusted(4.0);

    /// A counter with the default interval and distance and no series.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            interval: Self::DEFAULT_INTERVAL,
            distance: Self::DEFAULT_DISTANCE,
            last: None,
            count: 0,
        }
    }

    /// Replaces the longest time between two presses of one series. Zero
    /// lets only presses at one instant continue a series. Applies from the
    /// next press.
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Replaces the farthest distance, in logical pixels on each axis,
    /// between two presses of one series. Zero requires the same position.
    /// Applies from the next press.
    #[must_use]
    pub const fn with_distance(mut self, distance: Length) -> Self {
        self.distance = distance;
        self
    }

    /// Counts a press of `button` at `position`, in window coordinates, at
    /// the runtime time `at`, and returns its place in the series.
    pub fn press(&mut self, at: Duration, position: Point, button: PointerButton) -> u32 {
        let distance = self.distance.get();
        let continues = self.last.is_some_and(|last| {
            last.button == button
                && at >= last.at
                && at - last.at <= self.interval
                && (position.x() - last.position.x()).abs() <= distance
                && (position.y() - last.position.y()).abs() <= distance
        });
        self.count = if continues {
            self.count.saturating_add(1)
        } else {
            1
        };
        self.last = Some(Press {
            at,
            position,
            button,
        });
        self.count
    }

    /// Counts `event` if it is a [`PointerKind::Press`], at
    /// [`EventContext::now`] and the event's window position, and returns
    /// its place in the series; returns `None` for every other kind, which
    /// leaves the series as it was.
    pub fn press_event(&mut self, cx: &EventContext, event: PointerEvent) -> Option<u32> {
        match event.kind() {
            PointerKind::Press(button) => Some(self.press(cx.now(), event.position(), button)),
            _ => None,
        }
    }

    /// The place of the last counted press in its series, or zero before
    /// the first press and after [`Self::reset`].
    #[must_use]
    pub const fn count(&self) -> u32 {
        self.count
    }

    /// Ends the series: the next press counts as 1.
    pub fn reset(&mut self) {
        self.last = None;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_series_count_saturates() {
        let mut clicks = ClickCounter::new();
        let at = Point::new(0.0, 0.0).unwrap();
        clicks.press(Duration::ZERO, at, PointerButton::Primary);
        clicks.count = u32::MAX - 1;
        let next = |clicks: &mut ClickCounter, ms| {
            clicks.press(Duration::from_millis(ms), at, PointerButton::Primary)
        };
        assert_eq!(next(&mut clicks, 10), u32::MAX);
        assert_eq!(next(&mut clicks, 20), u32::MAX, "stays at the maximum");
    }
}
