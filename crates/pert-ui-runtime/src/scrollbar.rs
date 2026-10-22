//! The one overlay scrollbar geometry, shared by scroll areas and text editors.
//!
//! Scroll areas and multiline fields present the same bar, so they derive it here
//! instead of each repeating the lane, thumb and travel arithmetic. The extents
//! are the theme's scrollbar metrics, so the bar follows density like every other
//! control extent.
use crate::{Color, ColorRole, MeasureContext, MetricRole, Painter, Point, Rect, ThemeMetrics};

/// The thumb's color: translucent at rest, more opaque while the pointer is
/// over its bar or dragging it, so the bar shows that it can be dragged.
pub(crate) fn thumb_color(painter: &Painter, active: bool) -> Color {
    painter.color(if active {
        ColorRole::ScrollbarThumbHover
    } else {
        ColorRole::ScrollbarThumb
    })
}

/// The bar's theme extents in logical pixels, as read during arrangement.
///
/// Input and hit testing have no metric access, so a component reads these once
/// per arrangement and retains them beside its other arranged geometry; painting
/// uses the same retained values, which keeps what is drawn and what is hit
/// identical. Reading through the context registers the metric dependency, so a
/// density change arranges the component again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Extents {
    /// [`MetricRole::ScrollbarLane`].
    pub(crate) lane: f32,
    /// [`MetricRole::ScrollbarThumb`].
    pub(crate) thumb: f32,
    /// [`MetricRole::ScrollbarMinimumThumb`].
    pub(crate) minimum_thumb: f32,
}
impl Extents {
    const fn of(metrics: &ThemeMetrics) -> Self {
        Self {
            lane: metrics.metric(MetricRole::ScrollbarLane),
            thumb: metrics.metric(MetricRole::ScrollbarThumb),
            minimum_thumb: metrics.metric(MetricRole::ScrollbarMinimumThumb),
        }
    }
    /// The Standard-density defaults, held until the first arrangement.
    pub(crate) const STANDARD: Self =
        Self::of(&ThemeMetrics::for_density(crate::Density::Standard));
    pub(crate) fn read(cx: &MeasureContext<'_>) -> Self {
        Self {
            lane: cx.metric(MetricRole::ScrollbarLane),
            thumb: cx.metric(MetricRole::ScrollbarThumb),
            minimum_thumb: cx.metric(MetricRole::ScrollbarMinimumThumb),
        }
    }
}

/// One axis's bar: the track lane, the thumb and the maximum scroll offset, with
/// the dragging arithmetic every scrollbar shares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollbarGeometry {
    /// The pointer lane along the trailing edge.
    pub(crate) track: Rect,
    /// The visible thumb.
    pub(crate) thumb: Rect,
    /// The largest scroll offset; nonpositive when the content fits.
    pub(crate) maximum: f32,
    vertical: bool,
}
impl ScrollbarGeometry {
    /// `point`'s position along the bar's axis.
    fn along(&self, point: Point) -> f32 {
        if self.vertical { point.y() } else { point.x() }
    }
    /// Where a press at `point` grabs the thumb, measured from its start: the
    /// pressed position on the thumb, or its middle for a press elsewhere on the
    /// track, so the thumb then centers under the pointer.
    pub(crate) fn grab_offset(&self, point: Point) -> f32 {
        let along = self.along(point);
        let [x, y, width, height] = self.thumb.values();
        let (start, length) = if self.vertical {
            (y, height)
        } else {
            (x, width)
        };
        if along >= start && along < start + length {
            along - start
        } else {
            length / 2.0
        }
    }
    /// The scroll offset that keeps the thumb position grabbed at `grab` under
    /// `point`, clamped to the scroll range, or `None` when the thumb fills its
    /// track and cannot travel.
    pub(crate) fn offset_for_pointer(&self, point: Point, grab: f32) -> Option<f32> {
        let [x, y, width, height] = self.track.values();
        let (start, travel) = if self.vertical {
            (y, height - self.thumb.height())
        } else {
            (x, width - self.thumb.width())
        };
        (travel > 0.0)
            .then(|| ((self.along(point) - start - grab) / travel).clamp(0.0, 1.0) * self.maximum)
    }
}

/// The bar for one axis.
///
/// `area` is the scrolled region in the coordinate space the caller hit-tests and
/// paints in, `content` the total extent along the axis and `offset` the current
/// scroll position. `reserved` withholds space at the far end for a crossing bar.
/// Callers present a bar only while the content overflows; a nonpositive maximum
/// simply parks the thumb at the start.
pub(crate) fn geometry(
    extents: Extents,
    area: Rect,
    vertical: bool,
    content: f32,
    offset: f32,
    reserved: f32,
) -> Result<ScrollbarGeometry, crate::GeometryError> {
    let [x, y, width, height] = area.values();
    let (length, cross) = if vertical {
        (height, width)
    } else {
        (width, height)
    };
    let track = (length - reserved).max(0.0);
    let thumb = (track * length / content)
        .max(extents.minimum_thumb)
        .min(track);
    let maximum = content - length;
    let start = if maximum > 0.0 {
        offset / maximum * (track - thumb)
    } else {
        0.0
    };
    let lane = cross.min(extents.lane);
    let thickness = cross.min(extents.thumb);
    let near = (cross - extents.thumb).max(0.0);
    let (track_rect, thumb_rect) = if vertical {
        (
            Rect::new(x + width - lane, y, lane, track)?,
            Rect::new(x + near, y + start, thickness, thumb)?,
        )
    } else {
        (
            Rect::new(x, y + height - lane, track, lane)?,
            Rect::new(x + start, y + near, thumb, thickness)?,
        )
    };
    Ok(ScrollbarGeometry {
        track: track_rect,
        thumb: thumb_rect,
        maximum,
        vertical,
    })
}

/// The share of the viewport that Page Up and Page Down scroll, leaving a tenth
/// of the previous view visible for continuity.
pub(crate) const PAGE_STEP_FRACTION: f32 = 0.9;

/// The smallest scroll change along one axis that brings `target..target +
/// length` into `start..start + extent`: nothing when it is already inside,
/// otherwise aligning its near edge. A target longer than the view aligns its
/// start rather than its end.
pub(crate) fn reveal_delta(start: f32, extent: f32, target: f32, length: f32) -> f32 {
    if target < start {
        target - start
    } else {
        (target + length - start - extent)
            .max(0.0)
            .min(target - start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(vertical: bool) -> ScrollbarGeometry {
        // A 100-pixel view over 400 pixels of content, scrolled to 150, inside
        // an area whose origin is not zero.
        let area = Rect::new(20.0, 10.0, 50.0, 100.0).unwrap();
        let area = if vertical {
            area
        } else {
            Rect::new(10.0, 20.0, 100.0, 50.0).unwrap()
        };
        geometry(Extents::STANDARD, area, vertical, 400.0, 150.0, 0.0).unwrap()
    }

    #[test]
    fn a_press_on_the_thumb_keeps_its_position_and_a_track_press_centers_it() {
        for vertical in [true, false] {
            let bar = bar(vertical);
            let [x, y, w, h] = bar.thumb.values();
            let (start, length) = if vertical { (y, h) } else { (x, w) };
            let at = |along: f32| {
                if vertical {
                    Point::new(x, along).unwrap()
                } else {
                    Point::new(along, y).unwrap()
                }
            };
            assert_eq!(bar.grab_offset(at(start + 3.0)), 3.0);
            assert_eq!(bar.grab_offset(at(start + length)), length / 2.0);
            assert_eq!(bar.grab_offset(at(start - 1.0)), length / 2.0);
        }
    }

    #[test]
    fn dragging_keeps_the_grabbed_point_under_the_pointer_and_clamps() {
        for vertical in [true, false] {
            let bar = bar(vertical);
            let [x, y, w, h] = bar.thumb.values();
            let (start, travel_start) = if vertical {
                (y, bar.track.y())
            } else {
                (x, bar.track.x())
            };
            let at = |along: f32| {
                if vertical {
                    Point::new(x + w / 2.0, along).unwrap()
                } else {
                    Point::new(along, y + h / 2.0).unwrap()
                }
            };
            // Holding the pointer where it grabbed reproduces the current offset,
            // whatever the area's origin.
            let offset = bar.offset_for_pointer(at(start + 2.0), 2.0).unwrap();
            assert!((offset - 150.0).abs() < 1e-3, "{offset}");
            assert_eq!(
                bar.offset_for_pointer(at(travel_start - 50.0), 2.0),
                Some(0.0)
            );
            assert_eq!(
                bar.offset_for_pointer(at(travel_start + 1000.0), 2.0),
                Some(bar.maximum)
            );
        }
    }

    #[test]
    fn a_thumb_that_fills_its_track_does_not_travel() {
        let area = Rect::new(0.0, 0.0, 50.0, 100.0).unwrap();
        let bar = geometry(Extents::STANDARD, area, true, 100.0, 0.0, 0.0).unwrap();
        assert_eq!(
            bar.offset_for_pointer(Point::new(0.0, 50.0).unwrap(), 0.0),
            None
        );
    }

    #[test]
    fn reveal_moves_only_as_far_as_needed() {
        // Already visible: no change.
        assert_eq!(reveal_delta(0.0, 100.0, 20.0, 30.0), 0.0);
        // Before the view: align its start.
        assert_eq!(reveal_delta(50.0, 100.0, 20.0, 10.0), -30.0);
        // After the view: align its end.
        assert_eq!(reveal_delta(0.0, 100.0, 120.0, 10.0), 30.0);
        // Longer than the view: align its start, not its end.
        assert_eq!(reveal_delta(0.0, 100.0, 120.0, 300.0), 120.0);
    }
}
