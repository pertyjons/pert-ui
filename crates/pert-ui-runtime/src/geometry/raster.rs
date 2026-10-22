//! Private coverage fallback: curve flattening, stroke outlining and scanline
//! rasterization of validated paths, used by `VectorRasterizer`.
use super::{FillRule, LineCap, LineJoin, Path, PathError, StrokeStyle, Verb};
use crate::{DrawError, MaskImage, Point, Rect};

mod union;

// Largest deviation of flattened curves from the exact curve, in physical pixels.
const TOLERANCE: f64 = 0.2;
// Budget for flattened points and stroke outline edges of one recorded primitive,
// published as `VectorRasterizer::MAX_FLATTENED_POINTS` and `MAX_STROKE_EDGES`.
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
// Twice the signed area of a closed ring, relative to its first point. Sums of
// cross products of absolute coordinates cancel to zero or change sign far
// from the origin, around 1e9 in `f64` for a pixel-sized polygon.
fn signed_area(ring: &[Vec2]) -> f64 {
    let Some(&origin) = ring.first() else {
        return 0.0;
    };
    let local = |i: usize| sub(ring[i % ring.len()], origin);
    (1..ring.len()).map(|i| cross(local(i), local(i + 1))).sum()
}
// Left-hand normal of a unit direction.
fn normal(d: Vec2) -> Vec2 {
    (-d.1, d.0)
}

// Stroke outlines as a union of consistently oriented pieces, filled nonzero.
struct Stroker {
    pieces: Vec<Vec<Vec2>>,
    edges: usize,
    half: f64,
    style: StrokeStyle,
    tolerance: f64,
}
impl Stroker {
    fn piece(&mut self, mut ring: Vec<Vec2>) -> Result<(), PathError> {
        let area = signed_area(&ring);
        if area == 0.0 || !area.is_finite() {
            return Ok(());
        }
        if area < 0.0 {
            ring.reverse();
        }
        self.edges += ring.len();
        if self.edges > MAX_EDGES {
            return Err(PathError::TooLarge);
        }
        self.pieces.push(ring);
        Ok(())
    }
    // Angular step keeping a polygon of radius `half` within the tolerance.
    fn step(&self) -> f64 {
        if self.half <= self.tolerance {
            std::f64::consts::FRAC_PI_4
        } else {
            4.0 * (self.tolerance / (2.0 * self.half)).sqrt().asin()
        }
    }
    fn disc(&mut self, center: Vec2) -> Result<(), PathError> {
        let r = self.half;
        let remaining = MAX_EDGES - self.edges;
        let count = divisions((std::f64::consts::TAU / self.step()).max(8.0), remaining)?;
        let ring = (0..count)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / count as f64;
                (center.0 + r * a.cos(), center.1 + r * a.sin())
            })
            .collect();
        self.piece(ring)
    }
    fn cap(&mut self, end: Vec2, direction: Vec2) -> Result<(), PathError> {
        let h = self.half;
        match self.style.cap {
            LineCap::Butt => Ok(()),
            LineCap::Round => self.disc(end),
            LineCap::Square => {
                let n = scale(normal(direction), h);
                let d = scale(direction, h);
                self.piece(vec![
                    add(end, n),
                    add(add(end, n), d),
                    add(sub(end, n), d),
                    sub(end, n),
                ])
            }
        }
    }
    // The outer arc of the stroke circle between the offset normals of two unit
    // directions, on the side the path turns away from, even for a reversal.
    fn wedge(&mut self, at: Vec2, incoming: Vec2, outgoing: Vec2) -> Result<(), PathError> {
        use std::f64::consts::{PI, TAU};
        let side = if cross(incoming, outgoing) > 0.0 {
            -1.0
        } else {
            1.0
        };
        let from = scale(normal(incoming), side);
        let to = scale(normal(outgoing), side);
        let start = from.1.atan2(from.0);
        let mut sweep = (to.1.atan2(to.0) - start).rem_euclid(TAU);
        if sweep > PI {
            sweep -= TAU;
        }
        let middle = start + sweep / 2.0;
        let away = sub(incoming, outgoing);
        if middle.cos() * away.0 + middle.sin() * away.1 < 0.0 {
            sweep -= TAU.copysign(sweep);
        }
        let remaining = (MAX_EDGES - self.edges)
            .checked_sub(2)
            .ok_or(PathError::TooLarge)?;
        let count = divisions(sweep.abs() / self.step(), remaining)?;
        let r = self.half;
        let mut ring = Vec::with_capacity(count + 2);
        ring.push(at);
        for i in 0..=count {
            let angle = start + sweep * i as f64 / count as f64;
            ring.push((at.0 + r * angle.cos(), at.1 + r * angle.sin()));
        }
        self.piece(ring)
    }
    fn join(&mut self, at: Vec2, incoming: Vec2, outgoing: Vec2) -> Result<(), PathError> {
        let h = self.half;
        let (n0, n1) = (normal(incoming), normal(outgoing));
        // The outer side is the one the path turns away from.
        let side = if cross(incoming, outgoing) > 0.0 {
            -1.0
        } else {
            1.0
        };
        let a = add(at, scale(n0, side * h));
        let b = add(at, scale(n1, side * h));
        match self.style.join {
            LineJoin::Round => self.wedge(at, incoming, outgoing),
            LineJoin::Bevel => self.piece(vec![at, a, b]),
            LineJoin::Miter => {
                if let Some(m) = unit(add(n0, n1)) {
                    let cos_half = m.0 * n0.0 + m.1 * n0.1;
                    if cos_half > 0.0 && 1.0 / cos_half <= f64::from(self.style.miter_limit) {
                        let tip = add(at, scale(m, side * h / cos_half));
                        return self.piece(vec![at, a, tip, b]);
                    }
                }
                self.piece(vec![at, a, b])
            }
        }
    }
    fn subpath(&mut self, subpath: &Subpath) -> Result<(), PathError> {
        // Repeated points carry no direction; merge them, keeping the exact
        // tangent arriving at the first and the one leaving the last.
        let mut points: Vec<Vec2> = Vec::with_capacity(subpath.points.len());
        let mut tangents: Vec<[Option<Vec2>; 2]> = Vec::with_capacity(subpath.points.len());
        for (&p, &t) in subpath.points.iter().zip(&subpath.tangents) {
            if points.last() == Some(&p) {
                if let Some(last) = tangents.last_mut() {
                    last[1] = t[1];
                }
            } else {
                points.push(p);
                tangents.push(t);
            }
        }
        if subpath.closed && points.len() > 1 && points.first() == points.last() {
            points.pop();
            if let Some(end) = tangents.pop() {
                tangents[0][0] = end[0];
            }
        }
        if points.len() == 1 {
            let p = points[0];
            let h = self.half;
            return match self.style.cap {
                LineCap::Butt => Ok(()),
                LineCap::Round => self.disc(p),
                LineCap::Square => self.piece(vec![
                    (p.0 - h, p.1 - h),
                    (p.0 + h, p.1 - h),
                    (p.0 + h, p.1 + h),
                    (p.0 - h, p.1 + h),
                ]),
            };
        }
        let closed = subpath.closed && points.len() > 2;
        let count = if closed {
            points.len()
        } else {
            points.len() - 1
        };
        let total = points.len();
        let chord = |i: usize| unit(sub(points[(i + 1) % total], points[i]));
        // Directions arriving at and leaving each vertex: exact where the path
        // has a vertex, the chord inside a flattened curve.
        let arriving = |i: usize| {
            tangents[i][0]
                .and_then(unit)
                .or_else(|| chord((i + total - 1) % total))
        };
        let leaving = |i: usize| tangents[i][1].and_then(unit).or_else(|| chord(i));
        let h = self.half;
        for i in 0..count {
            let j = (i + 1) % total;
            let (Some(start), Some(end)) = (leaving(i), arriving(j)) else {
                continue;
            };
            let (n0, n1) = (scale(normal(start), h), scale(normal(end), h));
            let (p0, p1) = (points[i], points[j]);
            // Two triangles stay well formed even when the end normals differ.
            self.piece(vec![add(p0, n0), add(p1, n1), sub(p1, n1)])?;
            self.piece(vec![add(p0, n0), sub(p1, n1), sub(p0, n0)])?;
        }
        let joins = if closed { 0..count } else { 1..count };
        for i in joins {
            let (Some(incoming), Some(outgoing)) = (arriving(i), leaving(i)) else {
                continue;
            };
            if tangents[i] == [None, None] {
                // Inside a flattened curve the exact offset is smooth, so the
                // join is round whatever the style, within the tolerance.
                self.wedge(points[i], incoming, outgoing)?;
            } else {
                self.join(points[i], incoming, outgoing)?;
            }
        }
        if !closed {
            if let Some(start) = leaving(0) {
                self.cap(points[0], scale(start, -1.0))?;
            }
            if let Some(end) = arriving(count) {
                self.cap(points[count], end)?;
            }
        }
        Ok(())
    }
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
fn line_x(a: Vec2, b: Vec2, y: f64) -> f64 {
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
                return Err(DrawError::InvalidImage);
            };
            let [cx, cy, cw, ch] = clip.values().map(f64::from);
            let low = (min.0.max(cx), min.1.max(cy));
            let high = (max.0.min(cx + cw), max.1.min(cy + ch));
            if !(high.0 > low.0 && high.1 > low.1) {
                return Ok(None);
            }
            let axis = |min: f64, low: f64, high: f64| {
                // An error-free product retains the original grid phase even
                // when the product's leading word has no fractional bits.
                let leading = min * factor;
                let trailing = min.mul_add(factor, -leading);
                let phase = (leading.rem_euclid(1.0) + trailing.rem_euclid(1.0)).rem_euclid(1.0);
                let low = low * factor;
                let high = high * factor;
                let first = low - (low - phase).rem_euclid(1.0);
                let last = high + (phase - high).rem_euclid(1.0);
                (first / factor, (last - first).round())
            };
            let (x, width) = axis(min.0, low.0, high.0);
            let (y, height) = axis(min.1, low.1, high.1);
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
            return Err(DrawError::InvalidImage);
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
    fn on_grid(&self, p: Vec2) -> Vec2 {
        if self.remote {
            self.point(p)
        } else {
            (
                (p.0 - self.min.0) * self.factor,
                (p.1 - self.min.1) * self.factor,
            )
        }
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

// Rasterizes closed rings to 8-bit coverage on a physical grid anchored at the
// rings' logical bounds, cropped to the local clip. Four sub-scanlines per pixel
// row, with exact horizontal overlap, as for the built-in shapes.
fn rasterize(
    rings: &[Vec<Vec2>],
    rule: FillRule,
    scale: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
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
    fill_in(rings, rule, (min, max), scale, clip)
}

// Fills closed rings on the physical grid anchored at `bounds.0`, spanning to
// `bounds.1`, cropped to the local clip.
fn fill_in(
    rings: &[Vec<Vec2>],
    rule: FillRule,
    (min, max): (Vec2, Vec2),
    scale: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let Some(grid) = RasterGrid::new(min, max, scale, clip)? else {
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
    Ok(Some((mask, rect)))
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

/// A rectangle, in the geometry's local units, that contains all coverage
/// [`rasterize_vector`] can produce at `scale`, padded by one physical pixel.
///
/// Rasterization crops to its clip on a grid anchored at the coverage itself, so
/// two clips with the same intersection with this rectangle give identical
/// coverage. `None` when the padded bounds are not representable.
pub(crate) fn coverage_bounds(vector: &crate::VectorGeometry, scale: f64) -> Option<Rect> {
    // Flattened curves stay inside the hull of their control points.
    let reach = match vector.style() {
        crate::VectorStyle::Fill(_) => 0.0,
        crate::VectorStyle::Stroke(style) => {
            // A miter tip reaches at most the limit times the half width; a
            // square cap reaches the half width along both axes.
            let miter = match style.join {
                LineJoin::Miter => f64::from(style.miter_limit),
                LineJoin::Bevel | LineJoin::Round => 1.0,
            };
            let cap = match style.cap {
                LineCap::Square => std::f64::consts::SQRT_2,
                LineCap::Butt | LineCap::Round => 1.0,
            };
            f64::from(style.width.get()) / 2.0 * miter.max(cap)
        }
    };
    let pad = reach + 1.0 / scale;
    let [x, y, w, h] = vector.path().bounds().values().map(f64::from);
    let rect = Rect::new(
        (x - pad) as f32,
        (y - pad) as f32,
        (w + 2.0 * pad) as f32,
        (h + 2.0 * pad) as f32,
    )
    .ok()?;
    // Rounding to `f32` must not shrink the rectangle inside the coverage.
    let [rx, ry, rw, rh] = rect.values().map(f64::from);
    (rx <= x - reach && ry <= y - reach && rx + rw >= x + w + reach && ry + rh >= y + h + reach)
        .then_some(rect)
}

pub(crate) fn rasterize_vector(
    vector: &crate::VectorGeometry,
    scale: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let subpaths = flatten(vector.path(), TOLERANCE / scale)?;
    match vector.style() {
        crate::VectorStyle::Fill(rule) => {
            let rings: Vec<_> = subpaths.into_iter().map(|s| s.points).collect();
            rasterize(&rings, rule, scale, clip)
        }
        crate::VectorStyle::Stroke(style) => {
            if style.width.get() == 0.0 {
                return Ok(None);
            }
            if union::eligible(vector.path()) {
                return union::rasterize(&subpaths, style, scale, clip);
            }
            outlined(&subpaths, style, scale, clip)
        }
    }
}

// Outlines a stroke as polygons and fills them, for paths with curves.
fn outlined(
    subpaths: &[Subpath],
    style: StrokeStyle,
    scale: f64,
    clip: Option<Rect>,
) -> Result<Option<(MaskImage, Rect)>, DrawError> {
    let mut stroker = Stroker {
        pieces: Vec::new(),
        edges: 0,
        half: f64::from(style.width.get()) / 2.0,
        style,
        tolerance: TOLERANCE / scale,
    };
    for subpath in subpaths {
        stroker.subpath(subpath)?;
    }
    rasterize(&stroker.pieces, FillRule::NonZero, scale, clip)
}

#[cfg(test)]
mod tests {
    #[test]
    fn remote_fill_edges_cover_the_entire_local_clip() -> Result<(), Box<dyn std::error::Error>> {
        let rings = [vec![
            (0.0, -1e20),
            (2.0, -1e20),
            (2.0, 100000.0),
            (0.0, 100000.0),
        ]];
        for factor in [1.0, 1.25, 8.0] {
            let (mask, rect) = rasterize(
                &rings,
                FillRule::NonZero,
                factor,
                Some(Rect::new(0.0, 0.0, 2.0, 100.0)?),
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
    fn impossible_curve_and_round_counts_return_typed_budget_errors()
    -> Result<(), Box<dyn std::error::Error>> {
        let path = crate::PathBuilder::new()
            .move_to(Point::ZERO)
            .quad_to(Point::new(0.0, f32::MAX)?, Point::new(1.0, 0.0)?)
            .build()?;
        assert_eq!(flatten(&path, TOLERANCE).err(), Some(PathError::TooLarge));
        let style = StrokeStyle::new(crate::Length::new(2.0)?);
        let mut stroker = Stroker {
            pieces: Vec::new(),
            edges: 0,
            half: 1e20,
            style,
            tolerance: TOLERANCE,
        };
        assert!(stroker.step() > 0.0);
        assert_eq!(stroker.disc((0.0, 0.0)), Err(PathError::TooLarge));
        assert!(stroker.pieces.is_empty());
        let radius = 1e8;
        stroker.half = radius;
        let n = divisions(std::f64::consts::TAU / stroker.step(), MAX_EDGES)?;
        let sagitta = 2.0 * radius * (std::f64::consts::PI / (2.0 * n as f64)).sin().powi(2);
        assert!(sagitta <= TOLERANCE);
        stroker.edges = MAX_EDGES - 1;
        assert_eq!(
            stroker.wedge((0.0, 0.0), (1.0, 0.0), (0.0, 1.0)),
            Err(PathError::TooLarge)
        );
        Ok(())
    }

    use super::*;
    use std::time::{Duration, Instant};

    // The real-time probe's signal: several sines whose phase moves per frame.
    fn sample(frame: u64, index: usize, samples: usize) -> f32 {
        let t = index as f32 / samples as f32;
        let phase = frame as f32 * 0.07;
        0.5 + 0.3 * (t * 37.0 + phase).sin()
            + 0.12 * (t * 211.0 - 2.0 * phase).sin()
            + 0.05 * (t * 1301.0 + 3.0 * phase).sin()
    }
    fn trace(frame: u64, samples: usize) -> Vec<Point> {
        (0..samples)
            .map(|i| {
                Point::new(
                    80.0 + i as f32 * 800.0 / (samples - 1) as f32,
                    120.0 + (1.0 - sample(frame, i, samples)) * 400.0,
                )
                .expect("finite")
            })
            .collect()
    }
    fn workloads(frame: u64) -> Vec<(&'static str, Vec<Point>)> {
        let point = |x: f32, y: f32| Point::new(x, y).expect("finite");
        vec![
            ("trace, 800 points", trace(frame, 800)),
            ("trace, 4096 points", trace(frame, 4096)),
            (
                "long diagonal",
                vec![point(0.0, 0.0), point(800.0, 400.0 + frame as f32 % 2.0)],
            ),
            (
                "steep alternation, 400 points",
                (0..400)
                    .map(|i| {
                        point(
                            i as f32 * 2.0,
                            if (i + frame as usize).is_multiple_of(2) {
                                0.0
                            } else {
                                400.0
                            },
                        )
                    })
                    .collect(),
            ),
        ]
    }

    // A stroke fixture: its path, style and effective raster scale.
    struct Fixture {
        name: &'static str,
        path: Path,
        style: StrokeStyle,
        scale: f64,
    }
    fn fixtures() -> Result<Vec<Fixture>, Box<dyn std::error::Error>> {
        let p = |x: f32, y: f32| Point::new(x, y);
        let polyline = |points: &[(f32, f32)]| -> Result<Path, Box<dyn std::error::Error>> {
            Ok(Path::polyline(
                points
                    .iter()
                    .map(|&(x, y)| p(x, y))
                    .collect::<Result<Vec<_>, _>>()?,
            )?)
        };
        let styled = |width: f32,
                      cap: LineCap,
                      join: LineJoin|
         -> Result<StrokeStyle, Box<dyn std::error::Error>> {
            Ok(StrokeStyle::new(crate::Length::new(width)?)
                .with_join(join)
                .with_cap(cap))
        };
        let zigzag: Vec<(f32, f32)> = (0..24)
            .map(|i| (i as f32 * 1.5, if i % 2 == 0 { 0.0 } else { 30.0 }))
            .collect();
        let mut fixtures = Vec::new();
        let joins = [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter];
        for (scale, join) in [1.0, 1.25, 2.0]
            .into_iter()
            .flat_map(|scale| joins.map(|join| (scale, join)))
        {
            let round = |width, cap| styled(width, cap, join);
            fixtures.push(Fixture {
                name: "gentle turns",
                path: polyline(&[
                    (0.0, 0.0),
                    (10.0, 4.0),
                    (20.0, 0.0),
                    (30.0, 6.0),
                    (26.0, 14.0),
                ])?,
                style: round(3.0, LineCap::Butt)?,
                scale,
            });
            for cap in [LineCap::Butt, LineCap::Square, LineCap::Round] {
                fixtures.push(Fixture {
                    name: "horizontal line",
                    path: polyline(&[(2.0, 5.0), (30.0, 5.0)])?,
                    style: round(2.0, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "long diagonal",
                    path: polyline(&[(0.0, 0.0), (80.0, 37.0)])?,
                    style: round(1.5, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "steep zigzag",
                    path: polyline(&zigzag)?,
                    style: round(1.5, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "reversal",
                    path: polyline(&[(0.0, 0.0), (20.0, 3.0), (1.0, 6.0)])?,
                    style: round(4.0, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "repeated points",
                    path: polyline(&[(0.0, 0.0), (10.0, 0.0), (10.0, 0.0), (10.0, 10.0)])?,
                    style: round(3.0, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "short collinear segments",
                    path: polyline(&[(0.0, 0.0), (0.1, 0.0), (0.2, 0.0)])?,
                    style: round(4.0, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "hairline vertical",
                    path: polyline(&[(0.3, 0.0), (0.3, 10.0)])?,
                    style: round(0.05, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "thin zigzag",
                    path: polyline(&zigzag)?,
                    style: round(0.3, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "single point",
                    path: polyline(&[(4.0, 4.0)])?,
                    style: round(5.0, cap)?,
                    scale,
                });
                fixtures.push(Fixture {
                    name: "closed two points",
                    path: crate::PathBuilder::new()
                        .move_to(p(0.0, 0.0)?)
                        .line_to(p(12.0, 5.0)?)
                        .close()
                        .build()?,
                    style: round(3.0, cap)?,
                    scale,
                });
            }
            fixtures.push(Fixture {
                name: "self-crossing star",
                path: Path::polygon(
                    (0..5)
                        .map(|i| {
                            let a = std::f32::consts::TAU * (i * 2) as f32 / 5.0;
                            p(20.0 + 18.0 * a.cos(), 20.0 + 18.0 * a.sin())
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )?,
                style: round(2.5, LineCap::Butt)?,
                scale,
            });
            fixtures.push(Fixture {
                name: "trace",
                path: Path::polyline(
                    trace(3, 512)
                        .into_iter()
                        .map(|q| p(q.x() / 8.0, q.y() / 8.0))
                        .collect::<Result<Vec<_>, _>>()?,
                )?,
                style: round(1.5, LineCap::Butt)?,
                scale,
            });
        }
        Ok(fixtures)
    }
    // Per-pixel differences between direct coverage and the outlining
    // fallback on the fallback's own grid: largest, mean, and relative total.
    fn difference(fixture: &Fixture) -> Result<(u8, f64, f64), Box<dyn std::error::Error>> {
        let subpaths = flatten(&fixture.path, TOLERANCE / fixture.scale)?;
        let mut stroker = Stroker {
            pieces: Vec::new(),
            edges: 0,
            half: f64::from(fixture.style.width.get()) / 2.0,
            style: fixture.style,
            tolerance: TOLERANCE / fixture.scale,
        };
        for subpath in &subpaths {
            stroker.subpath(subpath)?;
        }
        // One grid covering both extents, so coverage either path adds outside
        // the other's bounds is compared too.
        let outline = stroker.pieces.iter().flatten().fold(
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
        );
        let direct = union::piece_bounds(&subpaths, fixture.style)?;
        let bounds = (
            (outline.0.0.min(direct.0.0), outline.0.1.min(direct.0.1)),
            (outline.1.0.max(direct.1.0), outline.1.1.max(direct.1.1)),
        );
        let drawn = bounds.1.0 > bounds.0.0 && bounds.1.1 > bounds.0.1;
        let fallback = if drawn && !stroker.pieces.is_empty() {
            fill_in(
                &stroker.pieces,
                FillRule::NonZero,
                bounds,
                fixture.scale,
                None,
            )?
        } else {
            None
        };
        let fast = if drawn {
            union::rasterize_anchored(&subpaths, fixture.style, bounds, fixture.scale)?
        } else {
            None
        };
        let (expected, actual) = match (fallback, fast) {
            (Some((expected, _)), Some((actual, _))) => (expected, actual),
            (None, None) => return Ok((0, 0.0, 0.0)),
            _ => return Err(format!("{}: only one implementation drew", fixture.name).into()),
        };
        if (expected.width(), expected.height()) != (actual.width(), actual.height()) {
            return Err(format!("{}: grids differ", fixture.name).into());
        }
        let (mut largest, mut sum, mut total, mut reference) = (0u8, 0u64, 0u64, 0u64);
        for (&a, &b) in expected.pixels().iter().zip(actual.pixels()) {
            largest = largest.max(a.abs_diff(b));
            sum += u64::from(a.abs_diff(b));
            total += u64::from(b);
            reference += u64::from(a);
        }
        let pixels = expected.pixels().len() as f64;
        Ok((
            largest,
            sum as f64 / pixels,
            (total as f64 - reference as f64) / reference as f64,
        ))
    }

    // Both paths sample four sub-scanlines with exact horizontal overlap, so
    // they differ only in geometry: the outline fill flattens round caps and
    // joins within 0.2 physical pixels. Where an arc runs nearly level, that
    // offset can move a whole sub-scanline's span of a pixel, a quarter of its
    // coverage. Mean and total tolerances measured on 2026-09-30.
    #[test]
    fn direct_coverage_matches_the_outline_fill() -> Result<(), Box<dyn std::error::Error>> {
        for fixture in fixtures()? {
            // Every production mask, compared below or not, must stay within
            // the bounds the coverage cache assumes for its geometry.
            let subpaths = flatten(&fixture.path, TOLERANCE / fixture.scale)?;
            let vector = crate::VectorGeometry::new(
                fixture.path.clone(),
                crate::VectorStyle::Stroke(fixture.style),
            );
            if let Some((_, rect)) =
                union::rasterize(&subpaths, fixture.style, fixture.scale, None)?
            {
                let limit = coverage_bounds(&vector, fixture.scale).ok_or("no coverage bounds")?;
                assert_eq!(
                    limit.intersection(rect),
                    rect,
                    "{} exceeds its coverage bounds",
                    fixture.name
                );
            }
            let dot = matches!(fixture.name, "single point" | "short collinear segments");
            if dot && fixture.style.cap == LineCap::Round {
                // Mostly a round cap's disc, which the outline fill's inscribed
                // polygon underestimates; checked against the exact disc below.
                continue;
            }
            let (largest, mean, total) = difference(&fixture)?;
            let context = format!(
                "{} at {} with {:?} caps and {:?} joins: largest {largest}, mean {mean:.2}, total {:+.3}%",
                fixture.name,
                fixture.scale,
                fixture.style.cap,
                fixture.style.join,
                total * 100.0
            );
            assert!(largest <= 64, "{context}");
            assert!(mean <= 2.5, "{context}");
            assert!(total.abs() <= 0.02, "{context}");
        }
        Ok(())
    }

    // With round caps and round joins a stroke is exactly the points within
    // half its width of the path. Sampling that distance on the same four
    // sub-scanlines, 256 columns per pixel, checks where direct coverage puts
    // its coverage, not only how much, including the fixtures that are mostly
    // a round cap's disc.
    #[test]
    fn round_strokes_match_their_distance_field() -> Result<(), Box<dyn std::error::Error>> {
        const COLUMNS: usize = 256;
        fn distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
            let (d, q) = (sub(b, a), sub(p, a));
            let length = d.0 * d.0 + d.1 * d.1;
            let t = if length > 0.0 {
                ((q.0 * d.0 + q.1 * d.1) / length).clamp(0.0, 1.0)
            } else {
                0.0
            };
            len(sub(q, scale(d, t)))
        }
        for fixture in fixtures()? {
            if fixture.style.cap != LineCap::Round || fixture.style.join != LineJoin::Round {
                continue;
            }
            let subpaths = flatten(&fixture.path, TOLERANCE / fixture.scale)?;
            let segments: Vec<(Vec2, Vec2)> = subpaths
                .iter()
                .flat_map(|subpath| {
                    let points = &subpath.points;
                    let count = if subpath.closed {
                        points.len()
                    } else {
                        points.len() - 1
                    };
                    (0..count.max(1)).map(move |i| (points[i], points[(i + 1) % points.len()]))
                })
                .collect();
            let half = f64::from(fixture.style.width.get()) / 2.0;
            let (mask, rect) = union::rasterize(&subpaths, fixture.style, fixture.scale, None)?
                .ok_or("a round stroke drew nothing")?;
            let (width, height) = (mask.width() as usize, mask.height() as usize);
            let (x0, y0) = (f64::from(rect.x()), f64::from(rect.y()));
            let mut largest = 0u8;
            for y in 0..height {
                for x in 0..width {
                    // Only segments that can reach this pixel.
                    let center = (
                        x0 + (x as f64 + 0.5) / fixture.scale,
                        y0 + (y as f64 + 0.5) / fixture.scale,
                    );
                    let near: Vec<_> = segments
                        .iter()
                        .copied()
                        .filter(|&(a, b)| distance(center, a, b) <= half + 0.75 / fixture.scale)
                        .collect();
                    let mut inside = 0usize;
                    for row in 0..SCANLINES as usize {
                        let py =
                            y0 + (y as f64 + (row as f64 + 0.5) * SCANLINE_WEIGHT) / fixture.scale;
                        for column in 0..COLUMNS {
                            let px = x0
                                + (x as f64 + (column as f64 + 0.5) / COLUMNS as f64)
                                    / fixture.scale;
                            if near.iter().any(|&(a, b)| distance((px, py), a, b) <= half) {
                                inside += 1;
                            }
                        }
                    }
                    let expected =
                        (inside as f64 * 255.0 / (COLUMNS * SCANLINES as usize) as f64 + 0.5) as u8;
                    largest = largest.max(expected.abs_diff(mask.pixels()[y * width + x]));
                }
            }
            assert!(
                largest <= 2,
                "{} at {}: largest {largest}",
                fixture.name,
                fixture.scale
            );
        }
        Ok(())
    }

    // The fallback's inscribed polygon underestimates a disc; direct coverage
    // covers the exact disc within its sampling error.
    #[test]
    fn a_round_dot_covers_the_exact_disc_area() -> Result<(), Box<dyn std::error::Error>> {
        for scale in [1.0, 1.25, 2.0] {
            let path = Path::polyline([Point::new(4.0, 4.0)?])?;
            let subpaths = flatten(&path, TOLERANCE / scale)?;
            let style = StrokeStyle::new(crate::Length::new(5.0)?).with_cap(LineCap::Round);
            let (mask, _) =
                union::rasterize(&subpaths, style, scale, None)?.ok_or("the dot drew nothing")?;
            let area: f64 = mask.pixels().iter().map(|&c| f64::from(c) / 255.0).sum();
            let exact = std::f64::consts::PI * (2.5 * scale).powi(2);
            assert!(
                (area / exact - 1.0).abs() < 0.02,
                "{area} vs {exact} at {scale}"
            );
        }
        Ok(())
    }

    // Two pieces that each cover part of a pixel cover it fully: coverage is
    // the union of the pieces, not the larger of their separate coverages.
    #[test]
    fn pieces_sharing_a_pixel_cover_their_union() -> Result<(), Box<dyn std::error::Error>> {
        let p = Point::new;
        let path = crate::PathBuilder::new()
            .move_to(p(0.0, 5.0)?)
            .line_to(p(10.5, 5.0)?)
            .move_to(p(10.5, 5.0)?)
            .line_to(p(20.0, 5.0)?)
            .build()?;
        let subpaths = flatten(&path, TOLERANCE)?;
        let style = StrokeStyle::new(crate::Length::new(4.0)?);
        let (mask, rect) =
            union::rasterize(&subpaths, style, 1.0, None)?.ok_or("the stroke drew nothing")?;
        assert_eq!(
            (rect.x(), rect.y(), mask.width(), mask.height()),
            (0.0, 3.0, 20, 4)
        );
        // Column 10 is split at x = 10.5 between the two subpaths.
        for row in 0..4 {
            assert_eq!(mask.pixels()[row * 20 + 10], 255, "row {row}");
        }
        Ok(())
    }

    // A clip crops the coverage on the unclipped grid: the clipped mask equals
    // the same pixels of the unclipped one, whatever the join.
    #[test]
    fn a_clip_crops_direct_coverage_on_its_own_grid() -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::polyline(trace(5, 256))?;
        let subpaths = flatten(&path, TOLERANCE)?;
        for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
            let style = StrokeStyle::new(crate::Length::new(2.5)?).with_join(join);
            for scale in [1.0, 1.25] {
                let (full, whole) = union::rasterize(&subpaths, style, scale, None)?
                    .ok_or("the trace drew nothing")?;
                let clip = Rect::new(300.3, 200.7, 211.0, 97.0)?;
                let (part, cropped) = union::rasterize(&subpaths, style, scale, Some(clip))?
                    .ok_or("the clip hid the trace")?;
                let dx = ((f64::from(cropped.x()) - f64::from(whole.x())) * scale).round() as usize;
                let dy = ((f64::from(cropped.y()) - f64::from(whole.y())) * scale).round() as usize;
                let (width, height) = (part.width() as usize, part.height() as usize);
                for y in 0..height {
                    let row = &full.pixels()[(dy + y) * full.width() as usize + dx..][..width];
                    assert_eq!(
                        &part.pixels()[y * width..][..width],
                        row,
                        "{join:?} at {scale}, row {y}"
                    );
                }
            }
        }
        Ok(())
    }

    // A round join covers only the outer sector of a turn: nearly collinear
    // short segments with butt caps stay between their endpoints.
    #[test]
    fn a_round_join_does_not_reach_past_butt_caps() -> Result<(), Box<dyn std::error::Error>> {
        let p = Point::new;
        let path = Path::polyline([p(0.0, 0.0)?, p(0.1, 0.0)?, p(0.2, 0.001)?])?;
        let style = StrokeStyle::new(crate::Length::new(4.0)?).with_join(LineJoin::Round);
        let subpaths = flatten(&path, TOLERANCE)?;
        let (_, rect) =
            union::rasterize(&subpaths, style, 1.0, None)?.ok_or("the stroke drew nothing")?;
        assert!(
            rect.x() > -0.01 && rect.x() + rect.width() < 1.21,
            "{rect:?}"
        );
        Ok(())
    }

    // A width below one physical pixel draws proportionally fainter coverage,
    // even when it falls between sample positions.
    #[test]
    fn a_hairline_draws_proportional_coverage() -> Result<(), Box<dyn std::error::Error>> {
        let p = Point::new;
        for (x, width) in [(0.3, 0.05), (0.51, 0.1), (0.2, 0.5)] {
            let path = Path::polyline([p(x, 0.0)?, p(x, 10.0)?])?;
            let style = StrokeStyle::new(crate::Length::new(width)?);
            let subpaths = flatten(&path, TOLERANCE)?;
            let (mask, _) =
                union::rasterize(&subpaths, style, 1.0, None)?.ok_or("the hairline vanished")?;
            let row = &mask.pixels()[5 * mask.width() as usize..][..mask.width() as usize];
            let total: u32 = row.iter().map(|&c| u32::from(c)).sum();
            let expected = f64::from(width) * 255.0;
            assert!(
                (f64::from(total) - expected).abs() <= 2.0,
                "width {width} at {x}: {row:?}"
            );
        }
        Ok(())
    }

    // Far from the origin both paths cover a stroke as they do near it: their
    // polygon orientation is computed relative to a corner, since absolute
    // cross products cancel there. 2^30 and multiples of 128 are exact in f32.
    #[test]
    fn a_stroke_far_from_the_origin_covers_as_near_it() -> Result<(), Box<dyn std::error::Error>> {
        let shift = 1_073_741_824.0f32;
        let p = |x: f32, y: f32, at: f32| Point::new(at + x, at + y);
        let straight = |at: f32| -> Result<Path, Box<dyn std::error::Error>> {
            Ok(Path::polyline([
                p(0.0, 0.0, at)?,
                p(256.0, 0.0, at)?,
                p(256.0, 256.0, at)?,
            ])?)
        };
        let curved = |at: f32| -> Result<Path, Box<dyn std::error::Error>> {
            Ok(crate::PathBuilder::new()
                .move_to(p(0.0, 0.0, at)?)
                .line_to(p(256.0, 0.0, at)?)
                .quad_to(p(256.0, 256.0, at)?, p(512.0, 256.0, at)?)
                .build()?)
        };
        for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
            let style = StrokeStyle::new(crate::Length::new(2.0)?).with_join(join);
            for (kind, path) in [
                ("straight", &straight as &dyn Fn(f32) -> _),
                ("curved", &curved),
            ] {
                let mask = |at: f32| -> Result<MaskImage, Box<dyn std::error::Error>> {
                    let vector =
                        crate::VectorGeometry::new(path(at)?, crate::VectorStyle::Stroke(style));
                    Ok(rasterize_vector(&vector, 1.0, None)?
                        .ok_or("nothing drawn")?
                        .0)
                };
                let (near, far) = (mask(0.0)?, mask(shift)?);
                assert_eq!(
                    (near.width(), near.height()),
                    (far.width(), far.height()),
                    "{kind} {join:?}"
                );
                let largest = near
                    .pixels()
                    .iter()
                    .zip(far.pixels())
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap_or(0);
                assert!(largest <= 1, "{kind} {join:?}: {largest}");
            }
        }
        Ok(())
    }

    // Both implementations reject a raster beyond the pixel limit alike.
    #[test]
    fn an_oversized_stroke_is_rejected_like_the_fallback() -> Result<(), Box<dyn std::error::Error>>
    {
        let path = Path::polyline([Point::new(0.0, 0.0)?, Point::new(1.0e6, 1.0e6)?])?;
        let style = StrokeStyle::new(crate::Length::new(2.0)?).with_join(LineJoin::Round);
        let subpaths = flatten(&path, TOLERANCE)?;
        assert!(matches!(
            union::rasterize(&subpaths, style, 1.0, None),
            Err(DrawError::InvalidImage)
        ));
        assert!(matches!(
            outlined(&subpaths, style, 1.0, None),
            Err(DrawError::InvalidImage)
        ));
        Ok(())
    }

    // Straight strokes take direct coverage whatever their join; curves
    // keep the outline fill.
    #[test]
    fn only_straight_strokes_are_eligible() -> Result<(), Box<dyn std::error::Error>> {
        let p = Point::new;
        let straight = Path::polyline([p(0.0, 0.0)?, p(4.0, 0.0)?, p(4.0, 4.0)?])?;
        let closed = Path::polygon([p(0.0, 0.0)?, p(4.0, 0.0)?, p(4.0, 4.0)?])?;
        let curved = crate::PathBuilder::new()
            .move_to(p(0.0, 0.0)?)
            .quad_to(p(2.0, 4.0)?, p(4.0, 0.0)?)
            .build()?;
        assert!(union::eligible(&straight));
        assert!(union::eligible(&closed));
        assert!(!union::eligible(&curved));
        Ok(())
    }

    // Splits stroke rasterization into its phases for the fast-path plan.
    // Timing evidence only; run in release with `--ignored --nocapture`.
    #[test]
    #[ignore = "timing profile; run in release"]
    fn stroke_phase_profile() -> Result<(), Box<dyn std::error::Error>> {
        const FRAMES: u64 = 60;
        let width = crate::Length::new(1.5)?;
        for (join, (index, (name, _))) in [LineJoin::Round, LineJoin::Miter]
            .into_iter()
            .flat_map(|join| workloads(0).into_iter().enumerate().map(move |w| (join, w)))
        {
            let style = StrokeStyle::new(width).with_join(join);
            let mut phases = [Duration::ZERO; 4];
            let (mut edges, mut active, mut rows) = (0usize, 0usize, 0usize);
            for frame in 0..FRAMES {
                let points = workloads(frame).swap_remove(index).1;
                let vector = crate::VectorGeometry::new(
                    Path::polyline(points)?,
                    crate::VectorStyle::Stroke(style),
                );
                let start = Instant::now();
                let subpaths = flatten(vector.path(), TOLERANCE)?;
                phases[0] += start.elapsed();
                let start = Instant::now();
                let mut stroker = Stroker {
                    pieces: Vec::new(),
                    edges: 0,
                    half: 0.75,
                    style,
                    tolerance: TOLERANCE,
                };
                for subpath in &subpaths {
                    stroker.subpath(subpath)?;
                }
                phases[1] += start.elapsed();
                let start = Instant::now();
                rasterize(&stroker.pieces, FillRule::NonZero, 1.0, None)?;
                phases[2] += start.elapsed();
                let start = Instant::now();
                union::rasterize(&subpaths, style, 1.0, None)?;
                phases[3] += start.elapsed();
                if frame == 0 {
                    // Edges spanning each sub-scanline, as the scan sees them.
                    let all: Vec<(f64, f64)> = stroker
                        .pieces
                        .iter()
                        .flat_map(|ring| {
                            (0..ring.len()).map(|i| {
                                let (a, b) = (ring[i].1, ring[(i + 1) % ring.len()].1);
                                (a.min(b), a.max(b))
                            })
                        })
                        .filter(|(top, bottom)| bottom > top)
                        .collect();
                    edges = all.len();
                    let low = all.iter().map(|e| e.0).fold(f64::INFINITY, f64::min);
                    let high = all.iter().map(|e| e.1).fold(f64::NEG_INFINITY, f64::max);
                    let mut y = low.floor() + 0.125;
                    while y < high {
                        active += all.iter().filter(|(t, b)| *t <= y && *b > y).count();
                        rows += 1;
                        y += 0.25;
                    }
                }
            }
            let per = |d: Duration| d / FRAMES as u32;
            println!(
                "{name}, {join:?} joins: {edges} outline edges, mean active edges per sub-scanline {:.0}; flatten {:?}, outline {:?}, scan {:?}; direct coverage {:?} per frame",
                active as f64 / rows.max(1) as f64,
                per(phases[0]),
                per(phases[1]),
                per(phases[2]),
                per(phases[3]),
            );
        }
        Ok(())
    }

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
    // billion sub-scanline steps on every path; it is rejected before any
    // coverage work, and an ordinary dense trace is not.
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
        let style = StrokeStyle::new(crate::Length::new(1.0)?);
        let mut curved = crate::PathBuilder::new().move_to(points[0]);
        for &point in &points[1..] {
            curved = curved.line_to(point);
        }
        let curved = curved
            .quad_to(Point::new(2100.0, 0.0)?, Point::new(2200.0, 4000.0)?)
            .build()?;
        let cases = [
            // The direct path for straight segments.
            (
                Path::polyline(points.clone())?,
                crate::VectorStyle::Stroke(style),
            ),
            // The outline fill, for a stroke with a curve and for a fill.
            (curved, crate::VectorStyle::Stroke(style)),
            (
                Path::polyline(points)?,
                crate::VectorStyle::Fill(FillRule::NonZero),
            ),
        ];
        for (path, vector_style) in cases {
            let vector = crate::VectorGeometry::new(path, vector_style);
            assert_eq!(
                rasterize_vector(&vector, 1.0, None).err(),
                Some(DrawError::TooComplex),
                "{vector_style:?}"
            );
        }
        // The real-time probe's densest workloads stay inside the budget.
        for (name, points) in workloads(0) {
            let vector = crate::VectorGeometry::new(
                Path::polyline(points)?,
                crate::VectorStyle::Stroke(style),
            );
            assert!(rasterize_vector(&vector, 2.0, None)?.is_some(), "{name}");
        }
        Ok(())
    }
}
