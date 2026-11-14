// Stroke coverage: one instance per segment record of a prepared stroke,
// drawn into the stroke's region of a coverage page with a maximum blend.
// The fragment stage evaluates, in f32 and in the same order, the candidates
// of `pert_ui_runtime::StrokeSegment::distance`, which is the reference. The
// maximum of the segments' coverage is the coverage of their smallest
// distance, so a page holds `PreparedStroke::coverage` up to a half float's
// precision.
// `strokes.rs` documents the records.
struct Segment {
    // a, b.
    g0: vec4<f32>,
    // d, n.
    g1: vec4<f32>,
    // End m, end k (a miter's reach), start k.
    g2: vec4<f32>,
    start_m: vec2<f32>,
    flags: u32,
    // The stroke's index among the frame's strokes.
    stroke: u32,
};
struct Stroke {
    // h, the thin-stroke factor, and where the span's origin lies in its
    // page, in pixels.
    place: vec4<f32>,
    // Two over the page's width and height; unused.
    scale: vec4<f32>,
};
@group(0) @binding(0) var<storage, read> segments: array<Segment>;
@group(0) @binding(1) var<storage, read> strokes: array<Stroke>;

const JOINED: u32 = 0u;
const CAPPED: u32 = 1u;
const START_BEVEL: u32 = 4u;
const END_BEVEL: u32 = 32u;
const SIDE_NEGATIVE: u32 = 256u;
const POINT: u32 = 2048u;
const CAP_ROUND: u32 = 1u;
const CAP_SQUARE: u32 = 2u;
const INFINITY: f32 = 3.0e38;
// The parity margin, and what covers the vertices' f32 rounding.
const MARGIN: f32 = 0.0625;
const PAD: f32 = 0.015625;

// The segment's record and its stroke's values travel as flat varyings, so
// the fragment stage reads no buffer.
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) g0: vec4<f32>,
    @location(1) @interpolate(flat) g1: vec4<f32>,
    @location(2) @interpolate(flat) g2: vec4<f32>,
    @location(3) @interpolate(flat) start_m: vec2<f32>,
    @location(4) @interpolate(flat) place: vec4<f32>,
    @location(5) @interpolate(flat) flags: u32,
};

// Triangle strips. Vertices 0-3 are a quad along the segment that holds
// every pixel center at which its body, its caps and a round or bevel join's
// wedge can have a candidate below half a pixel: `reach` across, and `reach`
// beyond an end unless the end is joined and nothing of this segment lies
// past it. A round cap's disc reaches behind the other end too. Vertices
// 4-7, drawn only for strokes with a miter, are the square around a miter's
// wedge, of the half size its record carries, and degenerate elsewhere. The
// scissor keeps a stroke inside its region.
@vertex fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> VertexOutput {
    let s = segments[index];
    let stroke = strokes[s.stroke];
    let h = stroke.place.x;
    let f = s.flags;
    let right = (vertex & 2u) != 0u;
    let sign = vec2<f32>(select(-1.0, 1.0, right), select(-1.0, 1.0, (vertex & 1u) != 0u));
    let reach = h + 0.5 + MARGIN + PAD;
    var position: vec2<f32>;
    if (f & POINT) != 0u {
        position = s.g0.xy + sign * select(reach, 0.0, vertex >= 4u);
    } else if vertex >= 4u {
        let miter = ((f >> 6u) & 3u) == 3u;
        position = s.g0.zw + sign * select(0.0, s.g2.z + PAD, miter);
    } else {
        let d = s.g1.xy;
        let join = (f >> 6u) & 3u;
        let start = f & 3u;
        let end = (f >> 3u) & 3u;
        let disc = ((f >> 9u) & 3u) == CAP_ROUND && (start == CAPPED || end == CAPPED);
        let before = select(reach, PAD, start == JOINED && !disc);
        let after = select(reach, PAD, end == JOINED && (join == 0u || join == 3u) && !disc);
        let base = select(s.g0.xy - d * before, s.g0.zw + d * after, right);
        position = base + vec2<f32>(-d.y, d.x) * (sign.y * reach);
    }
    let placed = stroke.place.zw + position;
    var output: VertexOutput;
    output.position = vec4<f32>(placed.x * stroke.scale.x - 1.0, 1.0 - placed.y * stroke.scale.y, 0.0, 1.0);
    output.g0 = s.g0;
    output.g1 = s.g1;
    output.g2 = s.g2;
    output.start_m = s.start_m;
    output.place = stroke.place;
    output.flags = f;
    return output;
}

// The length of (x, y), without a square root on an axis.
fn magnitude(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        return abs(y);
    }
    if y == 0.0 {
        return abs(x);
    }
    return sqrt(x * x + y * y);
}

fn distance_at(input: VertexOutput, h: f32, p: vec2<f32>) -> f32 {
    let f = input.flags;
    let cap = (f >> 9u) & 3u;
    let rx = p.x - input.g0.x;
    let ry = p.y - input.g0.y;
    if (f & POINT) != 0u {
        if cap == CAP_SQUARE {
            return max(abs(rx), abs(ry)) - h;
        }
        return magnitude(rx, ry) - h;
    }
    let wx = p.x - input.g0.z;
    let wy = p.y - input.g0.w;
    let u = rx * input.g1.x + ry * input.g1.y;
    let v = wx * input.g1.x + wy * input.g1.y;
    let start = f & 3u;
    let end = (f >> 3u) & 3u;
    var best = INFINITY;
    if (start != JOINED || u >= 0.0) && (end != JOINED || v <= 0.0) {
        let across = input.g1.x * ry - input.g1.y * rx;
        var distance = abs(across) - h;
        if start == CAPPED {
            let beyond = -u;
            distance = max(distance, select(beyond, beyond - h, cap == CAP_SQUARE));
        }
        if end == CAPPED {
            distance = max(distance, select(v, v - h, cap == CAP_SQUARE));
        }
        if (f & START_BEVEL) != 0u {
            distance = max(distance, rx * input.start_m.x + ry * input.start_m.y - input.g2.w);
        }
        if (f & END_BEVEL) != 0u {
            distance = max(distance, wx * input.g2.x + wy * input.g2.y - input.g2.z);
        }
        best = distance;
    }
    let join = (f >> 6u) & 3u;
    if join != 0u && v > 0.0 && wx * input.g1.z + wy * input.g1.w < 0.0 {
        var candidate: f32;
        if join == 1u {
            candidate = magnitude(wx, wy) - h;
        } else if join == 2u {
            candidate = max(magnitude(wx, wy) - h, wx * input.g2.x + wy * input.g2.y - input.g2.z);
        } else {
            let side = select(1.0, -1.0, (f & SIDE_NEGATIVE) != 0u);
            let fx = -input.g1.y * side;
            let fy = input.g1.x * side;
            let gx = -input.g1.w * side;
            let gy = input.g1.z * side;
            candidate = max(wx * fx + wy * fy, wx * gx + wy * gy) - h;
        }
        best = min(best, candidate);
    }
    if cap == CAP_ROUND {
        if start == CAPPED {
            best = min(best, magnitude(rx, ry) - h);
        }
        if end == CAPPED {
            best = min(best, magnitude(wx, wy) - h);
        }
    }
    return best;
}

// A fragment without coverage writes zero, which the maximum blend ignores.
@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let p = input.position.xy - input.place.zw;
    let coverage = clamp(0.5 - distance_at(input, input.place.x, p), 0.0, 1.0) * input.place.y;
    return vec4<f32>(coverage, 0.0, 0.0, 0.0);
}
