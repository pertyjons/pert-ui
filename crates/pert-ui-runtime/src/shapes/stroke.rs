//! Preparation and reference coverage for strokes.
//!
//! A stroke is prepared once per frame, as a shape is: mapped to physical
//! pixels, clipped to an integer span of frame pixels and expressed relative
//! to that span's origin, all in `f64`. The prepared segment records are
//! `f32`, and [`PreparedStroke::coverage`] evaluates them per pixel center.
//! That evaluation is the reference a renderer reproduces.
//!
//!
//! [`Painter::stroke_path`](crate::Painter::stroke_path) records a stroke as
//! [`DrawCommand::DrawVector`](crate::DrawCommand::DrawVector); a backend
//! that covers strokes itself prepares each one with [`PreparedStroke::new`]
//! and leaves the rest to
//! [`VectorRasterizer::prepare_fills`](crate::VectorRasterizer::prepare_fills).
use super::prepare::{DOMAIN, FrameMapping, PARITY_MARGIN, ShapePrepareError, Span};
use crate::{
    DrawError, GeometryError, LineCap, LineJoin, Point, Rect, VectorGeometry, VectorStyle,
};

const TILE: u32 = PreparedStroke::TILE;
const MAX_REFERENCES: usize = PreparedStroke::MAX_TILE_REFERENCES;
const MAX_LIST: usize = PreparedStroke::MAX_TILE_SEGMENTS;

/// What a listing or a quad adds to a reach for the `f32` evaluation, in
/// pixels. A candidate evaluated in `f32` differs from its `f64` value by
/// less than 1/32 pixel anywhere in the domain (see [`PreparedStroke`]); the
/// rest is headroom.
const PAD: f64 = 0.25;

// How a segment's end is treated. An open end was clipped away: it has no
// cap and no join and does not restrict the body.
const JOINED: u32 = 0;
const CAPPED: u32 = 1;
const OPEN: u32 = 2;
const START_SHIFT: u32 = 0;
const START_BEVEL: u32 = 1 << 2;
const END_SHIFT: u32 = 3;
const END_BEVEL: u32 = 1 << 5;
// The join the segment owns at its end: 0 none, 1 round, 2 bevel, 3 miter.
const JOIN_SHIFT: u32 = 6;
const JOIN_ROUND: u32 = 1;
const JOIN_BEVEL: u32 = 2;
const JOIN_MITER: u32 = 3;
// The path turns toward its left normal at the end, so the outer side is
// the negative one.
const SIDE_NEGATIVE: u32 = 1 << 8;
// The stroke's cap: 0 butt, 1 round, 2 square.
const CAP_SHIFT: u32 = 9;
const CAP_ROUND: u32 = 1;
const CAP_SQUARE: u32 = 2;
// A subpath of one point: `a` is the point.
const POINT: u32 = 1 << 11;

/// Why a stroke could not be prepared, where no other drawing error says
/// it; [`DrawError::Stroke`] carries it.
///
/// With the `serde` feature it implements `Serialize` and `Deserialize`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum StrokePrepareError {
    /// A recorded coordinate, the half width or the support lies farther
    /// than 65 536 physical pixels from the span's origin, where `f32`
    /// evaluation is no longer precise enough. A stroke tens of thousands
    /// of pixels wide, or a miter join that long (a high miter limit at a
    /// turn close to a reversal), does this. Lower the miter limit, use
    /// another join or draw the stroke narrower.
    #[error("stroke geometry lies outside the range that can be evaluated")]
    OutOfRange,
    /// The vector is a fill: [`PreparedStroke::accepts`] is false for it.
    #[error("only strokes are prepared, not fills")]
    Unsupported,
}

/// One segment of a prepared stroke, in physical pixels relative to the
/// span's origin: what [`PreparedStroke::segments`] gives a renderer that
/// evaluates the candidates itself, as the wgpu renderer does in its
/// fragment stage. This is output of [`PreparedStroke::new`] to read, not
/// geometry to construct. [`Self::distance`] is the reference for one
/// record.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct StrokeSegment {
    /// The start point; the point itself for a subpath of one point.
    pub a: [f32; 2],
    /// The end point.
    pub b: [f32; 2],
    /// The unit direction from the unclipped start to the unclipped end.
    pub d: [f32; 2],
    /// The next segment's unit direction, where the end has a join.
    pub n: [f32; 2],
    /// The outer unit bisector of a bevel at the end.
    pub end_m: [f32; 2],
    /// The bevel line's offset `k` at the end. For a miter, which reads no
    /// offset, how far from the end point its wedge can have a candidate
    /// below half a pixel plus the parity margin, with a quarter pixel to
    /// spare: the half size of a square that holds the wedge's pixels.
    pub end_k: f32,
    /// The outer unit bisector of a bevel at the start.
    pub start_m: [f32; 2],
    /// The bevel line's offset `k` at the start.
    pub start_k: f32,
    /// How the candidates apply, as bits from the least significant:
    ///
    /// | Bits | Meaning |
    /// |---|---|
    /// | 0–1 | The start: 0 joined (the body begins at it), 1 capped, 2 open (clipped away: no cap, no join, no limit on the body) |
    /// | 2 | The start has a bevel: `start_m` and `start_k` apply to the body |
    /// | 3–4 | The end, as bits 0–1 |
    /// | 5 | The end has a bevel: `end_m` and `end_k` apply to the body |
    /// | 6–7 | The join this segment owns at its end: 0 none, 1 round, 2 bevel, 3 miter |
    /// | 8 | The path turns toward its left normal at the end, so the join lies on the negative side |
    /// | 9–10 | The stroke's cap: 0 butt, 1 round, 2 square |
    /// | 11 | A subpath of one point: `a` is the point, and only the cap applies |
    pub flags: u32,
}

impl StrokeSegment {
    /// The smallest candidate distance of this segment at `p`, relative to
    /// the span's origin, for the half width `h`; infinite where it has
    /// none. [`PreparedStroke`] states the candidates. A stroke's coverage
    /// at a pixel of its span comes from the smallest of these over the
    /// segments its tile lists, and equally over all of them: a segment a
    /// tile does not list has no candidate below half a pixel plus the
    /// parity margin there, so it adds no coverage, though the smallest
    /// distance itself may then differ.
    #[must_use]
    pub fn distance(&self, h: f32, p: [f32; 2]) -> f32 {
        let f = self.flags;
        let cap = (f >> CAP_SHIFT) & 3;
        let rx = p[0] - self.a[0];
        let ry = p[1] - self.a[1];
        if f & POINT != 0 {
            return if cap == CAP_SQUARE {
                rx.abs().max(ry.abs()) - h
            } else {
                length(rx, ry) - h
            };
        }
        let wx = p[0] - self.b[0];
        let wy = p[1] - self.b[1];
        let u = rx * self.d[0] + ry * self.d[1];
        let v = wx * self.d[0] + wy * self.d[1];
        let start = (f >> START_SHIFT) & 3;
        let end = (f >> END_SHIFT) & 3;
        let mut best = f32::INFINITY;
        if (start != JOINED || u >= 0.0) && (end != JOINED || v <= 0.0) {
            let across = self.d[0] * ry - self.d[1] * rx;
            let mut distance = across.abs() - h;
            if start == CAPPED {
                let beyond = -u;
                distance = distance.max(if cap == CAP_SQUARE {
                    beyond - h
                } else {
                    beyond
                });
            }
            if end == CAPPED {
                distance = distance.max(if cap == CAP_SQUARE { v - h } else { v });
            }
            if f & START_BEVEL != 0 {
                distance = distance.max(rx * self.start_m[0] + ry * self.start_m[1] - self.start_k);
            }
            if f & END_BEVEL != 0 {
                distance = distance.max(wx * self.end_m[0] + wy * self.end_m[1] - self.end_k);
            }
            best = distance;
        }
        let join = (f >> JOIN_SHIFT) & 3;
        if join != 0 && v > 0.0 && wx * self.n[0] + wy * self.n[1] < 0.0 {
            let candidate = match join {
                JOIN_ROUND => length(wx, wy) - h,
                JOIN_BEVEL => {
                    (length(wx, wy) - h).max(wx * self.end_m[0] + wy * self.end_m[1] - self.end_k)
                }
                _ => {
                    let side = if f & SIDE_NEGATIVE != 0 { -1.0 } else { 1.0 };
                    let (fx, fy) = (-self.d[1] * side, self.d[0] * side);
                    let (gx, gy) = (-self.n[1] * side, self.n[0] * side);
                    (wx * fx + wy * fy).max(wx * gx + wy * gy) - h
                }
            };
            best = best.min(candidate);
        }
        if cap == CAP_ROUND {
            if start == CAPPED {
                best = best.min(length(rx, ry) - h);
            }
            if end == CAPPED {
                best = best.min(length(wx, wy) - h);
            }
        }
        best
    }
}

/// The reference evaluation of one pixel of a stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeCoverage {
    coverage: f32,
    distance: f32,
}

impl StrokeCoverage {
    const UNTOUCHED: Self = Self {
        coverage: 0.0,
        distance: f32::INFINITY,
    };

    /// The stroke's coverage, in `0..=1`: the ramp of the distance times the
    /// thin-stroke factor.
    #[must_use]
    pub const fn coverage(self) -> f32 {
        self.coverage
    }

    /// The smallest candidate distance over the pixel's tile list, in
    /// physical pixels, negative inside; infinite for a pixel no listed
    /// segment has a candidate at.
    #[must_use]
    pub const fn distance(self) -> f32 {
        self.distance
    }

    /// Whether the pixel is clear of the coverage ramp by the parity margin
    /// ([`PreparedShape::PARITY_MARGIN`](crate::PreparedShape::PARITY_MARGIN)),
    /// so that every renderer gives it the same saturated coverage.
    #[must_use]
    pub fn is_decided(self) -> bool {
        self.distance.abs() >= 0.5 + PARITY_MARGIN
    }
}

/// A stroke ready to evaluate for one frame, its curves flattened to
/// straight segments.
///
/// [`Self::new`] prepares in `f64` and narrows to `f32`;
/// [`Self::coverage`] evaluates. The two together define what a stroke
/// covers in a renderer that draws strokes itself. Where both axes map by
/// the viewport's scale, as in every frame of a window, the masks of
/// [`VectorRasterizer::prepare`](crate::VectorRasterizer::prepare) cover
/// the same region for a stroke at least a pixel wide, with area coverage
/// instead, so their edge pixels differ. A mapping that stretches one axis
/// stretches a mask with it, while this keeps the stroke's width.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Color, Component, DrawCommand, DrawError, Element, FrameMapping, Length, Painter,
///     Point, PreparedStroke, Runtime, RuntimeError, StrokeStyle, UpdateContext, Viewport};
/// use std::time::Duration;
///
/// struct Line;
/// impl Component for Line {
///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
///     fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
///         let points = [Point::new(4.0, 10.0)?, Point::new(28.0, 10.0)?];
///         p.stroke_polyline(&points, StrokeStyle::new(Length::new(2.0)?), Color::WHITE)
///     }
/// }
/// let viewport = Viewport::new(32.0, 20.0, 1.0)?;
/// let mut ui = Runtime::new();
/// ui.set_viewport(viewport)?;
/// ui.mount_tree(Element::new("line", Line))?;
/// ui.flush(Duration::ZERO)?;
/// let scene = ui.paint()?;
/// let mapping = FrameMapping::new(32, 20, viewport)?;
/// let Some(DrawCommand::DrawVector { vector, origin, scale, clip, .. }) =
///     scene.commands().iter().find(|c| matches!(c, DrawCommand::DrawVector { .. }))
/// else { panic!("no stroke") };
/// let vector = scene.vector(*vector).ok_or("the scene holds its vectors")?;
/// assert!(PreparedStroke::accepts(vector));
/// let stroke = PreparedStroke::new(vector, *origin, *scale, *clip, &mapping)?
///     .ok_or("the stroke is visible")?;
/// // Two pixel rows wide, centered on y = 10.
/// assert_eq!(stroke.coverage(16, 9).coverage(), 1.0);
/// assert_eq!(stroke.coverage(16, 10).coverage(), 1.0);
/// assert_eq!(stroke.coverage(16, 11).coverage(), 0.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Preparation
///
/// With `sx` and `sy` the frame pixels per logical unit of each axis, `s`
/// the viewport scale ([`FrameMapping`]) and `c` the command's scale:
///
/// 1. **Map.** Curves are flattened to chords within 0.2 pixels, measured
///    on the axis with the larger of `sx` and `sy`, times `c`. A vertex
///    inside a flattened curve has a round join whatever the style, since
///    the curve's edge is smooth there. Where a curve ends at a path
///    vertex, a point is added along the curve's exact tangent, at most
///    0.35 of the chord's length from the vertex and a tenth of a pixel
///    from the chord, so that the cap or the join at the vertex has the
///    tangent's direction rather than the chord's. A
///    path point `p` maps to `(origin + p·c)·(sx, sy)`, the width
///    to `w·s·c`. A stroke whose path's bounds, grown by the most its
///    style can reach, miss the frame or the clip prepares nothing, before
///    any curve is flattened. A width below one pixel is evaluated as one pixel, `h =
///    0.5`, with its coverage multiplied by `thin`, the width in pixels;
///    otherwise `h` is half the width and `thin` is 1.
/// 2. **Normalize**, as the mask coverage does: repeated points are dropped,
///    a closing point equal to the first is dropped, a closed subpath with
///    fewer than three points left is open, and a subpath of one point is a
///    point, which a butt cap does not draw. A segment of zero length after
///    mapping is dropped, and a vertex beside one has no join.
/// 3. **Joins.** At each interior vertex, and each vertex of a closed
///    subpath, with unit directions `d0` in and `d1` out: none for a
///    straight continuation; otherwise on the side the path turns away
///    from. The outer unit bisector `m` is the normalized sum of both outer
///    normals for a turn below 90 degrees and the normalized `d0 − d1`
///    otherwise, and `cos` is `m` times the outer normal of `d0`. A miter is
///    kept while `1/cos` is within the limit; else, and for a bevel, the
///    join is the bevel with `k = h·cos`. At a reversal (`cos ≤ 0`) the
///    bevel has `m = d0` and `k = 0`.
/// 4. **Support.** `S = (h + 0.5)·R·(1 + R·2⁻²²) + 1`, with `R` the largest
///    of 1, `√2` for a square cap and the largest kept miter's `1/cos`.
///    No pixel center farther than `S` from every point of the path has
///    coverage; the second factor is the `f32` bound below.
/// 5. **Span.** The mapped points' bounds grown by `S` and rounded outward,
///    intersected with the frame and with the clip, whose edges snap as a
///    shape's do. An empty span prepares nothing.
/// 6. **Domain.** `S` plus the span's larger extent must not exceed 65 536
///    pixels: every recorded coordinate then lies within that distance of
///    the span's origin, and so does every difference an evaluation forms.
///    Otherwise the stroke is refused, not drawn imprecisely.
/// 7. **Segments.** A segment is clipped to the span grown by `S`; one
///    wholly outside is dropped. A clipped end is open. The cut is taken
///    from the line's two-point form, so it holds for end points any
///    distance away. Directions come
///    from the unclipped points. A vertex is narrowed to `f32` once, so the
///    two records that share it hold the same bits.
/// 8. **Tiles.** The span is divided into tiles of [`Self::TILE`] pixels. A
///    tile lists, in path order, every segment that can have a candidate
///    below `0.5` plus the parity margin at one of its pixel centers; it may
///    list more.
/// 9. **Budgets.** More than [`Self::MAX_TILE_REFERENCES`] tile references
///    or [`Self::MAX_TILE_SEGMENTS`] segments in one tile refuse the
///    stroke, and no list is allocated beyond those limits. More than
///    [`Self::MAX_SEGMENTS`] segments kept after clipping refuse it too;
///    flattening has its own limit of points. Besides the lists,
///    preparation holds eight bytes per tile of the span, used or not,
///    which the frame bounds.
///
/// # Candidates
///
/// For a pixel center `p` relative to the span's origin and a segment from
/// `a` to `b` with unit direction `d`: `r = p − a`, `q = p − b`, `u = r·d`,
/// `v = q·d` and `s = d×r`.
///
/// - **Body**, where `u ≥ 0` at a joined start and `v ≤ 0` at a joined end:
///   the largest of `|s| − h`; at a capped start `−u`, less `h` for a square
///   cap; at a capped end `v` likewise; and at an end with a bevel `m·q − k`
///   (`m·r − k` at the start).
/// - **Join**, of the segment that ends at the vertex, where `v > 0` and
///   `q·n < 0` with `n` the next direction: `|q| − h` for a round join, the
///   larger of that and `m·q − k` for a bevel, and for a miter the larger
///   of `q` times each body's outer normal, less `h`.
/// - **Round cap**, at a capped end, everywhere: `|r| − h` or `|q| − h`.
/// - **Point**: `|r| − h` for a round cap, `max(|r.x|, |r.y|) − h` for a
///   square one.
///
/// The stroke's distance `D` is the smallest candidate over the pixel's
/// tile list, and its coverage `clamp(0.5 − D, 0, 1)·thin`. `D` is a bound,
/// not the distance, within a pixel of a bevel's, a miter's or a cap's
/// corners. Lengths with a zero component take no square root, so
/// axis-aligned distances are exact.
///
/// # Rounding
///
/// With every coordinate and difference within 65 536 pixels, narrowing an
/// end point to `f32` moves it by at most 2⁻⁹ pixel per axis, and the half
/// width by at most 2⁻⁸. A vertex moves once, for both records that hold
/// it, so its join and both body ends move with it. Evaluating a candidate
/// then adds the rounding of a few products of a difference with a unit
/// vector narrowed within 2⁻²⁴·⁵: less than 3.2·2⁻²⁴ of the difference's
/// length, under 1/50 pixel in the whole domain. Together a candidate
/// differs from its `f64` value by less than 1/32 pixel, which the listing's
/// reaches exceed by a quarter pixel and the support by one.
/// Only a miter amplifies the evaluation's share. Along its bisector the
/// candidate grows by `1/R` per pixel, so that error moves the tip by its
/// `R`-fold, `(h + 0.5)·R·R·3.2·2⁻²⁴` pixels at most, which the support's
/// factor `1 + R·2⁻²²` covers. A miter too long for the domain is refused
/// by step 6.
#[derive(Clone, Debug, PartialEq)]
pub struct PreparedStroke {
    span: [u32; 4],
    h: f32,
    thin: f32,
    segments: Vec<StrokeSegment>,
    // Tiles per row and rows.
    grid: [u32; 2],
    // Per tile, where its list begins in `lists`; one more entry ends the
    // last.
    starts: Vec<u32>,
    lists: Vec<u32>,
}

type V = [f64; 2];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1]]
}

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1]]
}

fn mul(a: V, s: f64) -> V {
    [a[0] * s, a[1] * s]
}

fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn cross(a: V, b: V) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn unit(a: V) -> Option<V> {
    // Not `hypot`, which is several times slower: mapped coordinates are
    // far from where the squares overflow.
    let length = dot(a, a).sqrt();
    (length > 0.0 && length.is_finite()).then(|| mul(a, 1.0 / length))
}

/// The left-hand normal of a unit direction.
fn normal(d: V) -> V {
    [-d[1], d[0]]
}

fn narrow(v: V) -> [f32; 2] {
    [v[0] as f32, v[1] as f32]
}

/// The length of `(x, y)`, without a square root on an axis, as a shape's.
fn length(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        y.abs()
    } else if y == 0.0 {
        x.abs()
    } else {
        (x * x + y * y).sqrt()
    }
}

/// The join at a vertex: its kind, the outer side, the bevel line and the
/// miter ratio.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Join {
    kind: u32,
    side_negative: bool,
    m: V,
    k: f64,
    ratio: f64,
}

impl Join {
    /// The join between the unit directions `d0` and `d1`, or `None` for a
    /// straight continuation.
    fn at(d0: V, d1: V, h: f64, join: LineJoin, limit: f64) -> Option<Self> {
        let turn = cross(d0, d1);
        if turn == 0.0 && dot(d0, d1) > 0.0 {
            return None;
        }
        let side = if turn > 0.0 { -1.0 } else { 1.0 };
        let side_negative = side < 0.0;
        if join == LineJoin::Round {
            return Some(Self {
                kind: JOIN_ROUND,
                side_negative,
                m: [0.0; 2],
                k: 0.0,
                ratio: 1.0,
            });
        }
        // Each form is well conditioned where it is used: the sum of the
        // normals of a near reversal is rounding noise.
        let m = if dot(d0, d1) > 0.0 {
            mul(unit(add(normal(d0), normal(d1)))?, side)
        } else {
            unit(sub(d0, d1))?
        };
        let cos = dot(m, mul(normal(d0), side));
        if cos <= 0.0 {
            return Some(Self {
                kind: JOIN_BEVEL,
                side_negative,
                m: d0,
                k: 0.0,
                ratio: 1.0,
            });
        }
        let miter = join == LineJoin::Miter && 1.0 / cos <= limit;
        Some(Self {
            kind: if miter { JOIN_MITER } else { JOIN_BEVEL },
            side_negative,
            m,
            k: h * cos,
            ratio: if miter { 1.0 / cos } else { 1.0 },
        })
    }
}

/// Where the line through `a` and `b`, of unit direction `d`, enters and
/// leaves `rect` (`x0, y0, x1, y1`), or `None` when it misses it. The
/// crossings come from the line's two-point form, which stays accurate when
/// both points are remote, where a parameter along the segment would lose
/// the rectangle between them.
fn chord(a: V, b: V, d: V, rect: [f64; 4]) -> Option<(V, V)> {
    let mut ends: Option<(V, V)> = None;
    let center = [(rect[0] + rect[2]) / 2.0, (rect[1] + rect[3]) / 2.0];
    let mut crossing = |p: V| {
        let along = dot(sub(p, center), d);
        ends = Some(match ends {
            None => (p, p),
            Some((first, last)) => (
                if along < dot(sub(first, center), d) {
                    p
                } else {
                    first
                },
                if along > dot(sub(last, center), d) {
                    p
                } else {
                    last
                },
            ),
        });
    };
    if a[1] != b[1] {
        for y in [rect[1], rect[3]] {
            let x = crate::geometry::line_x((a[0], a[1]), (b[0], b[1]), y);
            if x >= rect[0] && x <= rect[2] {
                crossing([x, y]);
            }
        }
    }
    if a[0] != b[0] {
        for x in [rect[0], rect[2]] {
            let y = crate::geometry::line_x((a[1], a[0]), (b[1], b[0]), x);
            if y >= rect[1] && y <= rect[3] {
                crossing([x, y]);
            }
        }
    }
    ends.filter(|(first, last)| first != last)
}

/// A segment before its record joins the stroke, with what the listing
/// needs: its ends relative to the span's origin in `f64`, how far from the
/// segment its body, caps and a round or bevel join can have a candidate
/// below the listing threshold, and how far from its end a miter can.
struct Draft {
    record: StrokeSegment,
    a: V,
    b: V,
    body_reach: f64,
    join_reach: f64,
}

/// One normalized subpath in physical pixels, its segments' directions and
/// its vertices' joins.
struct Subpath {
    points: Vec<V>,
    closed: bool,
    directions: Vec<Option<V>>,
    joins: Vec<Option<Join>>,
}

impl PreparedStroke {
    /// The side of a tile, in physical pixels.
    pub const TILE: u32 = 16;

    /// The most tile references one stroke may record, over all its tiles:
    /// 1,048,576 (2^20). A segment is referenced once by every tile it can
    /// cover a pixel of, so long segments across a large frame count most.
    pub const MAX_TILE_REFERENCES: usize = 1 << 20;

    /// The most segments one tile may list: 4,096 (2^12). Every pixel of a
    /// tile evaluates its whole list, so this bounds the work per pixel.
    pub const MAX_TILE_SEGMENTS: usize = 1 << 12;

    /// The most segments one stroke may keep after its curves are
    /// flattened and it is clipped to its span: 65,536 (2^16), the most
    /// points a path holds.
    pub const MAX_SEGMENTS: usize = 1 << 16;

    /// Whether `vector` is a stroke, which this type prepares. A backend
    /// leaves strokes to [`Self::new`] and fills to a mask, as
    /// [`VectorRasterizer::prepare_fills`](crate::VectorRasterizer::prepare_fills)
    /// does.
    #[must_use]
    pub fn accepts(vector: &VectorGeometry) -> bool {
        matches!(vector.style(), VectorStyle::Stroke(_))
    }

    /// Prepares the stroke `vector` for a frame. `origin`, `scale` and
    /// `clip` are its command's: the translation into the window, the
    /// magnification about it and the clip in window logical coordinates.
    ///
    /// `Ok(None)` means there is nothing to draw: the width is zero, the
    /// path leaves no segment or point, or the span is empty. The color is
    /// the caller's: a transparent stroke need not be prepared.
    ///
    /// # Errors
    /// [`DrawError::Stroke`] holding [`StrokePrepareError::Unsupported`] for
    /// a fill and [`StrokePrepareError::OutOfRange`] beyond the evaluation
    /// domain; [`DrawError::TooComplex`] over [`Self::MAX_SEGMENTS`] or a
    /// tile budget; [`DrawError::Runtime`]
    /// holding [`RuntimeError::Geometry`](crate::RuntimeError::Geometry)
    /// with [`GeometryError::NonFinite`] for a NaN or infinite scale and
    /// [`GeometryError::NonPositiveScale`] for a zero or negative one, as
    /// [`VectorRasterizer::prepare`](crate::VectorRasterizer::prepare)
    /// reports them; and [`DrawError::Path`] with
    /// [`PathError::TooLarge`](crate::PathError::TooLarge) when flattening
    /// the path's curves exceeds
    /// [`VectorRasterizer::MAX_FLATTENED_POINTS`](crate::VectorRasterizer::MAX_FLATTENED_POINTS).
    pub fn new(
        vector: &VectorGeometry,
        origin: Point,
        scale: f32,
        clip: Option<Rect>,
        mapping: &FrameMapping,
    ) -> Result<Option<Self>, DrawError> {
        let VectorStyle::Stroke(style) = vector.style() else {
            return Err(StrokePrepareError::Unsupported.into());
        };
        if !scale.is_finite() {
            return Err(GeometryError::NonFinite.into());
        }
        if scale <= 0.0 {
            return Err(GeometryError::NonPositiveScale.into());
        }
        let scale = f64::from(scale);
        let width = f64::from(style.width().get()) * mapping.scale * scale;
        // A NaN width compares false and so draws nothing.
        if width.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Ok(None);
        }
        let (h, thin) = if width < 1.0 {
            (0.5, width)
        } else {
            (width / 2.0, 1.0)
        };
        let (ox, oy) = (f64::from(origin.x()), f64::from(origin.y()));
        let map = |p: (f64, f64)| -> V {
            [
                (ox + p.0 * scale) * mapping.sx,
                (oy + p.1 * scale) * mapping.sy,
            ]
        };
        let cap = style.cap();
        let limit = f64::from(style.miter_limit());

        // A stroke whose path, grown by the most its style can reach, misses
        // the frame or the clip draws nothing: decided from the path's
        // bounds, which hold its curves, before any curve is flattened.
        let most = match style.join() {
            LineJoin::Miter => limit,
            LineJoin::Bevel | LineJoin::Round => 1.0,
        }
        .max(std::f64::consts::SQRT_2);
        let reach = (h + 0.5) * most * (1.0 + most * 2f64.powi(-22)) + 1.0;
        let [x, y, w, tall] = vector.path().bounds().values().map(f64::from);
        let (near, far) = (map((x, y)), map((x + w, y + tall)));
        let around = [
            (near[0] - reach).floor(),
            (near[1] - reach).floor(),
            (far[0] + reach).ceil(),
            (far[1] + reach).ceil(),
        ];
        if matches!(Span::new(around, clip, mapping), Ok(None)) {
            return Ok(None);
        }
        // Curves become chords within the tolerance in pixels, on the axis
        // that maps a unit to the most of them.
        let subpaths = crate::geometry::stroke_subpaths(
            vector.path(),
            crate::geometry::CHORD_TOLERANCE / (mapping.sx.max(mapping.sy) * scale),
        )?;

        // Steps 2 and 3, and the largest miter ratio.
        let mut paths: Vec<Subpath> = Vec::with_capacity(subpaths.len());
        let mut ratio = 1.0f64;
        let (mut low, mut high) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for source in subpaths {
            let closed = source.closed;
            let mut points: Vec<V> = Vec::with_capacity(source.points.len());
            // Whether each kept point lies inside a flattened curve; a
            // repeated point is a path vertex if either copy is.
            let mut smooth: Vec<bool> = Vec::with_capacity(source.points.len());
            let mut last = None;
            for (p, inside) in source.points.into_iter().zip(source.smooth) {
                if last != Some(p) {
                    points.push(map(p));
                    smooth.push(inside);
                    last = Some(p);
                } else if let Some(kept) = smooth.last_mut() {
                    *kept &= inside;
                }
            }
            if closed && points.len() > 1 && points.first() == points.last() {
                points.pop();
                if let (Some(inside), Some(first)) = (smooth.pop(), smooth.first_mut()) {
                    *first &= inside;
                }
            }
            let count = points.len();
            if count == 0 || (count == 1 && cap == LineCap::Butt) {
                continue;
            }
            for p in &points {
                low = [low[0].min(p[0]), low[1].min(p[1])];
                high = [high[0].max(p[0]), high[1].max(p[1])];
            }
            let closed = closed && count > 2;
            let directions: Vec<Option<V>> = (0..count)
                .map(|i| unit(sub(points[(i + 1) % count], points[i])))
                .collect();
            let mut joins = vec![None; count];
            if count > 1 {
                for (i, slot) in joins.iter_mut().enumerate() {
                    if !closed && (i == 0 || i + 1 == count) {
                        continue;
                    }
                    if let (Some(d0), Some(d1)) =
                        (directions[(i + count - 1) % count], directions[i])
                        && let Some(join) = Join::at(
                            d0,
                            d1,
                            h,
                            // Inside a flattened curve the edge is smooth.
                            if smooth[i] {
                                LineJoin::Round
                            } else {
                                style.join()
                            },
                            limit,
                        )
                    {
                        ratio = ratio.max(join.ratio);
                        *slot = Some(join);
                    }
                }
            }
            paths.push(Subpath {
                points,
                closed,
                directions,
                joins,
            });
        }
        if paths.is_empty() {
            return Ok(None);
        }

        // Steps 4 to 6.
        let reach = ratio.max(if cap == LineCap::Square {
            std::f64::consts::SQRT_2
        } else {
            1.0
        });
        let support = (h + 0.5) * reach * (1.0 + reach * 2f64.powi(-22)) + 1.0;
        let bounds = [
            (low[0] - support).floor(),
            (low[1] - support).floor(),
            (high[0] + support).ceil(),
            (high[1] + support).ceil(),
        ];
        let span = match Span::new(bounds, clip, mapping) {
            Ok(Some(span)) => span,
            Ok(None) => return Ok(None),
            Err(ShapePrepareError::OutOfRange | ShapePrepareError::BorderNotAllowed) => {
                return Err(StrokePrepareError::OutOfRange.into());
            }
        };
        let size = [span.x1 - span.x0, span.y1 - span.y0];
        // A NaN support compares false and is refused.
        if support + size[0].max(size[1]) > DOMAIN || support.is_nan() {
            return Err(StrokePrepareError::OutOfRange.into());
        }
        let grown = [
            span.x0 - support,
            span.y0 - support,
            span.x1 + support,
            span.y1 + support,
        ];
        let origin = [span.x0, span.y0];

        // Step 7.
        let ramp = h + 0.5 + f64::from(PARITY_MARGIN);
        // A point beside a butt or round cap's end line lies at most half a
        // pixel plus the margin beyond it and `ramp` across.
        let beside_cap = ramp.hypot(0.5 + f64::from(PARITY_MARGIN));
        let cap_bits = match cap {
            LineCap::Butt => 0,
            LineCap::Round => CAP_ROUND,
            LineCap::Square => CAP_SQUARE,
        } << CAP_SHIFT;
        let within =
            |p: V| p[0] >= grown[0] && p[0] <= grown[2] && p[1] >= grown[1] && p[1] <= grown[3];
        let most = paths.iter().map(|path| path.points.len()).sum::<usize>();
        let mut drafts: Vec<Draft> = Vec::with_capacity(most.min(Self::MAX_SEGMENTS));
        for path in &paths {
            let count = path.points.len();
            if count == 1 {
                let p = path.points[0];
                if within(p) {
                    if drafts.len() >= Self::MAX_SEGMENTS {
                        return Err(DrawError::TooComplex);
                    }
                    let at = sub(p, origin);
                    drafts.push(Draft {
                        record: StrokeSegment {
                            a: narrow(at),
                            b: narrow(at),
                            flags: POINT | cap_bits,
                            ..StrokeSegment::default()
                        },
                        a: at,
                        b: at,
                        body_reach: ramp
                            * if cap == LineCap::Square {
                                std::f64::consts::SQRT_2
                            } else {
                                1.0
                            }
                            + PAD,
                        join_reach: 0.0,
                    });
                }
            }
            let segments = if path.closed { count } else { count - 1 };
            for i in 0..segments {
                let next = (i + 1) % count;
                let (a, b) = (path.points[i], path.points[next]);
                let Some(d) = path.directions[i] else {
                    continue;
                };
                // The kept part and whether each end of it was cut off.
                let (inside_a, inside_b) = (within(a), within(b));
                let (from, to) = if inside_a && inside_b {
                    (a, b)
                } else {
                    let Some((entry, exit)) = chord(a, b, d, grown) else {
                        continue;
                    };
                    // An end outside lies before the entry or beyond the
                    // exit; the segment misses the rectangle when its start
                    // is beyond or its end before.
                    if (!inside_a && dot(sub(a, exit), d) > 0.0)
                        || (!inside_b && dot(sub(b, entry), d) < 0.0)
                    {
                        continue;
                    }
                    (
                        if inside_a { a } else { entry },
                        if inside_b { b } else { exit },
                    )
                };
                if drafts.len() >= Self::MAX_SEGMENTS {
                    return Err(DrawError::TooComplex);
                }
                let mut flags = cap_bits;
                let mut record = StrokeSegment {
                    d: narrow(d),
                    ..StrokeSegment::default()
                };
                let mut capped = false;
                let start = if !inside_a {
                    flags |= OPEN << START_SHIFT;
                    from
                } else {
                    if path.closed || i > 0 {
                        flags |= JOINED << START_SHIFT;
                        if let Some(join) = path.joins[i].filter(|join| join.kind == JOIN_BEVEL) {
                            flags |= START_BEVEL;
                            record.start_m = narrow(join.m);
                            record.start_k = join.k as f32;
                        }
                    } else {
                        flags |= CAPPED << START_SHIFT;
                        capped = true;
                    }
                    a
                };
                let mut join_reach = 0.0;
                let end = if !inside_b {
                    flags |= OPEN << END_SHIFT;
                    to
                } else {
                    if path.closed || i + 2 < count {
                        flags |= JOINED << END_SHIFT;
                        if let Some(join) = path.joins[next] {
                            flags |= join.kind << JOIN_SHIFT;
                            if join.side_negative {
                                flags |= SIDE_NEGATIVE;
                            }
                            record.n = narrow(path.directions[next].unwrap_or(d));
                            match join.kind {
                                JOIN_BEVEL => {
                                    flags |= END_BEVEL;
                                    record.end_m = narrow(join.m);
                                    record.end_k = join.k as f32;
                                }
                                JOIN_MITER => {
                                    join_reach =
                                        ramp * join.ratio * (1.0 + join.ratio * 2f64.powi(-22))
                                            + PAD;
                                    record.end_k = join_reach as f32;
                                }
                                _ => {}
                            }
                        }
                    } else {
                        flags |= CAPPED << END_SHIFT;
                        capped = true;
                    }
                    b
                };
                let (start, end) = (sub(start, origin), sub(end, origin));
                record.a = narrow(start);
                record.b = narrow(end);
                record.flags = flags;
                drafts.push(Draft {
                    record,
                    a: start,
                    b: end,
                    body_reach: match (capped, cap) {
                        (true, LineCap::Square) => ramp * std::f64::consts::SQRT_2,
                        (true, _) => beside_cap,
                        (false, _) => ramp,
                    } + PAD,
                    join_reach,
                });
            }
        }
        if drafts.is_empty() {
            return Ok(None);
        }

        // Steps 8 and 9. The span is whole pixels within the frame.
        let size = [size[0] as u32, size[1] as u32];
        let grid = [size[0].div_ceil(TILE), size[1].div_ceil(TILE)];
        let tiles = grid[0] as usize * grid[1] as usize;
        let side = f64::from(TILE);
        let cell = |v: f64, count: u32| (v / side).floor().clamp(0.0, f64::from(count - 1)) as u32;
        // The last segment that listed itself in each tile, so that its
        // body and its miter list it once.
        let mut listed = vec![u32::MAX; tiles];
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for (index, draft) in drafts.iter().enumerate() {
            let index = index as u32;
            let mut list = |tx: u32, ty: u32| {
                let tile = ty * grid[0] + tx;
                if listed[tile as usize] != index {
                    listed[tile as usize] = index;
                    // One past the budget is enough to refuse below.
                    if pairs.len() <= MAX_REFERENCES {
                        pairs.push((tile, index));
                    }
                }
            };
            // A center within `reach` of a point of the segment lies in a
            // row of tiles grown by `reach`, at most `reach` from the part
            // of the segment inside that grown row.
            let (a, b, reach) = (draft.a, draft.b, draft.body_reach);
            let (top, bottom) = (a[1].min(b[1]) - reach, a[1].max(b[1]) + reach);
            if bottom >= 0.0 && top < f64::from(size[1]) {
                let dy = b[1] - a[1];
                for ty in cell(top, grid[1])..=cell(bottom, grid[1]) {
                    let (t0, t1) = if dy == 0.0 {
                        (0.0, 1.0)
                    } else {
                        let first = (f64::from(ty) * side - reach - a[1]) / dy;
                        let second = (f64::from(ty + 1) * side + reach - a[1]) / dy;
                        (first.min(second).max(0.0), first.max(second).min(1.0))
                    };
                    if t0 > t1 {
                        continue;
                    }
                    let x0 = a[0] + (b[0] - a[0]) * t0;
                    let x1 = a[0] + (b[0] - a[0]) * t1;
                    let (left, right) = (x0.min(x1) - reach, x0.max(x1) + reach);
                    if right < 0.0 || left >= f64::from(size[0]) {
                        continue;
                    }
                    for tx in cell(left, grid[0])..=cell(right, grid[0]) {
                        list(tx, ty);
                    }
                }
            }
            // A miter's wedge, around the end point.
            let reach = draft.join_reach;
            if reach > 0.0
                && b[0] + reach >= 0.0
                && b[1] + reach >= 0.0
                && b[0] - reach < f64::from(size[0])
                && b[1] - reach < f64::from(size[1])
            {
                for ty in cell(b[1] - reach, grid[1])..=cell(b[1] + reach, grid[1]) {
                    for tx in cell(b[0] - reach, grid[0])..=cell(b[0] + reach, grid[0]) {
                        list(tx, ty);
                    }
                }
            }
            if pairs.len() > MAX_REFERENCES {
                return Err(DrawError::TooComplex);
            }
        }
        // Lists in tile order, each in path order.
        let mut starts = vec![0u32; tiles + 1];
        for &(tile, _) in &pairs {
            starts[tile as usize + 1] += 1;
        }
        if starts.iter().any(|&count| count as usize > MAX_LIST) {
            return Err(DrawError::TooComplex);
        }
        for tile in 0..tiles {
            starts[tile + 1] += starts[tile];
        }
        let mut next = listed;
        next.copy_from_slice(&starts[..tiles]);
        let mut lists = vec![0u32; pairs.len()];
        for &(tile, segment) in &pairs {
            let slot = &mut next[tile as usize];
            lists[*slot as usize] = segment;
            *slot += 1;
        }
        Ok(Some(Self {
            span: span.pixels(),
            h: h as f32,
            thin: thin as f32,
            segments: drafts.into_iter().map(|draft| draft.record).collect(),
            grid,
            starts,
            lists,
        }))
    }

    /// The frame pixels to evaluate: `[x0, y0, x1, y1]`, half-open. No
    /// pixel outside it is covered.
    #[must_use]
    pub const fn span(&self) -> [u32; 4] {
        self.span
    }

    /// The evaluated half width in physical pixels, at least 0.5.
    #[must_use]
    pub const fn half_width(&self) -> f32 {
        self.h
    }

    /// The factor on the coverage of a stroke narrower than one pixel: its
    /// width in pixels; otherwise 1.
    #[must_use]
    pub const fn thin(&self) -> f32 {
        self.thin
    }

    /// The segment records, in path order, for a renderer that evaluates
    /// [`StrokeSegment::distance`] itself. Every pixel center of the span
    /// at which a segment has a candidate below half a pixel lies within
    /// [`Self::half_width`] plus 0.5 of the segment across it and, along
    /// it, of an end that is capped or owns a round or bevel join, or
    /// within its record's `end_k` of an end that owns a miter; allow a
    /// little more for a renderer's own rounding. Outside the span a
    /// record is not meaningful: a clipped end is open, and its body's
    /// candidate goes on along the line.
    #[must_use]
    pub fn segments(&self) -> &[StrokeSegment] {
        &self.segments
    }

    /// The tiles that list a segment, as frame pixels `[x0, y0, x1, y1]`,
    /// half-open, row by row: every pixel with coverage lies in one, so a
    /// renderer evaluates these and skips the rest of the span.
    pub fn tiles(&self) -> impl Iterator<Item = [u32; 4]> + '_ {
        let [x0, y0, x1, y1] = self.span;
        let columns = self.grid[0];
        (0..self.grid[0] * self.grid[1])
            .filter(|&tile| self.starts[tile as usize] != self.starts[tile as usize + 1])
            .map(move |tile| {
                let (left, top) = (x0 + tile % columns * TILE, y0 + tile / columns * TILE);
                [
                    left,
                    top,
                    left + (x1 - left).min(TILE),
                    top + (y1 - top).min(TILE),
                ]
            })
    }

    /// The indices into the segment records that the tile holding the frame
    /// pixel `(x, y)` lists, in path order; empty outside the span.
    fn list(&self, x: u32, y: u32) -> &[u32] {
        let [x0, y0, x1, y1] = self.span;
        if x < x0 || y < y0 || x >= x1 || y >= y1 {
            return &[];
        }
        let tile = ((y - y0) / TILE * self.grid[0] + (x - x0) / TILE) as usize;
        &self.lists[self.starts[tile] as usize..self.starts[tile + 1] as usize]
    }

    /// Calls `visit` with the frame pixel and the coverage of every pixel
    /// the stroke covers, each once, tile by tile: the pixels at which
    /// [`Self::coverage`] reports a coverage above zero, with that
    /// coverage. It is what a renderer on the CPU draws from, and costs
    /// less than asking pixel by pixel: within a tile each listed segment
    /// is evaluated only at the pixel centers it can cover, those within
    /// its reach of it on each row, so a short segment of a dense trace
    /// and a thin diagonal across a tile cost a few pixels each rather
    /// than the tile.
    pub fn for_each_covered(&self, mut visit: impl FnMut(u32, u32, f32)) {
        const SIDE: usize = TILE as usize;
        // The rounding a candidate and these bounds may add; see "Rounding".
        const PAD: f32 = 1.0 / 16.0;
        let [x0, y0, x1, y1] = self.span;
        let mut nearest = [f32::INFINITY; SIDE * SIDE];
        for tile in 0..self.grid[0] * self.grid[1] {
            let list = &self.lists
                [self.starts[tile as usize] as usize..self.starts[tile as usize + 1] as usize];
            if list.is_empty() {
                continue;
            }
            // The tile's corner relative to the span, and its extent.
            let (left, top) = (tile % self.grid[0] * TILE, tile / self.grid[0] * TILE);
            let (width, height) = ((x1 - x0 - left).min(TILE), (y1 - y0 - top).min(TILE));
            nearest.fill(f32::INFINITY);
            for &index in list {
                let segment = &self.segments[index as usize];
                let flags = segment.flags;
                // A candidate below half a pixel lies within this of the
                // segment: across it, around a cap or a join's wedge, and to
                // the corner beside a butt or square cap.
                let capped = (flags >> START_SHIFT) & 3 == CAPPED
                    || (flags >> END_SHIFT) & 3 == CAPPED
                    || flags & POINT != 0;
                let reach = (self.h + 0.5)
                    * if capped {
                        std::f32::consts::SQRT_2
                    } else {
                        1.0
                    }
                    + PAD;
                // A miter's wedge, around the end point.
                let miter = if (flags >> JOIN_SHIFT) & 3 == JOIN_MITER {
                    segment.end_k
                } else {
                    0.0
                };
                let (a, b) = (segment.a, segment.b);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                for row in 0..height {
                    let y = (top + row) as f32 + 0.5;
                    // The part of the segment within `reach` of this row,
                    // and the columns within `reach` of that part.
                    let (first, last) = if dy == 0.0 {
                        (0.0, if (a[1] - y).abs() <= reach { 1.0 } else { -1.0 })
                    } else {
                        let (low, high) = ((y - reach - a[1]) / dy, (y + reach - a[1]) / dy);
                        (low.min(high).max(0.0), low.max(high).min(1.0))
                    };
                    let mut columns = (first <= last).then(|| {
                        let (from, to) = (a[0] + dx * first, a[0] + dx * last);
                        (from.min(to) - reach, from.max(to) + reach)
                    });
                    if miter > 0.0 && (b[1] - y).abs() <= miter {
                        let (from, to) = columns.unwrap_or((b[0], b[0]));
                        columns = Some((from.min(b[0] - miter), to.max(b[0] + miter)));
                    }
                    let Some((from, to)) = columns else {
                        continue;
                    };
                    // The pixels of the tile whose centers lie in the range.
                    let from = (from - 0.5).ceil().max(left as f32);
                    let to = (to - 0.5).floor().min((left + width - 1) as f32);
                    if from > to {
                        continue;
                    }
                    let base = row as usize * SIDE;
                    for column in from as u32..=to as u32 {
                        let slot = &mut nearest[base + (column - left) as usize];
                        *slot = slot.min(segment.distance(self.h, [column as f32 + 0.5, y]));
                    }
                }
            }
            for row in 0..height {
                for column in 0..width {
                    let distance = nearest[row as usize * SIDE + column as usize];
                    let coverage = (0.5 - distance).clamp(0.0, 1.0) * self.thin;
                    if coverage > 0.0 {
                        visit(x0 + left + column, y0 + top + row, coverage);
                    }
                }
            }
        }
    }

    /// The reference evaluation at the center of the frame pixel `(x, y)`.
    /// A pixel outside the span is uncovered.
    #[must_use]
    pub fn coverage(&self, x: u32, y: u32) -> StrokeCoverage {
        let list = self.list(x, y);
        if list.is_empty() {
            return StrokeCoverage::UNTOUCHED;
        }
        // Exact: whole numbers below 65 536 and a half.
        let p = [
            (x - self.span[0]) as f32 + 0.5,
            (y - self.span[1]) as f32 + 0.5,
        ];
        let distance = list
            .iter()
            .map(|&index| self.segments[index as usize].distance(self.h, p))
            .fold(f32::INFINITY, f32::min);
        StrokeCoverage {
            coverage: (0.5 - distance).clamp(0.0, 1.0) * self.thin,
            distance,
        }
    }
}

#[cfg(test)]
mod tests;
