//! Private curve flattening, for fills and strokes, and the scanline
//! rasterization of fills that `VectorRasterizer` uses.
use super::{FillRule, Path, PathError, Verb};
use crate::{DrawError, MaskImage, Point, Rect};

// Largest deviation of flattened curves from the exact curve, in physical pixels.
const TOLERANCE: f64 = 0.2;
// Budget for flattened points of one recorded primitive, published as
// `VectorRasterizer::MAX_FLATTENED_POINTS`.
pub(crate) const MAX_EDGES: usize = 1 << 20;
// Sub-scanlines per pixel row; each adds its span at this share of a pixel's height.
const SCANLINES: u32 = 4;
const SCANLINE_WEIGHT: f64 = 1.0 / SCANLINES as f64;

type Vec2 = (f64, f64);

struct Subpath {
    points: Vec<Vec2>,
    // Exact incoming and outgoing tangents at path vertices; `None` inside a
    // flattened curve, where the chords themselves are the best direction.
    tangents: Vec<[Option<Vec2>; 2]>,
    closed: bool,
}

// The first nonzero difference, as the exact tangent at a segment's end.
fn tangent(differences: &[Vec2]) -> Option<Vec2> {
    differences.iter().copied().find(|d| *d != (0.0, 0.0))
}

struct Flattener {
    subpaths: Vec<Subpath>,
    current: Option<Subpath>,
    total: usize,
}
impl Flattener {
    fn push(&mut self, p: Vec2, incoming: Option<Vec2>) -> Result<(), PathError> {
        self.total += 1;
        if self.total > MAX_EDGES {
            return Err(PathError::TooLarge);
        }
        if let Some(subpath) = &mut self.current {
            subpath.points.push(p);
            subpath.tangents.push([incoming, None]);
        }
        Ok(())
    }
    // Records the exact tangent with which a segment leaves the current point.
    fn begin(&mut self, outgoing: Option<Vec2>) {
        if let Some(last) = self
            .current
            .as_mut()
            .and_then(|subpath| subpath.tangents.last_mut())
        {
            last[1] = outgoing;
        }
    }
    fn finish(&mut self) {
        if let Some(done) = self.current.take()
            && done.points.len() > 1
        {
            self.subpaths.push(done);
        }
    }
}

fn flatten(path: &Path, tolerance: f64) -> Result<Vec<Subpath>, PathError> {
    let mut f = Flattener {
        subpaths: Vec::new(),
        current: None,
        total: 0,
    };
    let mut last = (0.0, 0.0);
    let v = |p: Point| (f64::from(p.x()), f64::from(p.y()));
    for verb in path.verbs.iter() {
        match *verb {
            Verb::Move(p) => {
                f.finish();
                last = v(p);
                f.total += 1;
                if f.total > MAX_EDGES {
                    return Err(PathError::TooLarge);
                }
                f.current = Some(Subpath {
                    points: vec![last],
                    tangents: vec![[None, None]],
                    closed: false,
                });
            }
            Verb::Line(p) => {
                let p = v(p);
                let d = tangent(&[sub(p, last)]);
                f.begin(d);
                f.push(p, d)?;
                last = p;
            }
            Verb::Quad(c, p) => {
                let (p0, p1, p2) = (last, v(c), v(p));
                f.begin(tangent(&[sub(p1, p0), sub(p2, p0)]));
                let dd = len(sub(add(p0, p2), scale(p1, 2.0)));
                let n = divisions((dd / (4.0 * tolerance)).sqrt(), MAX_EDGES - f.total)?;
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    let u = 1.0 - t;
                    let end = if i == n {
                        tangent(&[sub(p2, p1), sub(p2, p0)])
                    } else {
                        None
                    };
                    f.push(
                        add(
                            add(scale(p0, u * u), scale(p1, 2.0 * u * t)),
                            scale(p2, t * t),
                        ),
                        end,
                    )?;
                }
                last = p2;
            }
            Verb::Cubic(c1, c2, p) => {
                let (p0, p1, p2, p3) = (last, v(c1), v(c2), v(p));
                f.begin(tangent(&[sub(p1, p0), sub(p2, p0), sub(p3, p0)]));
                let dd = len(sub(add(p0, p2), scale(p1, 2.0)))
                    .max(len(sub(add(p1, p3), scale(p2, 2.0))));
                let n = divisions((0.75 * dd / tolerance).sqrt(), MAX_EDGES - f.total)?;
                for i in 1..=n {
                    let t = i as f64 / n as f64;
                    let u = 1.0 - t;
                    let end = if i == n {
                        tangent(&[sub(p3, p2), sub(p3, p1), sub(p3, p0)])
                    } else {
                        None
                    };
                    f.push(
                        add(
                            add(scale(p0, u * u * u), scale(p1, 3.0 * u * u * t)),
                            add(scale(p2, 3.0 * u * t * t), scale(p3, t * t * t)),
                        ),
                        end,
                    )?;
                }
                last = p3;
            }
            Verb::Close => {
                let Some(start) = f.current.as_ref().map(|subpath| subpath.points[0]) else {
                    continue;
                };
                if f.current
                    .as_ref()
                    .is_some_and(|subpath| subpath.points.len() == 1)
                {
                    // A close is a segment, so `M p Z` is a zero-length subpath.
                    f.push(start, None)?;
                } else if last != start {
                    let closing = tangent(&[sub(start, last)]);
                    f.begin(closing);
                    f.push(start, closing)?;
                }
                if let Some(mut done) = f.current.take() {
                    // The closing point coincides with the start: the start vertex
                    // takes its incoming tangent, and the duplicate is dropped.
                    if done.points.len() > 2
                        && let Some(end) = done.tangents.pop()
                    {
                        done.points.pop();
                        done.tangents[0][0] = end[0];
                    }
                    done.closed = true;
                    f.subpaths.push(done);
                }
                last = start;
            }
        }
    }
    f.finish();
    Ok(f.subpaths)
}

/// A subpath as the stroke coverage reads it: its points in the path's own
/// units, for each whether it lies inside a flattened curve, where the
/// stroke's edge is smooth and the join is round whatever the style, and
/// whether the subpath is closed.
pub(crate) struct StrokeSubpath {
    pub(crate) points: Vec<Vec2>,
    pub(crate) smooth: Vec<bool>,
    pub(crate) closed: bool,
}

/// The chord tolerance of flattened curves, in physical pixels.
pub(crate) const CHORD_TOLERANCE: f64 = TOLERANCE;

/// The subpaths of a path for the stroke coverage, with curves flattened
/// within `tolerance` in the path's units.
///
/// A curve's first and last chord do not have its tangent's direction at
/// the path vertex, which would turn a cap or a join there by an angle that
/// matters for a wide stroke. Each such vertex therefore gets a neighbor
/// along the exact tangent, so that the segment at the vertex has the
/// tangent's direction and the cap or join there follows the tangent; the
/// neighbor itself is smooth. It lies 0.35 of the chord's length from the
/// vertex, or nearer where that would take it farther than half the
/// tolerance from the chord: the curve stays within the tolerance of its
/// chord, but a tangent may leave the chord at any angle, as where a
/// control point almost coincides with the vertex.
pub(crate) fn stroke_subpaths(
    path: &Path,
    tolerance: f64,
) -> Result<Vec<StrokeSubpath>, PathError> {
    Ok(flatten(path, tolerance)?
        .into_iter()
        .map(|subpath| {
            let count = subpath.points.len();
            let mut points = Vec::with_capacity(count + 2);
            let mut smooth = Vec::with_capacity(count + 2);
            // A neighbor of `at` along `tangent`, toward `other` for
            // `sign` 1 and away from it for -1, unless the chord already
            // runs that way, as a line's does, up to the rounding of the
            // tangent's normalization.
            let neighbor = |at: Vec2, other: Vec2, tangent: Option<Vec2>, sign: f64| {
                let direction = scale(tangent.and_then(unit)?, sign);
                let chord = sub(other, at);
                let length = len(chord);
                let across = cross(chord, direction).abs();
                let along = chord.0 * direction.0 + chord.1 * direction.1;
                if across <= 1e-9 * length && along > 0.0 {
                    return None;
                }
                // Half the tolerance from the chord at most. Where the
                // tangent points away from the chord the curve turns back
                // within it: that turn is the path's own, joined in the
                // stroke's style, not a smooth vertex.
                let step = (0.35 * length).min(0.5 * tolerance * length / across);
                Some((add(at, scale(direction, step)), along >= 0.0))
            };
            for (i, (&p, tangents)) in subpath.points.iter().zip(&subpath.tangents).enumerate() {
                let previous = if i > 0 {
                    Some(subpath.points[i - 1])
                } else if subpath.closed {
                    subpath.points.last().copied()
                } else {
                    None
                };
                let next = if i + 1 < count {
                    Some(subpath.points[i + 1])
                } else if subpath.closed {
                    subpath.points.first().copied()
                } else {
                    None
                };
                if let Some((before, inside)) =
                    previous.and_then(|q| neighbor(p, q, tangents[0], -1.0))
                {
                    points.push(before);
                    smooth.push(inside);
                }
                points.push(p);
                smooth.push(*tangents == [None, None]);
                if let Some((after, inside)) = next.and_then(|q| neighbor(p, q, tangents[1], 1.0)) {
                    points.push(after);
                    smooth.push(inside);
                }
            }
            StrokeSubpath {
                points,
                smooth,
                closed: subpath.closed,
            }
        })
        .collect())
}

fn divisions(n: f64, remaining: usize) -> Result<usize, PathError> {
    let count = n.ceil().max(1.0);
    if !n.is_finite() || count > remaining as f64 {
        return Err(PathError::TooLarge);
    }
    Ok(count as usize)
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    (a.0 + b.0, a.1 + b.1)
}
fn sub(a: Vec2, b: Vec2) -> Vec2 {
    (a.0 - b.0, a.1 - b.1)
}
fn scale(a: Vec2, s: f64) -> Vec2 {
    (a.0 * s, a.1 * s)
}
fn len(a: Vec2) -> f64 {
    a.0.hypot(a.1)
}
fn cross(a: Vec2, b: Vec2) -> f64 {
    a.0 * b.1 - a.1 * b.0
}
fn unit(a: Vec2) -> Option<Vec2> {
    let length = len(a);
    (length > 0.0 && length.is_finite()).then(|| scale(a, 1.0 / length))
}

struct Edge {
    top: f64,
    bottom: f64,
    x: f64,
    slope: f64,
    anchor_y: f64,
    winding: i32,
}

// Beyond this offset, deriving a small cropped extent from two grid positions
// loses useful coverage precision. Ordinary grids retain their original arithmetic.
const REMOTE_GRID: f64 = 1_099_511_627_776.0;

// Evaluate a line near its visible endpoint. If both endpoints are remote,
// retain cancellation in the determinant with compensated products.
pub(crate) fn line_x(a: Vec2, b: Vec2, y: f64) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let near = if (a.1 - y).abs() < (b.1 - y).abs() {
        a
    } else {
        b
    };
    if near.0.abs().max(near.1.abs()) < REMOTE_GRID {
        (dx / dy).mul_add(y - near.1, near.0)
    } else {
        let product = a.1 * b.0;
        let error = a.1.mul_add(b.0, -product);
        let determinant = a.0.mul_add(b.1, -product) - error;
        dx.mul_add(y, determinant) / dy
    }
}

struct RasterGrid {
    width: usize,
    height: usize,
    origin: Vec2,
    offset: Vec2,
    min: Vec2,
    factor: f64,
    remote: bool,
}
impl RasterGrid {
    fn new(
        min: Vec2,
        max: Vec2,
        factor: f64,
        clip: Option<Rect>,
        edge: Vec2,
    ) -> Result<Option<Self>, DrawError> {
        let full = (
            ((max.0 - min.0) * factor).ceil(),
            ((max.1 - min.1) * factor).ceil(),
        );
        let (mut x0, mut y0, mut x1, mut y1) = (0.0, 0.0, full.0, full.1);
        if let Some(clip) = clip {
            let [cx, cy, cw, ch] = clip.values().map(f64::from);
            x0 = ((cx - min.0) * factor).floor().max(0.0);
            y0 = ((cy - min.1) * factor).floor().max(0.0);
            x1 = ((cx + cw - min.0) * factor).ceil().min(full.0);
            y1 = ((cy + ch - min.1) * factor).ceil().min(full.1);
        }
        // Ordinary offsets stay below 2^40 and usize::MAX / 16. With four
        // scanlines per row and at most MaskImage::MAX_PIXELS (16 Mi) rows,
        // offset-to-index conversion and the cropped row end fit usize even
        // on 32-bit targets. Remote grids use zero-based cropped indices.
        let safe_offset = REMOTE_GRID.min((usize::MAX / 16) as f64);
        let remote = x0 > safe_offset || y0 > safe_offset;
        let (origin, offset, width, height) = if remote {
            let Some(clip) = clip else {
                return Err(DrawError::CoverageTooLarge);
            };
            let [cx, cy, cw, ch] = clip.values().map(f64::from);
            let low = (min.0.max(cx), min.1.max(cy));
            let high = (max.0.min(cx + cw), max.1.min(cy + ch));
            if !(high.0 > low.0 && high.1 > low.1) {
                return Ok(None);
            }
            // So far from the anchor its own position holds no fraction of a
            // pixel, so the grid's edges are taken from `edge`, their
            // position within a pixel, which the anchor was placed on.
            let axis = |phase: f64, low: f64, high: f64| {
                let low = low * factor;
                let high = high * factor;
                let first = low - (low - phase).rem_euclid(1.0);
                let last = high + (phase - high).rem_euclid(1.0);
                (first / factor, (last - first).round())
            };
            let (x, width) = axis(edge.0, low.0, high.0);
            let (y, height) = axis(edge.1, low.1, high.1);
            ((x, y), (0.0, 0.0), width, height)
        } else {
            if !(x1 > x0 && y1 > y0) {
                return Ok(None);
            }
            (
                (min.0 + x0 / factor, min.1 + y0 / factor),
                (x0, y0),
                x1 - x0,
                y1 - y0,
            )
        };
        if !width.is_finite()
            || !height.is_finite()
            || width < 1.0
            || height < 1.0
            || width * height > MaskImage::MAX_PIXELS as f64
        {
            return Err(DrawError::CoverageTooLarge);
        }
        Ok(Some(Self {
            width: width as usize,
            height: height as usize,
            origin,
            offset,
            min,
            factor,
            remote,
        }))
    }
    fn point(&self, p: Vec2) -> Vec2 {
        if self.remote {
            (
                (p.0 - self.origin.0) * self.factor,
                (p.1 - self.origin.1) * self.factor,
            )
        } else {
            (
                (p.0 - self.min.0) * self.factor - self.offset.0,
                (p.1 - self.min.1) * self.factor - self.offset.1,
            )
        }
    }
    /// The grid's first corner in the rings' units, before narrowing.
    const fn corner(&self) -> Vec2 {
        self.origin
    }
    fn rect(&self) -> Result<Rect, crate::GeometryError> {
        Rect::new(
            self.origin.0 as f32,
            self.origin.1 as f32,
            (self.width as f64 / self.factor) as f32,
            (self.height as f64 / self.factor) as f32,
        )
    }
}

// Rasterizes closed rings to 8-bit coverage on a physical grid, cropped to the
// local clip. The grid has a pixel edge at `phase` pixels before the rings'
// origin on each axis, and starts at the last such edge at or before the rings'
// logical bounds: with the fractional frame position of the origin as `phase`,
// its cells are the frame's pixels. Four sub-scanlines per pixel row, with
// exact horizontal overlap, as for the built-in shapes.
fn rasterize(
    rings: &[Vec<Vec2>],
    rule: FillRule,
    scale: f64,
    clip: Option<Rect>,
    phase: Vec2,
) -> Result<Option<FillCoverage>, DrawError> {
    let mut min = (f64::INFINITY, f64::INFINITY);
    let mut max = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in rings.iter().flatten() {
        if !p.0.is_finite() || !p.1.is_finite() {
            return Err(crate::GeometryError::NonFinite.into());
        }
        min = (min.0.min(p.0), min.1.min(p.1));
        max = (max.0.max(p.0), max.1.max(p.1));
    }
    if !(max.0 > min.0 && max.1 > min.1) {
        return Ok(None);
    }
    // The distance from the last pixel edge to the bounds, below one pixel.
    // A sum that rounds to a whole pixel moves the anchor a pixel early,
    // which only adds an empty column or row.
    let back = |min: f64, phase: f64| (min * scale + phase).rem_euclid(1.0) / scale;
    let anchor = (min.0 - back(min.0, phase.0), min.1 - back(min.1, phase.1));
    let anchor = if anchor.0.is_finite() && anchor.1.is_finite() {
        anchor
    } else {
        min
    };
    // Where the grid's edges lie within a pixel of the rings' own units.
    let edge = ((-phase.0).rem_euclid(1.0), (-phase.1).rem_euclid(1.0));
    fill_in(rings, rule, (anchor, max), scale, clip, edge)
}

/// A fill's coverage, its rectangle in the path's units and that rectangle's
/// first corner before narrowing to `f32`, which placement on the frame's
/// pixels needs far from the origin.
type FillCoverage = (MaskImage, Rect, (f64, f64));

// Fills closed rings on the physical grid anchored at `bounds.0`, spanning to
// `bounds.1`, cropped to the local clip. `edge` is where the grid's edges lie
// within a pixel, which the anchor agrees with.
fn fill_in(
    rings: &[Vec<Vec2>],
    rule: FillRule,
    (min, max): (Vec2, Vec2),
    scale: f64,
    clip: Option<Rect>,
    edge: Vec2,
) -> Result<Option<FillCoverage>, DrawError> {
    let Some(grid) = RasterGrid::new(min, max, scale, clip, edge)? else {
        return Ok(None);
    };
    let (width, height) = (grid.width, grid.height);
    let mut edges = Vec::new();
    for ring in rings {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let (ax, ay) = grid.point(a);
            let (bx, by) = grid.point(b);
            if ay == by {
                continue;
            }
            let (top, bottom, x, winding) = if ay < by {
                (ay, by, ax, 1)
            } else {
                (by, ay, bx, -1)
            };
            let (x, anchor_y) = if grid.remote {
                ((line_x(a, b, grid.origin.1) - grid.origin.0) * scale, 0.0)
            } else {
                (x, top)
            };
            edges.push(Edge {
                top,
                bottom,
                x,
                slope: if grid.remote {
                    (b.0 - a.0) / (b.1 - a.1)
                } else {
                    (bx - ax) / (by - ay)
                },
                anchor_y,
                winding,
            });
        }
    }
    if fill_work(&edges, height) > crate::VectorRasterizer::MAX_COVERAGE_WORK as f64 {
        return Err(DrawError::TooComplex);
    }
    edges.sort_by(|a, b| a.top.total_cmp(&b.top));
    let mut pixels = vec![0u8; width * height];
    let mut row = vec![0.0f64; width];
    let mut active: Vec<usize> = Vec::new();
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    let mut next = 0;
    let mut any = false;
    for y in 0..height {
        for sample in 0..SCANLINES {
            let sy = y as f64 + (f64::from(sample) + 0.5) * SCANLINE_WEIGHT;
            while next < edges.len() && edges[next].top <= sy {
                active.push(next);
                next += 1;
            }
            active.retain(|&i| edges[i].bottom > sy);
            crossings.clear();
            crossings.extend(active.iter().filter(|&&i| edges[i].top <= sy).map(|&i| {
                let edge = &edges[i];
                (edge.x + (sy - edge.anchor_y) * edge.slope, edge.winding)
            }));
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0;
            let mut start = 0.0;
            for &(x, w) in &crossings {
                let inside = |w: i32| match rule {
                    FillRule::NonZero => w != 0,
                    FillRule::EvenOdd => w % 2 != 0,
                };
                let was = inside(winding);
                winding += w;
                match (was, inside(winding)) {
                    (false, true) => start = x,
                    (true, false) => span(&mut row, start, x),
                    _ => {}
                }
            }
        }
        for (x, value) in row.iter_mut().enumerate() {
            let coverage = (value.min(1.0) * 255.0).round() as u8;
            any |= coverage > 0;
            pixels[y * width + x] = coverage;
            *value = 0.0;
        }
    }
    if !any {
        return Ok(None);
    }
    let rect = grid.rect()?;
    let mask = MaskImage::new(width as u32, height as u32, pixels)?;
    Ok(Some((mask, rect, grid.corner())))
}

// The sub-scanline steps the scan below takes for `edges` in a mask `height`
// rows high: each edge is active on the sub-scanlines whose sample lies in
// `top..bottom`, and sample `k` lies at `(k + 0.5) / SCANLINES`.
fn fill_work(edges: &[Edge], height: usize) -> f64 {
    let samples = (height * SCANLINES as usize) as f64;
    let first_sample = |y: f64| (y * f64::from(SCANLINES) - 0.5).ceil().clamp(0.0, samples);
    edges
        .iter()
        .map(|edge| (first_sample(edge.bottom) - first_sample(edge.top)).max(0.0))
        .sum()
}

// Adds one sub-scanline's horizontal span to a row, `SCANLINE_WEIGHT` pixel high.
fn span(row: &mut [f64], a: f64, b: f64) {
    let width = row.len() as f64;
    let (a, b) = (a.clamp(0.0, width), b.clamp(0.0, width));
    if b <= a {
        return;
    }
    let (first, last) = (a.floor() as usize, b.floor() as usize);
    if first == last {
        row[first] += (b - a) * SCANLINE_WEIGHT;
        return;
    }
    row[first] += (first as f64 + 1.0 - a) * SCANLINE_WEIGHT;
    for value in &mut row[first + 1..last] {
        *value += SCANLINE_WEIGHT;
    }
    if last < row.len() {
        row[last] += (b - last as f64) * SCANLINE_WEIGHT;
    }
}

/// A rectangle, in the path's local units, that contains all coverage
/// [`rasterize_fill`] can produce at `scale`, padded by one physical pixel.
///
/// Rasterization crops to its clip on a grid that starts less than a pixel
/// before the coverage, so two clips with the same intersection with this
/// rectangle give identical coverage at the same phase. `None` when the padded
/// bounds are not representable.
pub(crate) fn coverage_bounds(path: &Path, scale: f64) -> Option<Rect> {
    // Flattened curves stay inside the hull of their control points.
    let pad = 1.0 / scale;
    let [x, y, w, h] = path.bounds().values().map(f64::from);
    let rect = Rect::new(
        (x - pad) as f32,
        (y - pad) as f32,
        (w + 2.0 * pad) as f32,
        (h + 2.0 * pad) as f32,
    )
    .ok()?;
    // Rounding to `f32` must not shrink the rectangle inside the coverage.
    let [rx, ry, rw, rh] = rect.values().map(f64::from);
    (rx <= x && ry <= y && rx + rw >= x + w && ry + rh >= y + h).then_some(rect)
}

/// The coverage of `path` filled with `rule` at `scale` physical pixels per
/// unit, cropped to `clip` in the path's units, on the pixel grid that has an
/// edge `phase` pixels before the path's origin on each axis (each in `0..1`).
/// The returned rectangle is in the path's units and lies on that grid.
pub(crate) fn rasterize_fill(
    path: &Path,
    rule: FillRule,
    scale: f64,
    clip: Option<Rect>,
    phase: (f64, f64),
) -> Result<Option<FillCoverage>, DrawError> {
    let rings: Vec<_> = flatten(path, TOLERANCE / scale)?
        .into_iter()
        .map(|s| s.points)
        .collect();
    rasterize(&rings, rule, scale, clip, phase)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_remote_fill_keeps_the_requested_grid() -> Result<(), Box<dyn std::error::Error>> {
        // The left edge is so far away that its own position holds no
        // fraction of a pixel; the grid still has its edges where the phase
        // puts them, 0.501 22 pixels before each whole unit, so the pixel the
        // right edge at 8 crosses is about half covered.
        let far = -(2f64.powi(60));
        let rings = [vec![(far, 0.0), (8.0, 0.0), (8.0, 4.0), (far, 4.0)]];
        let (mask, rect, corner) = rasterize(
            &rings,
            FillRule::NonZero,
            1.0,
            Some(Rect::new(-0.75, 0.0, 64.0, 4.0)?),
            (0.501_22, 0.0),
        )?
        .ok_or("visible fill")?;
        assert!(
            (corner.0 + 0.501_22)
                .rem_euclid(1.0)
                .min(1.0 - (corner.0 + 0.501_22).rem_euclid(1.0))
                < 1e-9
        );
        assert!((f64::from(rect.x()) - corner.0).abs() < 1e-6);
        let last = ((8.0 - corner.0).ceil() as usize) - 1;
        let row = &mask.pixels()[..mask.width() as usize];
        assert_eq!(row[last - 1], 255);
        assert_eq!(row[last], (0.501_22f64 * 255.0).round() as u8);
        assert!(row[last + 1..].iter().all(|value| *value == 0));
        Ok(())
    }
    #[test]
    fn remote_fill_edges_cover_the_entire_local_clip() -> Result<(), Box<dyn std::error::Error>> {
        let rings = [vec![
            (0.0, -1e20),
            (2.0, -1e20),
            (2.0, 100000.0),
            (0.0, 100000.0),
        ]];
        for factor in [1.0, 1.25, 8.0] {
            let (mask, rect, _) = rasterize(
                &rings,
                FillRule::NonZero,
                factor,
                Some(Rect::new(0.0, 0.0, 2.0, 100.0)?),
                (0.0, 0.0),
            )?
            .ok_or("visible fill")?;
            assert_eq!(rect.y(), 0.0);
            assert_eq!(mask.height(), (100.0 * factor) as u32);
            assert!(
                mask.pixels()
                    .chunks_exact(mask.width() as usize)
                    .all(|row| row[0] == 255)
            );
        }
        Ok(())
    }

    #[test]
    fn subdivision_admission_never_coarsens_or_overallocates() {
        assert_eq!(divisions(15_812.1, MAX_EDGES), Ok(15_813));
        assert_eq!(divisions(10.0, 10), Ok(10));
        for requested in [10.1, f64::INFINITY, f64::NAN] {
            assert_eq!(divisions(requested, 10), Err(PathError::TooLarge));
        }
        assert_eq!(divisions(0.0, 0), Err(PathError::TooLarge));
    }

    #[test]
    fn large_curves_keep_the_physical_chord_tolerance() -> Result<(), Box<dyn std::error::Error>> {
        let p = Point::new;
        let quadratic = crate::PathBuilder::new()
            .move_to(p(-74e6, -27.38e6)?)
            .quad_to(p(26e6, 46.62e6)?, p(126e6, -79.38e6)?)
            .build()?;
        let cubic = crate::PathBuilder::new()
            .move_to(p(-1e5, 0.0)?)
            .cubic_to(p(-2e4, 2e5)?, p(2e4, -2e5)?, p(1e5, 0.0)?)
            .build()?;
        for scale in [1.0, 1.25, 8.0] {
            for (path, is_quad) in [(&quadratic, true), (&cubic, false)] {
                let flattened = flatten(path, TOLERANCE / scale)?;
                let points = &flattened[0].points;
                let n = points.len() - 1;
                assert!(n > 1024);
                for (i, chord) in points.windows(2).enumerate() {
                    for fraction in [0.25, 0.5, 0.75] {
                        let t = (i as f64 + fraction) / n as f64;
                        let u = 1.0 - t;
                        let exact = if is_quad {
                            add(
                                add(
                                    scale_vec((-74e6, -27.38e6), u * u),
                                    scale_vec((26e6, 46.62e6), 2.0 * u * t),
                                ),
                                scale_vec((126e6, -79.38e6), t * t),
                            )
                        } else {
                            add(
                                add(
                                    scale_vec((-1e5, 0.0), u * u * u),
                                    scale_vec((-2e4, 2e5), 3.0 * u * u * t),
                                ),
                                add(
                                    scale_vec((2e4, -2e5), 3.0 * u * t * t),
                                    scale_vec((1e5, 0.0), t * t * t),
                                ),
                            )
                        };
                        let approximate = add(
                            scale_vec(chord[0], 1.0 - fraction),
                            scale_vec(chord[1], fraction),
                        );
                        assert!(len(sub(exact, approximate)) * scale <= TOLERANCE + 1e-6);
                    }
                }
            }
        }
        Ok(())
    }
    fn scale_vec(p: Vec2, factor: f64) -> Vec2 {
        super::scale(p, factor)
    }

    #[test]
    fn an_impossible_curve_count_returns_a_typed_budget_error()
    -> Result<(), Box<dyn std::error::Error>> {
        let path = crate::PathBuilder::new()
            .move_to(Point::ZERO)
            .quad_to(Point::new(0.0, f32::MAX)?, Point::new(1.0, 0.0)?)
            .build()?;
        assert_eq!(flatten(&path, TOLERANCE).err(), Some(PathError::TooLarge));
        Ok(())
    }

    use super::*;
    // The work counts exactly the sub-scanlines whose sample an edge covers.
    #[test]
    fn fill_work_counts_the_sampled_sub_scanlines() {
        let edge = |top: f64, bottom: f64| Edge {
            top,
            bottom,
            x: 0.0,
            slope: 0.0,
            anchor_y: top,
            winding: 1,
        };
        // A whole row holds four samples, at an eighth, three eighths and so on.
        assert_eq!(fill_work(&[edge(0.0, 1.0)], 4), 4.0);
        // A sample on the top edge counts; one on the bottom edge does not.
        assert_eq!(fill_work(&[edge(0.125, 0.375)], 4), 1.0);
        assert_eq!(fill_work(&[edge(0.13, 0.375)], 4), 0.0);
        // Rows outside the mask cost nothing.
        assert_eq!(fill_work(&[edge(-10.0, 10.0)], 2), 8.0);
        // 65 536 edges across 64 rows are exactly the budget.
        let edges: Vec<_> = (0..65_536).map(|_| edge(0.0, 64.0)).collect();
        assert_eq!(
            fill_work(&edges, 64),
            crate::VectorRasterizer::MAX_COVERAGE_WORK as f64
        );
    }

    // A permitted path alternating across a tall mask would take about a
    // billion sub-scanline steps; it is rejected before any coverage work.
    #[test]
    fn coverage_over_the_work_budget_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let count = 60_000;
        let zigzag = |i: usize| {
            Point::new(
                i as f32 * 2048.0 / count as f32,
                if i.is_multiple_of(2) { 0.0 } else { 4000.0 },
            )
        };
        let points = (0..count).map(zigzag).collect::<Result<Vec<_>, _>>()?;
        assert_eq!(
            rasterize_fill(
                &Path::polyline(points)?,
                FillRule::NonZero,
                1.0,
                None,
                (0.0, 0.0)
            )
            .err(),
            Some(DrawError::TooComplex)
        );
        Ok(())
    }
}
