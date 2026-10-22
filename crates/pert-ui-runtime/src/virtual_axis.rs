//! Window arithmetic for virtualized collections, with no component attached.
use crate::{GeometryError, LayoutError};
use std::ops::Range;

/// Actual line extents, with a Fenwick tree of positive prefix sums.
/// Recomputing affected nodes avoids cancellation of a small extent against
/// a much larger estimate or previous measurement.
#[derive(Clone, Debug, PartialEq)]
struct Measured {
    /// The estimate until a line is measured.
    extents: Vec<f32>,
    /// Fenwick tree of actual extents, one-based, in double precision.
    tree: Vec<f64>,
}
impl Measured {
    fn with_lines(lines: usize, estimate: f32) -> Self {
        Self {
            extents: vec![estimate; lines],
            tree: (0..=lines)
                .map(|at| at.isolate_lowest_one() as f64 * f64::from(estimate))
                .collect(),
        }
    }
    fn refresh(&mut self, at: usize) {
        let start = at - at.isolate_lowest_one();
        let mut child = at - 1;
        let mut sum = f64::from(self.extents[child]);
        while child > start {
            sum += self.tree[child];
            child -= child.isolate_lowest_one();
        }
        self.tree[at] = sum;
    }
    fn set(&mut self, line: usize, extent: f32) {
        self.extents[line] = extent;
        let mut at = line + 1;
        while at < self.tree.len() {
            self.refresh(at);
            at += at.isolate_lowest_one();
        }
    }
    /// The sum of every line before `line`.
    fn before(&self, line: usize) -> f64 {
        let mut at = line.min(self.extents.len());
        let mut sum = 0.0;
        while at > 0 {
            sum += self.tree[at];
            at -= at.isolate_lowest_one();
        }
        sum
    }
    fn resize(&mut self, lines: usize, estimate: f32) {
        if lines == self.extents.len() {
            return;
        }
        self.extents.resize(lines, estimate);
        self.tree = vec![0.0; lines + 1];
        for at in 1..=lines {
            self.refresh(at);
        }
    }
}

/// Items along a scrolling axis, arranged in lanes, with one fixed extent or with
/// measured extents over an estimate.
///
/// This is the arithmetic a virtualized container needs and nothing else: from an
/// offset and a viewport to the items that must be mounted, where each one sits,
/// and how far to scroll to show one. It knows nothing about components, input or
/// drawing, so [`VirtualList`](crate::VirtualList) and a consumer's own container
/// compute the same ranges.
///
/// Items fill lanes in order: item `i` is on line `i / lanes`, in lane
/// `i % lanes`. One lane is a list; several are a grid that scrolls along the
/// axis. Extents, offsets and viewports are logical pixels along the axis.
///
/// Positions along the axis, which are line starts, the content extent and
/// scroll offsets, are `f64`, while one line's extent and a viewport are `f32`
/// like the rest of the geometry. A long collection's content runs past the
/// 2^24 pixels where `f32` stops holding every whole pixel: 20 million rows of
/// 24 pixels are 480 million pixels, where neighbouring `f32` values are 32
/// pixels apart. In `f64` those line starts stay distinct and a small scroll
/// still moves. Arbitrarily disparate positive extents can still exceed f64's
/// relative precision: a tiny line after a huge one may share its rounded start.
/// A container keeps the offset as `f64` and converts only
/// positions relative to it, which are as small as the viewport, to `f32`.
///
/// [`VirtualAxis::new`] gives every line the same extent, and every answer is
/// plain multiplication and division. [`VirtualAxis::estimated`] gives each line
/// an estimate until [`VirtualAxis::measure_line`] records its real extent, for
/// rows of wrapped text or mixed heights. Measured extents are kept per line as a
/// positive extent, so a prefix lookup costs a logarithm of the line count.
/// An update recomputes index nodes in at most the square of that logarithm;
/// it never reconstructs a small positive measurement by subtracting its estimate.
/// Once an axis is estimated,
/// each line costs an `f32` extent plus an `f64` node of the index, measured
/// or not. Measuring a line that starts before the viewport returns a corrected
/// offset, which keeps what is on screen where it was: that is the anchoring that
/// stops content from jumping as estimates are replaced while scrolling upwards.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::VirtualAxis;
/// // 1,000 items, 24 pixels each, three to a line.
/// let axis = VirtualAxis::new(24.0)?.with_count(1_000).with_lanes(3)?;
/// assert_eq!(axis.lines(), 334);
/// assert_eq!(axis.extent(), 334.0 * 24.0);
/// // A 100-pixel viewport at offset 240 shows lines 10 to 14; one line of
/// // overscan on each side mounts lines 9 to 15, which are items 27 to 47.
/// assert_eq!(axis.window(240.0, 100.0, 1), 27..48);
/// assert_eq!(axis.line_offset(27), 9.0 * 24.0);
/// assert_eq!(axis.lane(28), 1);
///
/// // Estimated: line 2 turns out to be 60 pixels, not 20. It starts above a
/// // viewport at offset 100, so the offset moves by the 40 pixels it grew.
/// let mut rows = VirtualAxis::estimated(20.0)?.with_count(100);
/// assert_eq!(rows.measure_line(2, 60.0, 100.0)?, 140.0);
/// assert_eq!(rows.line_offset(3), 100.0);
/// assert_eq!(rows.extent(), 100.0 * 20.0 + 40.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct VirtualAxis {
    item_extent: f32,
    count: usize,
    lanes: usize,
    measured: Option<Measured>,
}
/// Accepts a finite, strictly positive extent.
fn positive(extent: f32) -> Result<(), GeometryError> {
    if !extent.is_finite() {
        return Err(GeometryError::NonFinite);
    }
    if extent < 0.0 {
        return Err(GeometryError::Negative);
    }
    if extent == 0.0 {
        return Err(GeometryError::Empty);
    }
    Ok(())
}
impl VirtualAxis {
    /// Creates an empty single-lane axis whose lines are all `item_extent` long.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](GeometryError::NonFinite),
    /// [`GeometryError::Negative`](GeometryError::Negative) or
    /// [`GeometryError::Empty`](GeometryError::Empty) for an extent that
    /// is not finite and positive.
    pub fn new(item_extent: f32) -> Result<Self, GeometryError> {
        positive(item_extent)?;
        Ok(Self {
            item_extent,
            count: 0,
            lanes: 1,
            measured: None,
        })
    }
    /// Creates an empty single-lane axis whose lines are `estimate` long until
    /// [`VirtualAxis::measure_line`] says otherwise.
    ///
    /// # Errors
    /// As [`Self::new`], for an estimate that is not finite and positive.
    pub fn estimated(estimate: f32) -> Result<Self, GeometryError> {
        let mut axis = Self::new(estimate)?;
        axis.measured = Some(Measured::with_lines(0, estimate));
        Ok(axis)
    }
    /// Sets how many items there are.
    pub fn with_count(mut self, count: usize) -> Self {
        self.set_count(count);
        self
    }
    /// Sets how many items there are, keeping the measured extents of lines that
    /// remain. Rebuilding the index costs the line count when it changes.
    pub fn set_count(&mut self, count: usize) {
        self.count = count;
        let lines = self.lines();
        if let Some(measured) = &mut self.measured {
            measured.resize(lines, self.item_extent);
        }
    }
    /// Sets how many items share a line. Measured extents are forgotten, since a
    /// line then holds different items.
    ///
    /// # Errors
    /// Returns [`LayoutError::NoLanes`] for zero lanes.
    pub fn with_lanes(mut self, lanes: usize) -> Result<Self, LayoutError> {
        if lanes == 0 {
            return Err(LayoutError::NoLanes);
        }
        self.lanes = lanes;
        self.forget();
        Ok(self)
    }
    /// Whether lines are measured over an estimate rather than fixed.
    #[must_use]
    pub const fn is_estimated(&self) -> bool {
        self.measured.is_some()
    }
    /// The fixed extent of every line, or the estimate of an unmeasured one.
    #[must_use]
    pub const fn item_extent(&self) -> f32 {
        self.item_extent
    }
    /// How many items there are.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
    /// How many items share a line; at least one.
    #[must_use]
    pub const fn lanes(&self) -> usize {
        self.lanes
    }
    /// How many lines the items occupy; the last one may be partly filled.
    #[must_use]
    pub const fn lines(&self) -> usize {
        self.count.div_ceil(self.lanes)
    }
    /// The extent of one line: fixed, measured, or still the estimate.
    #[must_use]
    pub fn line_extent(&self, line: usize) -> f32 {
        self.measured
            .as_ref()
            .and_then(|measured| measured.extents.get(line))
            .copied()
            .unwrap_or(self.item_extent)
    }
    // Where a line starts. The fixed case is one multiplication, which `f64`
    // holds exactly below 2^29 lines: an `f32` extent has 24 significant bits
    // and the product fits in the 53 of an `f64`.
    fn line_start(&self, line: usize) -> f64 {
        let fixed = line as f64 * f64::from(self.item_extent);
        match &self.measured {
            None => fixed,
            Some(measured) => measured.before(line),
        }
    }
    /// The extent of all lines.
    #[must_use]
    pub fn extent(&self) -> f64 {
        self.line_start(self.lines())
    }
    /// The largest useful offset for a viewport; zero when everything fits.
    #[must_use]
    pub fn maximum_offset(&self, viewport: f32) -> f64 {
        (self.extent() - f64::from(viewport.max(0.0))).max(0.0)
    }
    /// Limits an offset to what the viewport can scroll to. A NaN offset is
    /// zero.
    #[must_use]
    pub fn clamp_offset(&self, offset: f64, viewport: f32) -> f64 {
        if offset.is_nan() {
            return 0.0;
        }
        offset.clamp(0.0, self.maximum_offset(viewport))
    }
    /// The line an item is on.
    #[must_use]
    pub const fn line(&self, index: usize) -> usize {
        index / self.lanes
    }
    /// The lane an item is in, counted from zero across the axis.
    #[must_use]
    pub const fn lane(&self, index: usize) -> usize {
        index % self.lanes
    }
    /// Where an item's line starts along the axis, before any scroll offset.
    #[must_use]
    pub fn line_offset(&self, index: usize) -> f64 {
        self.line_start(self.line(index))
    }
    /// The line that contains a position along the axis, clamped to the lines
    /// there are; zero for an empty axis. A position on a boundary belongs to the
    /// line that starts there.
    #[must_use]
    pub fn line_at(&self, position: f64) -> usize {
        let lines = self.lines();
        if lines == 0 || position.is_nan() || position <= 0.0 {
            return 0;
        }
        if self.measured.is_none() {
            // Truncation is the intended floor of a nonnegative quotient.
            return ((position / f64::from(self.item_extent)).floor() as usize).min(lines - 1);
        }
        // The first line whose end is past the position. Line starts increase
        // with the line, so a binary search over them finds it.
        let (mut low, mut high) = (0, lines - 1);
        while low < high {
            let middle = low + (high - low) / 2;
            if self.line_start(middle + 1) > position {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    }
    /// How many whole lines a viewport holds at the nominal extent; at least one,
    /// so paging always moves. With measured lines this is an approximation.
    #[must_use]
    pub fn lines_per_page(&self, viewport: f32) -> usize {
        // Truncation is the intended floor of a nonnegative quotient.
        ((viewport.max(0.0) / self.item_extent).floor() as usize).max(1)
    }
    /// The items a viewport at `offset` needs mounted: every item on a line that
    /// the viewport touches, widened by `overscan` whole lines on each side.
    ///
    /// Empty for an empty axis or a viewport with no extent. The offset is used
    /// as given; clamp it first with [`VirtualAxis::clamp_offset`].
    #[must_use]
    pub fn window(&self, offset: f64, viewport: f32, overscan: usize) -> Range<usize> {
        if self.count == 0 || viewport <= 0.0 || !viewport.is_finite() || !offset.is_finite() {
            return 0..0;
        }
        let lines = self.lines();
        let viewport = f64::from(viewport);
        let (first, last) = if self.measured.is_none() {
            // Lines are counted; truncation is the intended floor and ceiling of
            // nonnegative quotients, exact for any line count a collection holds.
            let extent = f64::from(self.item_extent);
            (
                (offset / extent).floor().max(0.0) as usize,
                ((offset + viewport) / extent).ceil().max(0.0) as usize,
            )
        } else {
            let end = offset + viewport;
            let last = self.line_at(end);
            // The line under the far edge is needed unless the edge is its start.
            let touched = self.line_start(last) < end;
            (self.line_at(offset), last + usize::from(touched))
        };
        let first = first.saturating_sub(overscan).min(lines);
        let last = last.saturating_add(overscan).min(lines);
        let start = first.saturating_mul(self.lanes).min(self.count);
        let end = last.saturating_mul(self.lanes).min(self.count);
        start..end.max(start)
    }
    /// The offset that brings an item's whole line into a viewport at `offset`,
    /// moving as little as possible: unchanged when it is already inside,
    /// otherwise aligned to the nearer edge. A line longer than the viewport
    /// aligns to its start. The result is clamped to the scrollable range.
    #[must_use]
    pub fn reveal(&self, index: usize, offset: f64, viewport: f32) -> f64 {
        let start = self.line_offset(index);
        let extent = self.line_extent(self.line(index));
        let end = start + f64::from(extent);
        let next = if viewport <= 0.0 || start < offset || extent > viewport {
            start
        } else if end > offset + f64::from(viewport) {
            end - f64::from(viewport)
        } else {
            offset
        };
        self.clamp_offset(next, viewport)
    }
    /// Records a line's real extent and returns the offset that keeps the
    /// viewport's content where it was.
    ///
    /// A line that starts before `offset` lies above the viewport, so its growth
    /// or shrinkage is added to the offset; a line at or after it leaves the
    /// offset alone. The returned offset is not clamped. On a fixed axis nothing
    /// is recorded and `offset` is returned unchanged, so a container can call
    /// this unconditionally.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Geometry`](crate::RuntimeError::Geometry) for an
    /// extent that is not finite and positive, as [`Self::new`] does, and
    /// [`RuntimeError::Layout`](crate::RuntimeError::Layout) holding
    /// [`LayoutError::InvalidChild`] for a line past the end.
    pub fn measure_line(
        &mut self,
        line: usize,
        extent: f32,
        offset: f64,
    ) -> Result<f64, crate::RuntimeError> {
        positive(extent)?;
        if line >= self.lines() {
            return Err(LayoutError::InvalidChild.into());
        }
        let above = self.line_start(line) < offset;
        let Some(measured) = &mut self.measured else {
            return Ok(offset);
        };
        let previous = measured.extents[line];
        if extent == previous {
            return Ok(offset);
        }
        let change = f64::from(extent) - f64::from(previous);
        measured.set(line, extent);
        Ok(if above { offset + change } else { offset })
    }
    /// Forgets every measured extent, returning all lines to the estimate. Use it
    /// when the items changed so that a line no longer holds what was measured.
    pub fn forget(&mut self) {
        let lines = self.lines();
        if let Some(measured) = &mut self.measured {
            *measured = Measured::with_lines(lines, self.item_extent);
        }
    }
    /// The items to mount for a window plus `pinned` items that must stay mounted
    /// outside it, in presentation order: pinned items before the window, the
    /// window, then pinned items after it. Pinned items inside the window or past
    /// the end are dropped, and duplicates are removed.
    #[must_use]
    pub fn mount_order(
        &self,
        window: Range<usize>,
        pinned: impl IntoIterator<Item = usize>,
    ) -> Vec<usize> {
        let mut pinned: Vec<usize> = pinned
            .into_iter()
            .filter(|index| *index < self.count && !window.contains(index))
            .collect();
        pinned.sort_unstable();
        pinned.dedup();
        let split = pinned.partition_point(|index| *index < window.start);
        let mut order = Vec::with_capacity(pinned.len() + window.len());
        order.extend_from_slice(&pinned[..split]);
        order.extend(window);
        order.extend_from_slice(&pinned[split..]);
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_measurements_survive_disparate_estimates_and_remeasurement() {
        for estimate in [100.0, f32::MAX] {
            let mut rows = VirtualAxis::estimated(estimate).unwrap().with_count(2);
            for extent in [1e-6, f32::from_bits(1), 2e-6] {
                rows.measure_line(0, extent, 0.0).unwrap();
                assert_eq!(rows.line_extent(0), extent);
                assert_eq!(rows.line_offset(1), f64::from(extent));
                assert_eq!(rows.line_at(f64::from(extent)), 1);
                rows.set_count(3);
                assert_eq!(rows.line_offset(1), f64::from(extent));
                rows.set_count(2);
            }
        }
    }

    #[test]
    fn replacing_all_large_estimates_recovers_small_prefix_sums() {
        let mut rows = VirtualAxis::estimated(f32::MAX).unwrap().with_count(8);
        for extent in [1e-6, 1e-5, 1e-6] {
            for line in 0..8 {
                rows.measure_line(line, extent, 0.0).unwrap();
            }
            assert_eq!(rows.extent(), 8.0 * f64::from(extent));
            for line in 0..8 {
                assert_eq!(rows.line_offset(line), line as f64 * f64::from(extent));
                assert_eq!(rows.line_at(rows.line_offset(line)), line);
            }
        }
    }

    fn axis(count: usize, lanes: usize) -> VirtualAxis {
        VirtualAxis::new(10.0)
            .expect("valid extent")
            .with_count(count)
            .with_lanes(lanes)
            .expect("valid lanes")
    }

    #[test]
    fn invalid_extents_and_lanes_are_rejected() {
        for (extent, error) in [
            (0.0, GeometryError::Empty),
            (-1.0, GeometryError::Negative),
            (f32::NAN, GeometryError::NonFinite),
            (f32::INFINITY, GeometryError::NonFinite),
        ] {
            assert_eq!(VirtualAxis::new(extent), Err(error));
        }
        assert_eq!(
            VirtualAxis::new(10.0).map(|axis| axis.with_lanes(0)),
            Ok(Err(LayoutError::NoLanes))
        );
    }

    #[test]
    fn a_window_covers_whole_lines_and_stays_inside_the_items() {
        let list = axis(100, 1);
        assert_eq!(list.window(0.0, 35.0, 0), 0..4);
        assert_eq!(list.window(35.0, 30.0, 0), 3..7);
        assert_eq!(list.window(35.0, 30.0, 2), 1..9);
        assert_eq!(list.window(990.0, 50.0, 4), 95..100);
        // Nothing to mount without items, without a viewport or with a bad offset.
        assert_eq!(axis(0, 1).window(0.0, 50.0, 4), 0..0);
        assert_eq!(list.window(0.0, 0.0, 4), 0..0);
        assert_eq!(list.window(f64::NAN, 50.0, 4), 0..0);
        // A partly filled last line ends at the item count, not the line's end.
        let grid = axis(10, 4);
        assert_eq!((grid.lines(), grid.extent()), (3, 30.0));
        assert_eq!(grid.window(0.0, 100.0, 0), 0..10);
        assert_eq!(grid.window(10.0, 10.0, 0), 4..8);
        assert_eq!(
            (grid.line(9), grid.lane(9), grid.line_offset(9)),
            (2, 1, 20.0)
        );
    }

    #[test]
    fn reveal_moves_as_little_as_possible_and_never_past_the_ends() {
        let list = axis(100, 1);
        assert_eq!(list.reveal(5, 20.0, 50.0), 20.0, "already inside");
        assert_eq!(list.reveal(1, 20.0, 50.0), 10.0, "aligned to the start");
        assert_eq!(list.reveal(9, 20.0, 50.0), 50.0, "aligned to the end");
        assert_eq!(list.reveal(99, 0.0, 50.0), 950.0);
        assert_eq!(list.reveal(0, 500.0, 50.0), 0.0);
        assert_eq!(
            list.reveal(3, 0.0, 5.0),
            30.0,
            "a line longer than the view"
        );
        assert_eq!(axis(3, 1).reveal(2, 0.0, 50.0), 0.0, "everything fits");
        assert_eq!(list.lines_per_page(55.0), 5);
        assert_eq!(list.lines_per_page(3.0), 1);
    }

    #[test]
    fn pinned_items_surround_the_window_in_order() {
        let list = axis(100, 1);
        assert_eq!(
            list.mount_order(10..13, [50, 2, 11, 2, 400]),
            [2, 10, 11, 12, 50]
        );
        assert_eq!(list.mount_order(0..0, []), Vec::<usize>::new());
    }

    fn estimated(count: usize) -> VirtualAxis {
        VirtualAxis::estimated(10.0)
            .map(|axis| axis.with_count(count))
            .expect("valid axis")
    }

    #[test]
    fn an_unmeasured_estimated_axis_answers_like_a_fixed_one() {
        let (fixed, rows) = (axis(1_000, 1), estimated(1_000));
        assert!(rows.is_estimated() && !fixed.is_estimated());
        for (offset, viewport, overscan) in [(0.0, 35.0, 0), (35.0, 30.0, 2), (9_990.0, 50.0, 4)] {
            assert_eq!(
                rows.window(offset, viewport, overscan),
                fixed.window(offset, viewport, overscan),
                "{offset}"
            );
        }
        assert_eq!(rows.extent(), fixed.extent());
        assert_eq!(rows.line_at(35.0), 3);
        assert_eq!(rows.line_at(40.0), 4, "a boundary belongs to the next line");
        assert_eq!(rows.line_at(1e9), 999);
        assert_eq!(estimated(0).line_at(5.0), 0);
    }

    #[test]
    fn measured_lines_move_everything_after_them_and_nothing_before() {
        let mut rows = estimated(100);
        assert_eq!(rows.measure_line(2, 50.0, 0.0), Ok(0.0));
        assert_eq!(rows.line_extent(2), 50.0);
        assert_eq!((rows.line_offset(2), rows.line_offset(3)), (20.0, 70.0));
        assert_eq!(rows.extent(), 1_040.0);
        // Lines 0, 1 and the tall line 2 fill a 60-pixel viewport at the top.
        assert_eq!(rows.window(0.0, 60.0, 0), 0..3);
        assert_eq!(rows.line_at(69.0), 2);
        assert_eq!(rows.line_at(70.0), 3);
        // Revealing the tall line from above aligns its end to the viewport's.
        assert_eq!(rows.reveal(2, 0.0, 60.0), 10.0);
        // A line can shrink again, and repeating a measurement changes nothing.
        assert_eq!(rows.measure_line(2, 5.0, 0.0), Ok(0.0));
        assert_eq!(rows.measure_line(2, 5.0, 0.0), Ok(0.0));
        assert_eq!(rows.extent(), 995.0);
    }

    #[test]
    fn measuring_above_the_viewport_anchors_what_is_on_screen() {
        let mut rows = estimated(100);
        // The viewport is at 300, so line 30 is at its top.
        let top = rows.line_at(300.0);
        let offset = rows.measure_line(5, 45.0, 300.0).expect("valid");
        assert_eq!(offset, 335.0, "line 5 grew by 35 above the viewport");
        assert_eq!(
            rows.line_at(offset),
            top,
            "the same line is still at the top"
        );
        // A line at or below the viewport's start leaves the offset alone.
        assert_eq!(rows.measure_line(top, 80.0, offset), Ok(offset));
        assert_eq!(rows.measure_line(90, 80.0, offset), Ok(offset));
    }

    #[test]
    fn measurements_survive_a_count_change_and_are_rejected_when_invalid() {
        let mut rows = estimated(10);
        rows.measure_line(1, 30.0, 0.0).expect("valid");
        rows.measure_line(8, 40.0, 0.0).expect("valid");
        rows.set_count(5);
        assert_eq!(rows.extent(), 70.0, "line 8 is gone, line 1 remains");
        rows.set_count(20);
        assert_eq!(rows.extent(), 220.0);
        rows.forget();
        assert_eq!(rows.extent(), 200.0);
        for extent in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                rows.measure_line(0, extent, 0.0),
                Err(crate::RuntimeError::Geometry(_))
            ));
        }
        assert_eq!(
            rows.measure_line(20, 10.0, 0.0),
            Err(LayoutError::InvalidChild.into())
        );
        // A fixed axis ignores measurements, so a container may always report them.
        let mut fixed = axis(10, 1);
        assert_eq!(fixed.measure_line(3, 99.0, 7.0), Ok(7.0));
        assert_eq!(fixed.extent(), 100.0);
    }

    // Past 2^24 pixels `f32` no longer holds every whole pixel; 20 million
    // 24-pixel rows reach 480 million, where `f32` values are 32 apart.
    #[test]
    fn line_starts_stay_exact_far_past_the_precision_of_f32() {
        let count = 20_000_000;
        let fixed = VirtualAxis::new(24.0).expect("valid").with_count(count);
        assert_eq!(fixed.extent(), 480_000_000.0);
        let mut rows = VirtualAxis::estimated(24.0)
            .expect("valid")
            .with_count(count);
        rows.measure_line(3, 30.0, 0.0).expect("valid");
        for line in [count - 1_000, count - 999, count - 2, count - 1] {
            assert_eq!(fixed.line_offset(line), line as f64 * 24.0);
            assert_eq!(rows.line_offset(line), line as f64 * 24.0 + 6.0);
            assert_eq!(fixed.line_at(fixed.line_offset(line)), line);
            assert_eq!(rows.line_at(rows.line_offset(line) + 1.0), line);
        }
        // Neighbouring lines never share a start, and a one-pixel scroll at the
        // far end still moves the window's first line boundary by one pixel.
        let last = fixed.maximum_offset(240.0);
        assert!(fixed.line_offset(count - 2) < fixed.line_offset(count - 1));
        assert_eq!(last - 1.0 + 1.0, last);
        assert_eq!(fixed.window(last - 1.0, 240.0, 0), count - 11..count);
        assert_eq!(fixed.window(last, 240.0, 0), count - 10..count);
        assert_eq!(fixed.reveal(count - 1, 0.0, 240.0), last);
    }

    #[test]
    fn a_long_axis_stays_consistent_between_its_two_directions() {
        let mut rows = estimated(200_000);
        for line in (0..200_000).step_by(7) {
            rows.measure_line(line, 10.0 + (line % 13) as f32, 0.0)
                .expect("valid");
        }
        for line in [0, 1, 6, 7, 99_999, 123_456, 199_999] {
            let start = rows.line_offset(line);
            assert_eq!(rows.line_at(start), line, "start of {line}");
            let inside = start + f64::from(rows.line_extent(line)) / 2.0;
            assert_eq!(rows.line_at(inside), line, "inside {line}");
        }
    }
}
