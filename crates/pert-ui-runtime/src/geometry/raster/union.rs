//! Direct coverage for strokes of straight segments.
//!
//! The stroke is the union of its segment bodies, joins and caps. It is
//! decomposed into convex pieces: segment bodies, discs for round caps,
//! circular sectors for round joins, triangles and kites for bevel and miter
//! joins, and boxes for square caps. Each piece contributes one horizontal
//! span per sub-scanline. The spans of a sub-scanline are merged, so a point
//! inside several pieces counts once, and their exact horizontal overlap with
//! each pixel is accumulated as the outline fill does. Coverage therefore
//! follows the same four-sub-scanline model as a fill of the stroke's outline,
//! without building the outline or tracking its active edges: the work is the
//! sum over pieces of the sub-scanlines each one spans, plus sorting the spans
//! of each sub-scanline.
use super::{
    MAX_EDGES, REMOTE_GRID, RasterGrid, SCANLINE_WEIGHT, SCANLINES, Subpath, Vec2, add, cross, len,
    line_x, normal, scale, signed_area, sub, unit,
};
use crate::{DrawError, GeometryError, LineCap, LineJoin, MaskImage, PathError, Rect, StrokeStyle};

// Corners of the largest convex polygon piece: a round join's cone.
const CORNERS: usize = 5;

// A convex piece of the stroke, in the path's own units.
#[derive(Clone, Copy)]
enum Piece {
    Disc {
        center: Vec2,
        radius: f64,
    },
    // Points whose offset from `origin` lies within `half` across the unit
    // `direction` and within `start..=end` along it: a segment body, a square
    // cap's extension of it, or the square of a zero-length subpath.
    Band {
        origin: Vec2,
        direction: Vec2,
        half: f64,
        start: f64,
        end: f64,
        endpoint: Vec2,
        after: f64,
    },
    // A convex polygon with positive signed area: a bevel or miter join.
    Polygon {
        corners: [Vec2; CORNERS],
        count: usize,
    },
    // A disc cut to a convex polygon: a round join's circular sector.
    Sector {
        center: Vec2,
        radius: f64,
        corners: [Vec2; CORNERS],
        count: usize,
    },
}
// The bounds of points, as minimum and maximum corners.
fn extent<'a>(points: impl IntoIterator<Item = &'a Vec2>) -> (Vec2, Vec2) {
    points.into_iter().fold(
        (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(min, max), p| {
            (
                (min.0.min(p.0), min.1.min(p.1)),
                (max.0.max(p.0), max.1.max(p.1)),
            )
        },
    )
}
// Corners in positive orientation, or `None` for a polygon without area.
fn oriented(corners: &[Vec2]) -> Option<([Vec2; CORNERS], usize)> {
    let count = corners.len();
    if count > CORNERS {
        return None;
    }
    let area = signed_area(corners);
    if area == 0.0 || !area.is_finite() {
        return None;
    }
    let mut ordered = [(0.0, 0.0); CORNERS];
    ordered[..count].copy_from_slice(corners);
    if area < 0.0 {
        ordered[..count].reverse();
    }
    Some((ordered, count))
}
impl Piece {
    fn polygon(corners: &[Vec2]) -> Option<Self> {
        oriented(corners).map(|(corners, count)| Self::Polygon { corners, count })
    }
    fn bounds(self) -> (Vec2, Vec2) {
        let disc = |center: Vec2, radius: f64| {
            (
                (center.0 - radius, center.1 - radius),
                (center.0 + radius, center.1 + radius),
            )
        };
        match self {
            Self::Disc { center, radius } => disc(center, radius),
            Self::Band {
                origin,
                direction,
                half,
                start,
                end,
                endpoint,
                after,
            } => {
                let n = scale(normal(direction), half);
                let (a, b) = (
                    add(origin, scale(direction, start)),
                    if origin
                        .0
                        .abs()
                        .max(origin.1.abs())
                        .max(endpoint.0.abs())
                        .max(endpoint.1.abs())
                        > REMOTE_GRID
                    {
                        add(endpoint, scale(direction, after))
                    } else {
                        add(origin, scale(direction, end))
                    },
                );
                extent(&[add(a, n), add(b, n), sub(b, n), sub(a, n)])
            }
            Self::Polygon { corners, count } => extent(&corners[..count]),
            Self::Sector {
                center,
                radius,
                corners,
                count,
            } => {
                let (a, b) = (disc(center, radius), extent(&corners[..count]));
                (
                    (a.0.0.max(b.0.0), a.0.1.max(b.0.1)),
                    (a.1.0.min(b.1.0), a.1.1.min(b.1.1)),
                )
            }
        }
    }
}

// A line `x = offset + slope * y`.
#[derive(Clone, Copy)]
struct Line {
    offset: f64,
    slope: f64,
}
impl Line {
    const LEFTMOST: Self = Self {
        offset: f64::NEG_INFINITY,
        slope: 0.0,
    };
    const RIGHTMOST: Self = Self {
        offset: f64::INFINITY,
        slope: 0.0,
    };
    fn at(self, y: f64) -> f64 {
        self.offset + self.slope * y
    }
}
// The half-planes of a positively oriented convex polygon as bounds on x:
// right of every left line and left of every right line. The inside of edge
// p -> q satisfies (qx - px)(y - py) - (qy - py)(x - px) >= 0, whose side
// follows the edge's vertical direction; horizontal edges bound only y, which
// the piece's rows already do. Only the first `lefts` and `rights` slots
// are used.
#[derive(Clone, Copy)]
struct Bounds {
    left: [Line; CORNERS],
    right: [Line; CORNERS],
    lefts: usize,
    rights: usize,
}
impl Bounds {
    fn new(corners: &[Vec2]) -> Self {
        let mut bounds = Self {
            left: [Line::LEFTMOST; CORNERS],
            right: [Line::RIGHTMOST; CORNERS],
            lefts: 0,
            rights: 0,
        };
        for i in 0..corners.len() {
            let (p, q) = (corners[i], corners[(i + 1) % corners.len()]);
            let rise = q.1 - p.1;
            if rise == 0.0 {
                continue;
            }
            let slope = (q.0 - p.0) / rise;
            let line = Line {
                offset: p.0 - slope * p.1,
                slope,
            };
            if rise > 0.0 {
                bounds.right[bounds.rights] = line;
                bounds.rights += 1;
            } else {
                bounds.left[bounds.lefts] = line;
                bounds.lefts += 1;
            }
        }
        bounds
    }
    fn span(&self, y: f64) -> (f64, f64) {
        (
            self.left[..self.lefts]
                .iter()
                .map(|line| line.at(y))
                .fold(f64::NEG_INFINITY, f64::max),
            self.right[..self.rights]
                .iter()
                .map(|line| line.at(y))
                .fold(f64::INFINITY, f64::min),
        )
    }
}
// A piece prepared in physical pixels, whose horizontal extent at any height
// is found without division.
#[derive(Clone, Copy)]
enum Shape {
    Disc {
        center: Vec2,
        square: f64,
    },
    // Right of both left lines and left of both right lines.
    Band {
        left: [Line; 2],
        right: [Line; 2],
    },
    Polygon(Bounds),
    Sector {
        center: Vec2,
        square: f64,
        bounds: Bounds,
    },
}
impl Shape {
    // Prepares a piece through `local`, which scales by `factor` and translates.
    fn new(
        piece: Piece,
        local: impl Fn(Vec2) -> Vec2,
        factor: f64,
        grid: Option<&RasterGrid>,
    ) -> Self {
        let mapped = |corners: [Vec2; CORNERS], count: usize| {
            let mut points = [(0.0, 0.0); CORNERS];
            for (point, &corner) in points.iter_mut().zip(&corners[..count]) {
                *point = local(corner);
            }
            Bounds::new(&points[..count])
        };
        match piece {
            Piece::Disc { center, radius } => Self::Disc {
                center: local(center),
                square: (radius * factor).powi(2),
            },
            Piece::Band {
                origin,
                direction: (dx, dy),
                half,
                start,
                end,
                endpoint,
                after,
            } => {
                let (ox, oy) = local(origin);
                // A remote endpoint can lose its intercept even when another
                // segment keeps the overall grid minimum near the viewport.
                let stable_grid = grid.filter(|g| {
                    g.remote
                        || origin.0.abs().max(origin.1.abs()) > REMOTE_GRID
                        || endpoint.0.abs().max(endpoint.1.abs()) > REMOTE_GRID
                });
                let anchor = stable_grid.map(|g| if g.remote { g.origin } else { g.min });
                let (half, start, end) = (half * factor, start * factor, end * factor);
                // Across the band: |dx * (y - oy) - dy * (x - ox)| <= half.
                let across = if dy == 0.0 {
                    [Line::LEFTMOST, Line::RIGHTMOST]
                } else {
                    let slope = dx / dy;
                    let width = (half / dy).abs();
                    let offset = if let Some(anchor) = anchor {
                        (line_x(origin, endpoint, anchor.1) - anchor.0) * factor
                    } else {
                        ox - slope * oy
                    };
                    [
                        Line {
                            offset: offset - width,
                            slope,
                        },
                        Line {
                            offset: offset + width,
                            slope,
                        },
                    ]
                };
                // Along the band: start <= dx * (x - ox) + dy * (y - oy) <= end.
                let along = if dx == 0.0 {
                    [Line::LEFTMOST, Line::RIGHTMOST]
                } else {
                    let slope = -dy / dx;
                    let (a, b) = if let Some(anchor) = anchor {
                        let boundary = |point: Vec2, extension: f64| {
                            let point = add(point, scale((dx, dy), extension / factor));
                            ((point.0 + slope * (anchor.1 - point.1)) - anchor.0) * factor
                        };
                        (boundary(origin, start), boundary(endpoint, after * factor))
                    } else {
                        (ox + start / dx - slope * oy, ox + end / dx - slope * oy)
                    };
                    [
                        Line {
                            offset: a.min(b),
                            slope,
                        },
                        Line {
                            offset: a.max(b),
                            slope,
                        },
                    ]
                };
                Self::Band {
                    left: [across[0], along[0]],
                    right: [across[1], along[1]],
                }
            }
            Piece::Polygon { corners, count } => Self::Polygon(mapped(corners, count)),
            Piece::Sector {
                center,
                radius,
                corners,
                count,
            } => Self::Sector {
                center: local(center),
                square: (radius * factor).powi(2),
                bounds: mapped(corners, count),
            },
        }
    }
    // The horizontal extent at height `y`, empty unless left < right.
    fn span(&self, y: f64) -> (f64, f64) {
        // A row outside the circle, also by rounding at its top or bottom, is
        // empty; returning NaN instead would let `max`/`min` in a sector
        // silently fall back to the cone's span.
        let disc = |center: Vec2, square: f64| {
            let dy = y - center.1;
            let reach = square - dy * dy;
            if reach.is_nan() || reach < 0.0 {
                return (f64::INFINITY, f64::NEG_INFINITY);
            }
            let dx = reach.sqrt();
            (center.0 - dx, center.0 + dx)
        };
        match self {
            Self::Disc { center, square } => disc(*center, *square),
            Self::Band { left, right } => (
                left[0].at(y).max(left[1].at(y)),
                right[0].at(y).min(right[1].at(y)),
            ),
            Self::Polygon(bounds) => bounds.span(y),
            Self::Sector {
                center,
                square,
                bounds,
            } => {
                let (a, b) = (disc(*center, *square), bounds.span(y));
                (a.0.max(b.0), a.1.min(b.1))
            }
        }
    }
}

// Sub-scanline `i` lies at `(i + 0.5) / SCANLINES` of its pixel row. These find,
// for a height in sub-scanlines, the first sub-scanline at or after it (at most
// `limit`) and the last at or before it. `f64::ceil` and `f64::floor` are
// library calls on baseline x86-64 and were a large share of the per-row cost;
// one truncating conversion of a clamped, non-negative value is not.
fn first_scanline(position: f64, limit: usize) -> usize {
    let value = position - 0.5;
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    let value = value.min(limit as f64);
    let truncated = value as usize;
    truncated + usize::from((truncated as f64) < value)
}
fn last_scanline(position: f64, limit: usize) -> Option<usize> {
    let value = position - 0.5;
    (value >= 0.0).then(|| value.min(limit as f64) as usize)
}

// Adds one sub-scanline's span `a..b` on the unclipped grid, already clamped
// to the columns `offset..offset + row.len()` that `row` holds, at
// `SCANLINE_WEIGHT` of a pixel's height, with exact horizontal overlap. Working
// on the unclipped grid keeps coverage bit-identical for any clip that leaves
// the same pixels visible.
fn accumulate(row: &mut [f64], offset: usize, a: f64, b: f64) {
    if b <= a {
        return;
    }
    // Non-negative, so truncation is the floor.
    let (first, last) = (a as usize, b as usize);
    if first == last {
        row[first - offset] += (b - a) * SCANLINE_WEIGHT;
        return;
    }
    row[first - offset] += (first as f64 + 1.0 - a) * SCANLINE_WEIGHT;
    for value in &mut row[first + 1 - offset..last - offset] {
        *value += SCANLINE_WEIGHT;
    }
    if last - offset < row.len() {
        row[last - offset] += (b - last as f64) * SCANLINE_WEIGHT;
    }
}

// Whether the direct path draws this stroke: straight segments, any join.
pub(super) fn eligible(path: &crate::Path) -> bool {
    path.elements().iter().all(|element| {
        matches!(
            element,
            crate::PathElement::Move(_) | crate::PathElement::Line(_) | crate::PathElement::Close
        )
    })
}

// The join between two unit directions at a vertex, as the outlining stroker
// builds it, on the side the path turns away from: the circular sector between
// both offset corners for a round join, the triangle to both offset corners for
// a bevel, and the kite to the miter tip when the miter stays within the limit.
// A straight continuation has no join.
fn join(at: Vec2, incoming: Vec2, outgoing: Vec2, half: f64, style: StrokeStyle) -> Option<Piece> {
    let (n0, n1) = (normal(incoming), normal(outgoing));
    let side = if cross(incoming, outgoing) > 0.0 {
        -1.0
    } else {
        1.0
    };
    let (from, to) = (scale(n0, side), scale(n1, side));
    let a = add(at, scale(from, half));
    let b = add(at, scale(to, half));
    match style.join() {
        LineJoin::Round => {
            // The outer corner lies toward `incoming - outgoing`, also for a
            // reversal; the cone from `at` between both offset directions,
            // truncated beyond the radius, cuts the disc to that sector.
            let middle = unit(sub(incoming, outgoing))?;
            let reach = 2.0 * half;
            let (corners, count) = oriented(&[
                at,
                add(at, scale(from, reach)),
                add(add(at, scale(from, reach)), scale(middle, reach)),
                add(add(at, scale(to, reach)), scale(middle, reach)),
                add(at, scale(to, reach)),
            ])?;
            Some(Piece::Sector {
                center: at,
                radius: half,
                corners,
                count,
            })
        }
        LineJoin::Bevel => Piece::polygon(&[at, a, b]),
        LineJoin::Miter => {
            if let Some(m) = unit(add(n0, n1)) {
                let cos_half = m.0 * n0.0 + m.1 * n0.1;
                if cos_half > 0.0 && 1.0 / cos_half <= f64::from(style.miter_limit()) {
                    let tip = add(at, scale(m, side * half / cos_half));
                    return Piece::polygon(&[at, a, tip, b]);
                }
            }
            Piece::polygon(&[at, a, b])
        }
    }
}

// Decomposes flattened straight subpaths into convex pieces, with the same
// treatment of repeated points, closure and zero-length subpaths as the
// outlining stroker.
fn pieces(subpaths: &[Subpath], half: f64, style: StrokeStyle) -> Result<Vec<Piece>, PathError> {
    let cap = style.cap();
    let mut pieces = Vec::new();
    let disc = |center| Piece::Disc {
        center,
        radius: half,
    };
    for subpath in subpaths {
        let mut points: Vec<Vec2> = Vec::with_capacity(subpath.points.len());
        for &p in &subpath.points {
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
        if subpath.closed && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if let [p] = points[..] {
            match cap {
                LineCap::Butt => {}
                LineCap::Round => pieces.push(disc(p)),
                LineCap::Square => pieces.push(Piece::Band {
                    origin: p,
                    direction: (1.0, 0.0),
                    half,
                    start: -half,
                    end: half,
                    endpoint: p,
                    after: half,
                }),
            }
            continue;
        }
        let closed = subpath.closed && points.len() > 2;
        let total = points.len();
        let count = if closed { total } else { total - 1 };
        let square = !closed && cap == LineCap::Square;
        for i in 0..count {
            let (a, b) = (points[i], points[(i + 1) % total]);
            let Some(direction) = unit(sub(b, a)) else {
                continue;
            };
            // A square cap extends the first and last body by half the width.
            let before = if square && i == 0 { half } else { 0.0 };
            let after = if square && i + 1 == count { half } else { 0.0 };
            pieces.push(Piece::Band {
                origin: a,
                direction,
                half,
                start: -before,
                end: len(sub(b, a)) + after,
                endpoint: b,
                after,
            });
        }
        // Joins at interior vertices, every vertex of a closed path.
        let joins = if closed { 0..total } else { 1..total - 1 };
        for i in joins {
            let (before, at, after) = (
                points[(i + total - 1) % total],
                points[i],
                points[(i + 1) % total],
            );
            if let (Some(incoming), Some(outgoing)) = (unit(sub(at, before)), unit(sub(after, at)))
            {
                pieces.extend(join(at, incoming, outgoing, half, style));
            }
        }
        if !closed && cap == LineCap::Round {
            pieces.push(disc(points[0]));
            pieces.push(disc(points[total - 1]));
        }
        if pieces.len().saturating_mul(CORNERS) > MAX_EDGES {
            return Err(PathError::TooLarge);
        }
    }
    Ok(pieces)
}

// The bounds of all pieces, or an error when one is not finite.
fn bounds(pieces: &[Piece]) -> Result<(Vec2, Vec2), DrawError> {
    let mut min = (f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for piece in pieces {
        let (low, high) = piece.bounds();
        if !low.0.is_finite() || !low.1.is_finite() || !high.0.is_finite() || !high.1.is_finite() {
            return Err(GeometryError::NonFinite.into());
        }
        min = (min.0.min(low.0), min.1.min(low.1));
        max = (max.0.max(high.0), max.1.max(high.1));
    }
    Ok((min, max))
}

/// Rasterizes an eligible stroke on a physical grid anchored at its bounds,
/// cropped to the local clip, as the outline fill does.
pub(super) fn rasterize(
    subpaths: &[Subpath],
    style: StrokeStyle,
    factor: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let pieces = pieces(subpaths, f64::from(style.width().get()) / 2.0, style)?;
    let (min, max) = bounds(&pieces)?;
    if !(max.0 > min.0 && max.1 > min.1) {
        return Ok(None);
    }
    cover(&pieces, min, max, factor, clip)
}

// One piece's extent on one sub-scanline, in physical pixels of the mask.
#[derive(Clone, Copy)]
struct Span {
    scanline: usize,
    left: f64,
    right: f64,
}

// Covers pieces on the physical grid anchored at `min`, spanning to `max`.
fn cover(
    pieces: &[Piece],
    min: Vec2,
    max: Vec2,
    factor: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let Some(grid) = RasterGrid::new(min, max, factor, clip)? else {
        return Ok(None);
    };
    let (width, height) = (grid.width, grid.height);
    let (x0, y0) = grid.offset;
    let x1 = x0 + width as f64;
    let local = |p: Vec2| grid.on_grid(p);
    let scanlines = SCANLINES as usize;
    let (left_edge, right_edge) = (x0, x1);
    let (left, top) = (x0 as usize, y0 as usize * scanlines);
    let last = top + height * scanlines - 1;
    // The sub-scanlines each piece meets inside the mask.
    let reach = |piece: &Piece| {
        let (low, high) = piece.bounds();
        let (low, high) = (local(low), local(high));
        let first = first_scanline(low.1 * f64::from(SCANLINES), last + 1).max(top);
        last_scanline(high.1 * f64::from(SCANLINES), last).map(|end| first..=end)
    };
    // One step and at most one span per piece and sub-scanline, counted
    // before any span is stored.
    let mut work = 0usize;
    for piece in pieces {
        if let Some(range) = reach(piece) {
            work = work.saturating_add(range.count());
        }
    }
    if work > crate::VectorRasterizer::MAX_COVERAGE_WORK {
        return Err(DrawError::TooComplex);
    }
    // Every piece's span on every sub-scanline it meets, clamped to the mask.
    let mut spans = Vec::with_capacity(work);
    for &piece in pieces {
        let shape = Shape::new(piece, local, factor, Some(&grid));
        let Some(range) = reach(&piece) else {
            continue;
        };
        for scanline in range {
            let (a, b) = shape.span((scanline as f64 + 0.5) * SCANLINE_WEIGHT);
            let (a, b) = (a.max(left_edge), b.min(right_edge));
            if a < b {
                spans.push(Span {
                    scanline: scanline - top,
                    left: a,
                    right: b,
                });
            }
        }
    }
    if spans.is_empty() {
        return Ok(None);
    }
    // Order the spans by sub-scanline, then by position. Sorting the spans
    // themselves keeps the memory proportional to the bounded work rather
    // than to the mask's height, which a tall narrow mask makes large.
    spans.sort_unstable_by(|a, b| a.scanline.cmp(&b.scanline).then(a.left.total_cmp(&b.left)));
    let mut pixels = vec![0u8; width * height];
    let mut row = vec![0.0f64; width];
    let mut any = false;
    let mut next = 0;
    for y in 0..height {
        let below = (y + 1) * scanlines;
        while let Some(&Span { scanline, .. }) =
            spans.get(next).filter(|span| span.scanline < below)
        {
            // Merge overlapping spans, so the union counts each point once.
            let mut current: Option<(f64, f64)> = None;
            while let Some(&Span {
                left: start,
                right: end,
                ..
            }) = spans.get(next).filter(|span| span.scanline == scanline)
            {
                next += 1;
                current = match current {
                    Some((a, b)) if start <= b => Some((a, b.max(end))),
                    Some((a, b)) => {
                        accumulate(&mut row, left, a, b);
                        Some((start, end))
                    }
                    None => Some((start, end)),
                };
            }
            if let Some((a, b)) = current {
                accumulate(&mut row, left, a, b);
            }
        }
        for (pixel, value) in pixels[y * width..(y + 1) * width]
            .iter_mut()
            .zip(row.iter_mut())
        {
            if *value > 0.0 {
                // Non-negative, so adding a half and truncating rounds.
                let coverage = (value.min(1.0) * 255.0 + 0.5) as u8;
                any |= coverage > 0;
                *pixel = coverage;
                *value = 0.0;
            }
        }
    }
    if !any {
        return Ok(None);
    }
    let rect = grid.rect()?;
    let mask = MaskImage::new(width as u32, height as u32, pixels)?;
    Ok(Some((mask, rect)))
}

// The pieces' bounds, so a test can anchor the outline fill and this path on
// one grid covering both.
#[cfg(test)]
pub(super) fn piece_bounds(
    subpaths: &[Subpath],
    style: StrokeStyle,
) -> Result<(Vec2, Vec2), DrawError> {
    let half = f64::from(style.width().get()) / 2.0;
    bounds(&pieces(subpaths, half, style)?)
}

// Rasterizes an eligible stroke on a grid anchored at a caller's bounds.
#[cfg(test)]
pub(super) fn rasterize_anchored(
    subpaths: &[Subpath],
    style: StrokeStyle,
    bounds: (Vec2, Vec2),
    factor: f64,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let half = f64::from(style.width().get()) / 2.0;
    cover(
        &pieces(subpaths, half, style)?,
        bounds.0,
        bounds.1,
        factor,
        None,
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn remote_strokes_keep_visible_endpoints_and_crop_grid_coverage()
    -> Result<(), Box<dyn std::error::Error>> {
        let cases = [
            (vec![(10.0, -1e20), (10.0, 200.0)], (10.0, 50.0)),
            (vec![(-1e20, 10.0), (200.0, 10.0)], (50.0, 10.0)),
            (vec![(-1e20, -1e20), (50.0, 60.0)], (20.0, 30.0)),
            (vec![(-1e20, -3e20), (1e20, 3e20)], (10.0, 30.0)),
        ];
        for factor in [1.0, 1.25, 1.5, 8.0] {
            for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
                for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
                    let style = StrokeStyle::new(crate::Length::new(2.0)?)
                        .with_cap(cap)
                        .with_join(join);
                    for (points, probe) in &cases {
                        let subpaths = [Subpath {
                            points: points.clone(),
                            tangents: vec![[None, None]; points.len()],
                            closed: false,
                        }];
                        let clip = Rect::new(0.3, 0.7, 64.0, 64.0)?;
                        let (mask, rect) = rasterize(&subpaths, style, factor, Some(clip))?
                            .ok_or("visible remote stroke")?;
                        let x = ((probe.0 - f64::from(rect.x())) * factor).floor() as usize;
                        let y = ((probe.1 - f64::from(rect.y())) * factor).floor() as usize;
                        assert!(
                            x < mask.width() as usize && y < mask.height() as usize,
                            "{points:?} {factor} {rect:?}"
                        );
                        assert!(
                            mask.pixels()[y * mask.width() as usize + x] > 0,
                            "{points:?} {factor} {cap:?} {join:?}"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn opposing_remote_steep_endpoints_match_near_geometry_on_the_same_grid()
    -> Result<(), Box<dyn std::error::Error>> {
        let bounds = ((-1e20, -3e20), (1e20, 3e20));
        let style = StrokeStyle::new(crate::Length::new(2.0)?);
        for factor in [1.0, 1.25, 1.5, 8.0] {
            let render = |points: Vec<Vec2>| {
                let subpaths = [Subpath {
                    tangents: vec![[None, None]; points.len()],
                    points,
                    closed: false,
                }];
                cover(
                    &pieces(&subpaths, 1.0, style)?,
                    bounds.0,
                    bounds.1,
                    factor,
                    Some(Rect::new(0.3, 0.7, 64.0, 64.0)?),
                )
            };
            let (remote, remote_rect) = render(vec![bounds.0, bounds.1])?.ok_or("remote line")?;
            // Both lines are independently known to lie on x=y/3; the nearby
            // endpoints avoid cancellation and all endpoints lie outside the clip.
            let (near, near_rect) =
                render(vec![(-100.0, -300.0), (100.0, 300.0)])?.ok_or("near line")?;
            assert_eq!(remote_rect, near_rect);
            assert_eq!(remote.pixels().len(), near.pixels().len());
            for (&actual, &expected) in remote.pixels().iter().zip(near.pixels()) {
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "scale {factor}: {actual} vs {expected}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn remote_crop_changes_preserve_overlapping_coverage_bytes()
    -> Result<(), Box<dyn std::error::Error>> {
        let subpaths = [Subpath {
            points: vec![(-1e20, -1e20), (50.0, 60.0)],
            tangents: vec![[None, None]; 2],
            closed: false,
        }];
        let style = StrokeStyle::new(crate::Length::new(2.0)?).with_cap(LineCap::Round);
        for factor in [1.0, 1.25, 1.5, 8.0] {
            let (outer, outer_rect) = rasterize(
                &subpaths,
                style,
                factor,
                Some(Rect::new(0.3, 0.7, 64.0, 64.0)?),
            )?
            .ok_or("outer coverage")?;
            let (inner, inner_rect) = rasterize(
                &subpaths,
                style,
                factor,
                Some(Rect::new(7.2, 11.4, 32.0, 32.0)?),
            )?
            .ok_or("inner coverage")?;
            let x = (f64::from(inner_rect.x() - outer_rect.x()) * factor).round() as usize;
            let y = (f64::from(inner_rect.y() - outer_rect.y()) * factor).round() as usize;
            for row in 0..inner.height() as usize {
                let begin = (y + row) * outer.width() as usize + x;
                let source = row * inner.width() as usize;
                assert_eq!(
                    &outer.pixels()[begin..begin + inner.width() as usize],
                    &inner.pixels()[source..source + inner.width() as usize],
                    "{factor} row {row}",
                );
            }
        }
        Ok(())
    }

    #[test]
    fn remote_end_caps_and_real_corners_match_a_nearby_integer_grid_reference()
    -> Result<(), Box<dyn std::error::Error>> {
        // Integer scales preserve the same grid phase for both starting bounds.
        // The bend and final endpoint are visible, so joins and caps contribute
        // actual coverage rather than merely selecting different style values.
        for factor in [1.0, 8.0] {
            for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
                for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
                    let style = StrokeStyle::new(crate::Length::new(2.0)?)
                        .with_cap(cap)
                        .with_join(join);
                    let render = |start| {
                        let subpaths = [Subpath {
                            points: vec![(20.0, start), (20.0, 20.0), (40.0, 20.0)],
                            tangents: vec![[None, None]; 3],
                            closed: false,
                        }];
                        rasterize(
                            &subpaths,
                            style,
                            factor,
                            Some(Rect::new(0.3, 0.7, 64.0, 64.0)?),
                        )
                    };
                    let (remote, remote_rect) = render(-1e20)?.ok_or("remote coverage")?;
                    let (near, near_rect) = render(-100.0)?.ok_or("reference coverage")?;
                    assert_eq!(remote_rect, near_rect, "{factor} {cap:?} {join:?}");
                    assert_eq!(remote.pixels(), near.pixels(), "{factor} {cap:?} {join:?}");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn remote_vertical_bounds_cover_small_and_large_viewports_to_the_last_row()
    -> Result<(), Box<dyn std::error::Error>> {
        let subpaths = [Subpath {
            points: vec![(1.0, -1e20), (1.0, 100000.0)],
            tangents: vec![[None, None]; 2],
            closed: false,
        }];
        let style = StrokeStyle::new(crate::Length::new(1.0)?);
        for height in [100.0, 100000.0] {
            let (mask, rect) = rasterize(
                &subpaths,
                style,
                1.0,
                Some(Rect::new(0.0, 0.0, 10.0, height)?),
            )?
            .ok_or("visible stroke")?;
            assert_eq!(rect.y(), 0.0);
            assert_eq!(mask.height(), height as u32);
            assert!(
                mask.pixels()[..mask.width() as usize]
                    .iter()
                    .any(|p| *p > 0)
            );
            assert!(
                mask.pixels()[mask.pixels().len() - mask.width() as usize..]
                    .iter()
                    .any(|p| *p > 0)
            );
        }
        Ok(())
    }

    use super::*;

    // A sub-scanline just outside a round join's circle is empty, even though
    // the sector's cone reaches beyond it: the cone must not stand in for a
    // circle row that rounding put out of reach.
    #[test]
    fn a_row_outside_a_sector_circle_is_empty() -> Result<(), Box<dyn std::error::Error>> {
        let style = StrokeStyle::new(crate::Length::new(2.0)?).with_join(LineJoin::Round);
        let piece = join((10.0, 10.0), (1.0, 0.0), (0.0, 1.0), 1.0, style)
            .ok_or("a right-angle turn has a join")?;
        let shape = Shape::new(piece, |p| p, 1.0, None);
        let (left, right) = shape.span(10.0 - 1.0 - 1.0e-9);
        assert!(
            left.partial_cmp(&right)
                .is_none_or(std::cmp::Ordering::is_gt)
        );
        let (left, right) = shape.span(9.5);
        assert!(left < right);
        Ok(())
    }
}
