//! The preparation and the reference coverage of
//! `plans/rendering/stroke-spec.md`, outside the library.
//!
//! Preparation works in `f64` and narrows to `f32` records; the reference
//! evaluates the records in `f32` in the order `stroke.wgsl` does.

/// The parity margin, as `PreparedShape::PARITY_MARGIN`.
pub const MARGIN: f64 = 1.0 / 16.0;
/// Recorded values must lie within this many pixels of the span's origin.
pub const DOMAIN: f64 = 65_536.0;
/// Provisional budgets; slice 0 sets the real ones from the measurements.
pub const MAX_REFERENCES: usize = 1 << 22;
pub const MAX_LIST: usize = 1 << 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    Round,
    Bevel,
    Miter,
}

#[derive(Clone, Debug)]
pub struct Subpath {
    pub points: Vec<[f32; 2]>,
    pub closed: bool,
}

/// One `DrawVector` stroke: geometry-local points, placed by `origin` and
/// `scale` as the command places them.
#[derive(Clone, Debug)]
pub struct Stroke {
    pub subpaths: Vec<Subpath>,
    pub width: f32,
    pub cap: Cap,
    pub join: Join,
    pub miter_limit: f32,
    /// Straight-alpha linear color.
    pub color: [f32; 4],
    pub origin: [f32; 2],
    pub scale: f32,
    /// Window-coordinate clip: x, y, width, height.
    pub clip: Option<[f32; 4]>,
}

impl Stroke {
    pub fn polyline(points: Vec<[f32; 2]>, width: f32, cap: Cap, join: Join) -> Self {
        Self {
            subpaths: vec![Subpath {
                points,
                closed: false,
            }],
            width,
            cap,
            join,
            miter_limit: 4.0,
            color: [1.0; 4],
            origin: [0.0; 2],
            scale: 1.0,
            clip: None,
        }
    }
}

/// The frame: physical extent and physical pixels per logical pixel.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

// Segment flags. An end is joined (the body stops at it), capped, or open
// (clipped away: no cap, no join, no restriction).
pub const JOINED: u32 = 0;
pub const CAPPED: u32 = 1;
pub const OPEN: u32 = 2;
const START_SHIFT: u32 = 0;
const START_BEVEL: u32 = 1 << 2;
const END_SHIFT: u32 = 3;
const END_BEVEL: u32 = 1 << 5;
const JOIN_SHIFT: u32 = 6; // 0 none, 1 round, 2 bevel, 3 miter
const SIDE_NEGATIVE: u32 = 1 << 8;
const CAP_SHIFT: u32 = 9; // 0 butt, 1 round, 2 square
const POINT: u32 = 1 << 11;
const CAP_ROUND: u32 = 1;
const CAP_SQUARE: u32 = 2;

/// One segment record, relative to its stroke's span origin; 64 bytes on the
/// GPU (`stroke.wgsl`'s `Segment`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Segment {
    pub a: [f32; 2],
    pub b: [f32; 2],
    pub d: [f32; 2],
    /// The next segment's direction at a joined end.
    pub n: [f32; 2],
    /// The bevel's outer bisector and offset at the end, and at the start.
    pub end_m: [f32; 2],
    pub end_k: f32,
    pub start_m: [f32; 2],
    pub start_k: f32,
    pub flags: u32,
    /// The stroke's index in the frame, for the per-segment quads.
    pub stroke: u32,
}
impl Segment {
    fn write(&self, out: &mut Vec<u8>) {
        let values = [
            self.a[0],
            self.a[1],
            self.b[0],
            self.b[1],
            self.d[0],
            self.d[1],
            self.n[0],
            self.n[1],
            self.end_m[0],
            self.end_m[1],
            self.end_k,
            self.start_k,
            self.start_m[0],
            self.start_m[1],
            f32::from_bits(self.flags),
            f32::from_bits(self.stroke),
        ];
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
}

/// Per stroke: span origin in frame pixels, evaluated half width, the
/// thin-stroke factor and the premultiplied linear color; 32 bytes.
#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub origin: [u32; 2],
    pub h: f32,
    pub thin: f32,
    pub color: [f32; 4],
}
/// A tile of a stroke's span in frame pixels and its segment list; 32 bytes.
#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub rect: [u32; 4],
    pub start: u32,
    pub count: u32,
    pub stroke: u32,
}

/// Where one stroke lies: its span in frame pixels and its segments and
/// tiles in the frame's records.
#[derive(Clone, Debug)]
pub struct Extent {
    pub span: [u32; 4],
    /// Whether any join is a miter, whose wedge the quads path draws apart.
    pub miters: bool,
    pub segments: std::ops::Range<u32>,
    pub tiles: std::ops::Range<u32>,
}

/// A frame's prepared strokes, in drawing order.
#[derive(Default)]
pub struct Prepared {
    pub headers: Vec<Header>,
    pub extents: Vec<Extent>,
    pub segments: Vec<Segment>,
    pub lists: Vec<u32>,
    pub tiles: Vec<Tile>,
    // Scratch, kept between frames.
    pairs: Vec<(u32, u32)>,
    counts: Vec<u32>,
    marks: Vec<bool>,
}
impl Prepared {
    pub fn clear(&mut self) {
        self.headers.clear();
        self.extents.clear();
        self.segments.clear();
        self.lists.clear();
        self.tiles.clear();
    }
    /// Little-endian GPU bytes: headers, segments, lists and tiles.
    pub fn encode(&self, out: &mut [Vec<u8>; 4]) {
        for bytes in out.iter_mut() {
            bytes.clear();
        }
        for header in &self.headers {
            for value in [
                header.origin[0],
                header.origin[1],
                header.h.to_bits(),
                header.thin.to_bits(),
            ] {
                out[0].extend_from_slice(&value.to_le_bytes());
            }
            for value in header.color {
                out[0].extend_from_slice(&value.to_le_bytes());
            }
        }
        for segment in &self.segments {
            segment.write(&mut out[1]);
        }
        for index in &self.lists {
            out[2].extend_from_slice(&index.to_le_bytes());
        }
        for tile in &self.tiles {
            for value in tile
                .rect
                .into_iter()
                .chain([tile.start, tile.count, tile.stroke, 0])
            {
                out[3].extend_from_slice(&value.to_le_bytes());
            }
        }
    }
}

/// What the preparation records for a stroke's tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tiling {
    /// Each tile with the list of segments that can cover its pixels, for
    /// the tiles path and the CPU.
    Lists,
    /// Only which cells a segment can cover, merged into runs along each
    /// row, for the quads path's composite; no lists.
    Cells,
}

/// What one stroke's preparation produced.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub segments: usize,
    pub references: usize,
    pub tiles: usize,
    pub longest: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A value would lie beyond `DOMAIN`.
    Domain,
    /// Over a budget.
    TooComplex,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Refusal {}

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
fn length(a: V) -> f64 {
    // Not `hypot`: several times slower, and the domain keeps the squares
    // far from overflow.
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}
fn unit(a: V) -> Option<V> {
    let l = length(a);
    (l > 0.0 && l.is_finite()).then(|| mul(a, 1.0 / l))
}
fn normal(d: V) -> V {
    [-d[1], d[0]]
}
fn narrow(v: V) -> [f32; 2] {
    [v[0] as f32, v[1] as f32]
}

// The join at a vertex between two unit directions, as union.rs builds it.
#[derive(Clone, Copy)]
struct JoinInfo {
    kind: u32,
    side_negative: bool,
    m: V,
    k: f64,
    ratio: f64,
}
fn join_at(d0: V, d1: V, h: f64, join: Join, limit: f64) -> JoinInfo {
    let none = JoinInfo {
        kind: 0,
        side_negative: false,
        m: [0.0; 2],
        k: 0.0,
        ratio: 1.0,
    };
    let turn = cross(d0, d1);
    if turn == 0.0 && dot(d0, d1) > 0.0 {
        return none;
    }
    let side = if turn > 0.0 { -1.0 } else { 1.0 };
    if join == Join::Round {
        return JoinInfo {
            kind: 1,
            side_negative: side < 0.0,
            ..none
        };
    }
    // The outer bisector: from the sum of the normals for a turn below 90
    // degrees and from the difference of the directions otherwise, where
    // the outer corner lies, as union.rs places a round join's. Each is well
    // conditioned where it is used; the sum of the normals of a near
    // reversal is rounding noise.
    let outer = if dot(d0, d1) > 0.0 {
        unit(add(normal(d0), normal(d1))).map(|m| mul(m, side))
    } else {
        unit(sub(d0, d1))
    };
    let Some(m) = outer else {
        return none;
    };
    let cos_half = dot(m, mul(normal(d0), side));
    let k = h * cos_half;
    if cos_half <= 0.0 {
        // A reversal: the bevel line is the perpendicular through the vertex.
        return JoinInfo {
            kind: 2,
            side_negative: side < 0.0,
            m: d0,
            k: 0.0,
            ratio: 1.0,
        };
    }
    if join == Join::Miter && cos_half > 0.0 && 1.0 / cos_half <= limit {
        JoinInfo {
            kind: 3,
            side_negative: side < 0.0,
            m,
            k,
            ratio: 1.0 / cos_half,
        }
    } else {
        JoinInfo {
            kind: 2,
            side_negative: side < 0.0,
            m,
            k,
            ratio: 1.0,
        }
    }
}

// The parameters t0 <= t1 of a..b inside `rect` (x0, y0, x1, y1).
fn clip_segment(a: V, b: V, rect: [f64; 4]) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = sub(b, a);
    for (p, q) in [
        (-d[0], a[0] - rect[0]),
        (d[0], rect[2] - a[0]),
        (-d[1], a[1] - rect[1]),
        (d[1], rect[3] - a[1]),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t0 <= t1).then_some((t0, t1))
}

fn point_rect_distance(p: V, rect: [f64; 4]) -> f64 {
    let dx = (rect[0] - p[0]).max(p[0] - rect[2]).max(0.0);
    let dy = (rect[1] - p[1]).max(p[1] - rect[3]).max(0.0);
    dx.hypot(dy)
}
fn point_segment_distance(p: V, a: V, b: V) -> f64 {
    let d = sub(b, a);
    let l = dot(d, d);
    let t = if l > 0.0 {
        (dot(sub(p, a), d) / l).clamp(0.0, 1.0)
    } else {
        0.0
    };
    length(sub(p, add(a, mul(d, t))))
}
fn segment_rect_distance(a: V, b: V, rect: [f64; 4]) -> f64 {
    if clip_segment(a, b, rect).is_some() {
        return 0.0;
    }
    let corners = [
        [rect[0], rect[1]],
        [rect[2], rect[1]],
        [rect[0], rect[3]],
        [rect[2], rect[3]],
    ];
    corners
        .iter()
        .map(|&c| point_segment_distance(c, a, b))
        .chain([point_rect_distance(a, rect), point_rect_distance(b, rect)])
        .fold(f64::INFINITY, f64::min)
}

// A segment before narrowing, with what its tile test needs.
struct Draft {
    record: Segment,
    a: V,
    b: V,
    body_reach: f64,
    join_reach: f64,
}

/// Prepares one stroke for a frame and appends it to `out`; `None` when it
/// draws nothing there.
pub fn prepare(
    stroke: &Stroke,
    frame: Frame,
    tile: u32,
    tiling: Tiling,
    out: &mut Prepared,
) -> Result<Option<Stats>, Refusal> {
    let factor = frame.scale * f64::from(stroke.scale);
    let width = f64::from(stroke.width) * factor;
    // A NaN width compares false, so it draws nothing too.
    if width.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || stroke.color[3] == 0.0 {
        return Ok(None);
    }
    let (h, thin) = if width < 1.0 {
        (0.5, width)
    } else {
        (width / 2.0, 1.0)
    };
    let map = |p: [f32; 2]| -> V {
        [
            (f64::from(stroke.origin[0]) + f64::from(p[0]) * f64::from(stroke.scale)) * frame.scale,
            (f64::from(stroke.origin[1]) + f64::from(p[1]) * f64::from(stroke.scale)) * frame.scale,
        ]
    };
    // Normalized subpaths, as union.rs's `pieces`.
    let mut paths: Vec<(Vec<V>, bool)> = Vec::new();
    for subpath in &stroke.subpaths {
        let mut points: Vec<V> = Vec::with_capacity(subpath.points.len());
        for &p in &subpath.points {
            let p = map(p);
            if points.last() != Some(&p) {
                points.push(p);
            }
        }
        if subpath.closed && points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        if points.is_empty() || (points.len() == 1 && stroke.cap == Cap::Butt) {
            continue;
        }
        let closed = subpath.closed && points.len() > 2;
        paths.push((points, closed));
    }
    if paths.is_empty() {
        return Ok(None);
    }
    let limit = f64::from(stroke.miter_limit);
    // Directions per segment, joins per vertex and the largest miter ratio
    // that occurs.
    let mut directions: Vec<Vec<Option<V>>> = Vec::with_capacity(paths.len());
    let mut joins: Vec<Vec<Option<JoinInfo>>> = Vec::with_capacity(paths.len());
    let mut ratio = 1.0f64;
    for (points, closed) in &paths {
        let n = points.len();
        let mut at = vec![None; n];
        let ds: Vec<Option<V>> = (0..n)
            .map(|i| unit(sub(points[(i + 1) % n], points[i])))
            .collect();
        if n > 1 {
            let direction = |i: usize| ds[i];
            for (i, slot) in at.iter_mut().enumerate() {
                let interior = if *closed { true } else { i > 0 && i + 1 < n };
                if !interior {
                    continue;
                }
                let (Some(d0), Some(d1)) = (direction((i + n - 1) % n), direction(i)) else {
                    continue;
                };
                let info = join_at(d0, d1, h, stroke.join, limit);
                ratio = ratio.max(info.ratio);
                *slot = Some(info);
            }
        }
        joins.push(at);
        directions.push(ds);
    }
    let r = ratio.max(if stroke.cap == Cap::Square {
        std::f64::consts::SQRT_2
    } else {
        1.0
    });
    let support = (h + 0.5) * r * (1.0 + r * 2f64.powi(-22)) + 1.0;
    // The span: the points' bounds grown by the support, in the frame and
    // the clip by the pixel-center rule.
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for (points, _) in &paths {
        for p in points {
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        }
    }
    let mut span = [
        (lo[0] - support).floor().max(0.0),
        (lo[1] - support).floor().max(0.0),
        (hi[0] + support).ceil().min(f64::from(frame.width)),
        (hi[1] + support).ceil().min(f64::from(frame.height)),
    ];
    if let Some([x, y, w, h]) = stroke.clip {
        let edge = |v: f32| (f64::from(v) * frame.scale - 0.5).ceil();
        span[0] = span[0].max(edge(x));
        span[1] = span[1].max(edge(y));
        span[2] = span[2].min(edge(x + w));
        span[3] = span[3].min(edge(y + h));
    }
    if !(span[2] > span[0] && span[3] > span[1]) {
        return Ok(None);
    }
    if support + (span[2] - span[0]).max(span[3] - span[1]) > DOMAIN {
        return Err(Refusal::Domain);
    }
    let grown = [
        span[0] - support,
        span[1] - support,
        span[2] + support,
        span[3] + support,
    ];
    let origin = [span[0], span[1]];
    let rel = |p: V| narrow(sub(p, origin));
    let cap_bits = match stroke.cap {
        Cap::Butt => 0,
        Cap::Round => CAP_ROUND,
        Cap::Square => CAP_SQUARE,
    } << CAP_SHIFT;
    let ramp = h + 0.5 + MARGIN;
    let pad = 0.25;
    let mut drafts: Vec<Draft> =
        Vec::with_capacity(paths.iter().map(|(points, _)| points.len()).sum());
    let within =
        |p: V| p[0] >= grown[0] && p[0] <= grown[2] && p[1] >= grown[1] && p[1] <= grown[3];
    for (((points, closed), at), ds) in paths.iter().zip(&joins).zip(&directions) {
        let n = points.len();
        if n == 1 {
            let p = points[0];
            if point_rect_distance(p, grown) == 0.0 {
                let square = stroke.cap == Cap::Square;
                drafts.push(Draft {
                    record: Segment {
                        a: rel(p),
                        b: rel(p),
                        flags: POINT | cap_bits,
                        ..Segment::default()
                    },
                    a: sub(p, origin),
                    b: sub(p, origin),
                    body_reach: ramp
                        * if square {
                            std::f64::consts::SQRT_2
                        } else {
                            1.0
                        }
                        + pad,
                    join_reach: 0.0,
                });
            }
            continue;
        }
        let count = if *closed { n } else { n - 1 };
        for i in 0..count {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let Some(d) = ds[i] else {
                continue;
            };
            let clipped = if within(a) && within(b) {
                Some((0.0, 1.0))
            } else {
                clip_segment(a, b, grown)
            };
            let Some((t0, t1)) = clipped else {
                continue;
            };
            let start_join = at[i];
            let end_join = at[(i + 1) % n];
            let start_joined = *closed || i > 0;
            let end_joined = *closed || i + 1 < n - 1;
            let mut flags = cap_bits;
            let mut record = Segment {
                d: narrow(d),
                ..Segment::default()
            };
            let ca = if t0 > 0.0 {
                flags |= OPEN << START_SHIFT;
                add(a, mul(sub(b, a), t0))
            } else {
                if start_joined {
                    flags |= JOINED << START_SHIFT;
                    if let Some(join) = start_join.filter(|j| j.kind == 2) {
                        flags |= START_BEVEL;
                        record.start_m = narrow(join.m);
                        record.start_k = join.k as f32;
                    }
                } else {
                    flags |= CAPPED << START_SHIFT;
                }
                a
            };
            let mut join_reach = 0.0;
            let cb = if t1 < 1.0 {
                flags |= OPEN << END_SHIFT;
                add(a, mul(sub(b, a), t1))
            } else {
                if end_joined {
                    flags |= JOINED << END_SHIFT;
                    if let Some(join) = end_join.filter(|j| j.kind != 0) {
                        flags |= join.kind << JOIN_SHIFT;
                        if join.side_negative {
                            flags |= SIDE_NEGATIVE;
                        }
                        let next = ds[(i + 1) % n].unwrap_or(d);
                        record.n = narrow(next);
                        if join.kind == 2 {
                            flags |= END_BEVEL;
                            record.end_m = narrow(join.m);
                            record.end_k = join.k as f32;
                        }
                        join_reach = ramp * join.ratio * (1.0 + join.ratio * 2f64.powi(-22)) + pad;
                        if join.kind == 3 {
                            // The distance does not read a miter's end k:
                            // the quads path's wedge square takes its reach.
                            record.end_k = join_reach as f32;
                        }
                    }
                } else {
                    flags |= CAPPED << END_SHIFT;
                }
                b
            };
            // Vertices are narrowed once, so a shared vertex is the same in
            // both records.
            record.a = rel(ca);
            record.b = rel(cb);
            record.flags = flags;
            let square_end = stroke.cap == Cap::Square
                && (flags & 3 == CAPPED || (flags >> END_SHIFT) & 3 == CAPPED);
            drafts.push(Draft {
                record,
                a: sub(ca, origin),
                b: sub(cb, origin),
                body_reach: ramp
                    * if square_end {
                        std::f64::consts::SQRT_2
                    } else {
                        1.0
                    }
                    + pad,
                join_reach,
            });
        }
    }
    if drafts.is_empty() {
        return Ok(None);
    }
    let size = [(span[2] - span[0]) as u32, (span[3] - span[1]) as u32];
    let first_tile = out.tiles.len() as u32;
    if tiling == Tiling::Cells {
        let runs = cells(&drafts, size, tile, span, out);
        record(out, stroke, span, h, thin, &drafts, first_tile);
        return Ok(Some(Stats {
            segments: drafts.len(),
            references: 0,
            tiles: runs,
            longest: 0,
        }));
    }
    // Tile lists: every segment that can have a candidate below half a pixel
    // plus the margin at one of a tile's pixel centers.
    let grid = [size[0].div_ceil(tile), size[1].div_ceil(tile)];
    let tile_f = f64::from(tile);
    let pairs = &mut out.pairs;
    pairs.clear();
    for (index, draft) in drafts.iter().enumerate() {
        let reach = draft.body_reach.max(draft.join_reach);
        let lo = [
            draft.a[0].min(draft.b[0]) - reach,
            draft.a[1].min(draft.b[1]) - reach,
        ];
        let hi = [
            draft.a[0].max(draft.b[0]) + reach,
            draft.a[1].max(draft.b[1]) + reach,
        ];
        let cell = |v: f64, n: u32| ((v / tile_f).floor().max(0.0) as u32).min(n);
        let (x0, x1) = (cell(lo[0], grid[0]), cell(hi[0], grid[0] - 1));
        let (y0, y1) = (cell(lo[1], grid[1]), cell(hi[1], grid[1] - 1));
        if hi[0] < 0.0 || hi[1] < 0.0 || x0 >= grid[0] || y0 >= grid[1] {
            continue;
        }
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let centers = [
                    f64::from(tx * tile) + 0.5,
                    f64::from(ty * tile) + 0.5,
                    f64::from(((tx + 1) * tile).min(size[0])) - 0.5,
                    f64::from(((ty + 1) * tile).min(size[1])) - 0.5,
                ];
                let near = segment_rect_distance(draft.a, draft.b, centers) <= draft.body_reach
                    || (draft.join_reach > 0.0
                        && point_rect_distance(draft.b, centers) <= draft.join_reach);
                if near {
                    pairs.push((ty * grid[0] + tx, index as u32));
                }
            }
        }
        if pairs.len() > MAX_REFERENCES {
            return Err(Refusal::TooComplex);
        }
    }
    // Lists in tile order, each in segment order.
    let tiles = (grid[0] * grid[1]) as usize;
    let counts = &mut out.counts;
    counts.clear();
    counts.resize(tiles + 1, 0);
    for &(t, _) in pairs.iter() {
        counts[t as usize + 1] += 1;
    }
    let longest = counts.iter().copied().max().unwrap_or(0) as usize;
    if longest > MAX_LIST {
        return Err(Refusal::TooComplex);
    }
    for i in 0..tiles {
        counts[i + 1] += counts[i];
    }
    let base_segment = out.segments.len() as u32;
    let base_list = out.lists.len();
    out.lists.resize(base_list + pairs.len(), 0);
    let mut next = counts[..tiles].to_vec();
    for &(t, s) in pairs.iter() {
        let slot = &mut next[t as usize];
        out.lists[base_list + *slot as usize] = base_segment + s;
        *slot += 1;
    }
    let stroke_index = out.headers.len() as u32;
    let mut recorded = 0;
    for t in 0..tiles {
        let (start, end) = (counts[t], counts[t + 1]);
        if start == end {
            continue;
        }
        let (tx, ty) = (t as u32 % grid[0], t as u32 / grid[0]);
        let ox = span[0] as u32;
        let oy = span[1] as u32;
        out.tiles.push(Tile {
            rect: [
                ox + tx * tile,
                oy + ty * tile,
                ox + ((tx + 1) * tile).min(size[0]),
                oy + ((ty + 1) * tile).min(size[1]),
            ],
            start: base_list as u32 + start,
            count: end - start,
            stroke: stroke_index,
        });
        recorded += 1;
    }
    let references = pairs.len();
    record(out, stroke, span, h, thin, &drafts, first_tile);
    Ok(Some(Stats {
        segments: drafts.len(),
        references,
        tiles: recorded,
        longest,
    }))
}

// Appends a prepared stroke's header, extent and segments.
fn record(
    out: &mut Prepared,
    stroke: &Stroke,
    span: [f64; 4],
    h: f64,
    thin: f64,
    drafts: &[Draft],
    first_tile: u32,
) {
    let stroke_index = out.headers.len() as u32;
    let base_segment = out.segments.len() as u32;
    let [red, green, blue, alpha] = stroke.color;
    out.headers.push(Header {
        origin: [span[0] as u32, span[1] as u32],
        h: h as f32,
        thin: thin as f32,
        color: [red * alpha, green * alpha, blue * alpha, alpha],
    });
    out.extents.push(Extent {
        span: span.map(|v| v as u32),
        miters: drafts
            .iter()
            .any(|d| (d.record.flags >> JOIN_SHIFT) & 3 == 3),
        segments: base_segment..base_segment + drafts.len() as u32,
        tiles: first_tile..out.tiles.len() as u32,
    });
    out.segments.extend(drafts.iter().map(|d| Segment {
        stroke: stroke_index,
        ..d.record
    }));
}

// Marks every cell holding a pixel center within a segment's reach, row band
// by row band, and records the marked cells as runs along each row; returns
// how many runs. A center within `reach` of a segment point lies in a row
// band grown by `reach`, at most `reach` from the part of the segment inside
// that band.
fn cells(drafts: &[Draft], size: [u32; 2], tile: u32, span: [f64; 4], out: &mut Prepared) -> usize {
    let grid = [size[0].div_ceil(tile), size[1].div_ceil(tile)];
    let side = f64::from(tile);
    let marks = &mut out.marks;
    marks.clear();
    marks.resize((grid[0] * grid[1]) as usize, false);
    let index = |v: f64, n: u32| (v / side).floor().clamp(0.0, f64::from(n - 1)) as u32;
    for draft in drafts {
        let reach = draft.body_reach.max(draft.join_reach);
        let (a, b) = (draft.a, draft.b);
        let (low, high) = (
            [a[0].min(b[0]) - reach, a[1].min(b[1]) - reach],
            [a[0].max(b[0]) + reach, a[1].max(b[1]) + reach],
        );
        if high[0] < 0.0
            || high[1] < 0.0
            || low[0] >= f64::from(size[0])
            || low[1] >= f64::from(size[1])
        {
            continue;
        }
        let dy = b[1] - a[1];
        for ty in index(low[1], grid[1])..=index(high[1], grid[1]) {
            let band = [
                f64::from(ty) * side - reach,
                f64::from(ty + 1) * side + reach,
            ];
            let (t0, t1) = if dy == 0.0 {
                (0.0, 1.0)
            } else {
                let (ta, tb) = ((band[0] - a[1]) / dy, (band[1] - a[1]) / dy);
                (ta.min(tb).max(0.0), ta.max(tb).min(1.0))
            };
            if t0 > t1 {
                continue;
            }
            let x0 = a[0] + (b[0] - a[0]) * t0;
            let x1 = a[0] + (b[0] - a[0]) * t1;
            let row = (ty * grid[0]) as usize;
            for tx in index(x0.min(x1) - reach, grid[0])..=index(x0.max(x1) + reach, grid[0]) {
                marks[row + tx as usize] = true;
            }
        }
    }
    let stroke = out.headers.len() as u32;
    let (ox, oy) = (span[0] as u32, span[1] as u32);
    let mut runs = 0;
    for ty in 0..grid[1] {
        let row = (ty * grid[0]) as usize;
        let mut tx = 0;
        while tx < grid[0] {
            if !marks[row + tx as usize] {
                tx += 1;
                continue;
            }
            let start = tx;
            while tx < grid[0] && marks[row + tx as usize] {
                tx += 1;
            }
            out.tiles.push(Tile {
                rect: [
                    ox + start * tile,
                    oy + ty * tile,
                    ox + (tx * tile).min(size[0]),
                    oy + ((ty + 1) * tile).min(size[1]),
                ],
                start: 0,
                count: 0,
                stroke,
            });
            runs += 1;
        }
    }
    runs
}

/// The length of (x, y), exact on an axis, as `shapes.wgsl`'s `span`.
fn magnitude(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        return y.abs();
    }
    if y == 0.0 {
        return x.abs();
    }
    (x * x + y * y).sqrt()
}

/// The smallest candidate of one segment at `p`, relative to the span's
/// origin; infinite when it has none. `stroke.wgsl` evaluates the same.
pub fn segment_distance(s: &Segment, h: f32, p: [f32; 2]) -> f32 {
    let f = s.flags;
    let cap = (f >> CAP_SHIFT) & 3;
    let rx = p[0] - s.a[0];
    let ry = p[1] - s.a[1];
    if f & POINT != 0 {
        return if cap == CAP_SQUARE {
            rx.abs().max(ry.abs()) - h
        } else {
            magnitude(rx, ry) - h
        };
    }
    let wx = p[0] - s.b[0];
    let wy = p[1] - s.b[1];
    let u = rx * s.d[0] + ry * s.d[1];
    let v = wx * s.d[0] + wy * s.d[1];
    let start = (f >> START_SHIFT) & 3;
    let end = (f >> END_SHIFT) & 3;
    let mut best = f32::INFINITY;
    if (start != JOINED || u >= 0.0) && (end != JOINED || v <= 0.0) {
        let across = s.d[0] * ry - s.d[1] * rx;
        let mut distance = across.abs() - h;
        if start == CAPPED {
            let e = -u;
            distance = distance.max(if cap == CAP_SQUARE { e - h } else { e });
        }
        if end == CAPPED {
            let e = v;
            distance = distance.max(if cap == CAP_SQUARE { e - h } else { e });
        }
        if f & START_BEVEL != 0 {
            distance = distance.max(rx * s.start_m[0] + ry * s.start_m[1] - s.start_k);
        }
        if f & END_BEVEL != 0 {
            distance = distance.max(wx * s.end_m[0] + wy * s.end_m[1] - s.end_k);
        }
        best = distance;
    }
    let join = (f >> JOIN_SHIFT) & 3;
    if join != 0 && v > 0.0 {
        let along_next = wx * s.n[0] + wy * s.n[1];
        if along_next < 0.0 {
            let candidate = match join {
                1 => magnitude(wx, wy) - h,
                2 => (magnitude(wx, wy) - h).max(wx * s.end_m[0] + wy * s.end_m[1] - s.end_k),
                _ => {
                    let side = if f & SIDE_NEGATIVE != 0 { -1.0 } else { 1.0 };
                    let (fx, fy) = (-s.d[1] * side, s.d[0] * side);
                    let (gx, gy) = (-s.n[1] * side, s.n[0] * side);
                    (wx * fx + wy * fy).max(wx * gx + wy * gy) - h
                }
            };
            best = best.min(candidate);
        }
    }
    if cap == CAP_ROUND {
        if start == CAPPED {
            best = best.min(magnitude(rx, ry) - h);
        }
        if end == CAPPED {
            best = best.min(magnitude(wx, wy) - h);
        }
    }
    best
}

/// The coverage of a distance, as for shapes, times the thin-stroke factor.
pub fn coverage(distance: f32, thin: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0) * thin
}

/// The reference at frame pixel (x, y) of one tile: the smallest distance
/// over its list (infinite for none) and the coverage.
pub fn evaluate(prepared: &Prepared, tile: &Tile, x: u32, y: u32) -> (f32, f32) {
    let header = &prepared.headers[tile.stroke as usize];
    let p = [
        (x - header.origin[0]) as f32 + 0.5,
        (y - header.origin[1]) as f32 + 0.5,
    ];
    let list = &prepared.lists[tile.start as usize..(tile.start + tile.count) as usize];
    let distance = list
        .iter()
        .map(|&i| segment_distance(&prepared.segments[i as usize], header.h, p))
        .fold(f32::INFINITY, f32::min);
    (distance, coverage(distance, header.thin))
}

/// Whether a reference distance decides its pixel: at least half a pixel
/// plus the margin from the boundary on either side.
pub fn decided(distance: f32) -> bool {
    f64::from(distance).abs() >= 0.5 + MARGIN
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            width: 64,
            height: 64,
            scale: 1.0,
        }
    }
    // The coverage at frame pixel (x, y) over every segment, ignoring tiles.
    fn brute(prepared: &Prepared, x: u32, y: u32) -> f32 {
        let header = &prepared.headers[0];
        if x < header.origin[0] || y < header.origin[1] {
            return f32::INFINITY;
        }
        let p = [
            (x - header.origin[0]) as f32 + 0.5,
            (y - header.origin[1]) as f32 + 0.5,
        ];
        prepared
            .segments
            .iter()
            .map(|s| segment_distance(s, header.h, p))
            .fold(f32::INFINITY, f32::min)
    }

    #[test]
    fn a_horizontal_line_covers_its_rows() {
        let stroke = Stroke::polyline(vec![[8.0, 20.0], [40.0, 20.0]], 2.0, Cap::Butt, Join::Miter);
        let mut prepared = Prepared::default();
        prepare(&stroke, frame(), 16, Tiling::Lists, &mut prepared)
            .unwrap()
            .unwrap();
        let at = |x, y| coverage(brute(&prepared, x, y), 1.0);
        assert_eq!(at(20, 19), 1.0);
        assert_eq!(at(20, 20), 1.0);
        assert_eq!(at(20, 18), 0.0);
        assert_eq!(at(20, 21), 0.0);
        // Butt ends at x = 8 and 40.
        assert_eq!(at(7, 19), 0.0);
        assert_eq!(at(8, 19), 1.0);
        assert_eq!(at(39, 19), 1.0);
        assert_eq!(at(40, 19), 0.0);
    }

    #[test]
    fn tile_lists_hold_every_segment_that_covers_a_pixel() {
        for join in [Join::Round, Join::Bevel, Join::Miter] {
            for cap in [Cap::Butt, Cap::Round, Cap::Square] {
                let points = (0..40)
                    .map(|i| {
                        let t = i as f32;
                        [4.0 + t * 1.4, 32.0 + 20.0 * (t * 0.9).sin()]
                    })
                    .collect();
                let mut stroke = Stroke::polyline(points, 3.0, cap, join);
                stroke.miter_limit = 10.0;
                let mut prepared = Prepared::default();
                prepare(&stroke, frame(), 8, Tiling::Lists, &mut prepared)
                    .unwrap()
                    .unwrap();
                let mut seen = vec![false; 64 * 64];
                for tile in &prepared.tiles {
                    for y in tile.rect[1]..tile.rect[3] {
                        for x in tile.rect[0]..tile.rect[2] {
                            let (listed, _) = evaluate(&prepared, tile, x, y);
                            let all = brute(&prepared, x, y);
                            assert!(
                                listed == all
                                    || (all >= 0.5 + MARGIN as f32
                                        && listed >= 0.5 + MARGIN as f32),
                                "{join:?} {cap:?} ({x}, {y}): listed {listed}, all {all}"
                            );
                            seen[(y * 64 + x) as usize] = true;
                        }
                    }
                }
                // A pixel outside every tile is uncovered.
                for y in 0..64 {
                    for x in 0..64 {
                        if !seen[(y * 64 + x) as usize] {
                            assert!(brute(&prepared, x, y) >= 0.5 + MARGIN as f32, "({x}, {y})");
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod join_tests {
    use super::*;

    #[test]
    fn a_near_reversal_has_the_reversal_bevel() {
        // Rounded directions of an exact reversal in frame pixels.
        let d0 = unit([100.000_007_629_394_53, 0.0]).unwrap();
        let d1 = unit([-60.000_007_629_394_53, 0.0]).unwrap();
        for (a, b) in [(d0, d1), (d1, d0), ([1.0, 0.0], [-1.0, 0.0])] {
            let join = join_at(a, b, 1.5, Join::Bevel, 4.0);
            assert_eq!((join.kind, join.k), (2, 0.0));
            assert_eq!(join.m, a);
        }
    }

    #[test]
    fn the_bevel_line_lies_on_the_outer_side() {
        // A right-angle turn from +x to +y: the outer corner is at (+, -).
        let join = join_at([1.0, 0.0], [0.0, 1.0], 2.0, Join::Bevel, 4.0);
        let half = std::f64::consts::FRAC_1_SQRT_2;
        assert!((join.m[0] - half).abs() < 1e-12 && (join.m[1] + half).abs() < 1e-12);
        assert!((join.k - 2.0 * half).abs() < 1e-12);
        // An obtuse turn takes the other branch and agrees with it.
        let d1 = unit([-1.0, 1.0]).unwrap();
        let obtuse = join_at([1.0, 0.0], d1, 2.0, Join::Miter, 10.0);
        let sum = unit(add(normal([1.0, 0.0]), normal(d1))).unwrap();
        let expected = mul(sum, -1.0);
        assert!(
            (obtuse.m[0] - expected[0]).abs() < 1e-12 && (obtuse.m[1] - expected[1]).abs() < 1e-12
        );
    }
}
