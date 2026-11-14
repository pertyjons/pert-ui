// Strokes covered per pixel from their segments, by either path:
//
// - tiles: one instanced quad per recorded tile, each pixel taking the
//   smallest distance over the tile's list;
// - quads: one or two quads per segment into an R8 coverage atlas with a
//   maximum blend, then the stroke's tiles composited from the atlas. The
//   maximum of the segments' coverage is the coverage of their smallest
//   distance, so both paths draw the same coverage up to 8-bit rounding.
//
// `stroke.rs`'s `segment_distance` is the reference; the fragment stages
// evaluate the same expressions in the same order.
struct Frame {
    size: vec2<f32>,
    atlas: vec2<f32>,
};
struct Header {
    origin: vec2<u32>,
    h: f32,
    thin: f32,
    color: vec4<f32>,
};
// a, b; d, n; end m, end k (a miter's reach), start k; start m, flags, stroke.
struct Segment {
    g0: vec4<f32>,
    g1: vec4<f32>,
    g2: vec4<f32>,
    g3: vec4<f32>,
};
// x0, y0, x1, y1; list start, list count, stroke, unused.
struct Tile {
    rect: vec4<u32>,
    list: vec4<u32>,
};
@group(0) @binding(0) var<uniform> frame: Frame;
@group(0) @binding(1) var<storage, read> headers: array<Header>;
@group(0) @binding(2) var<storage, read> segments: array<Segment>;
@group(0) @binding(3) var<storage, read> lists: array<u32>;
@group(0) @binding(4) var<storage, read> tiles: array<Tile>;
// Per stroke: where its span's origin lies in the coverage atlas.
@group(0) @binding(5) var<storage, read> regions: array<vec4<u32>>;
@group(1) @binding(0) var atlas: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) tile: u32,
};

fn tile_corner(vertex: u32, tile: u32) -> vec4<f32> {
    let rect = vec4<f32>(tiles[tile].rect);
    let right = vertex >= 2u;
    let bottom = (vertex & 1u) == 1u;
    let x = select(rect.x, rect.z, right);
    let y = select(rect.y, rect.w, bottom);
    return vec4<f32>(2.0 * x / frame.size.x - 1.0, 1.0 - 2.0 * y / frame.size.y, 0.0, 1.0);
}

@vertex fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) tile: u32) -> VertexOutput {
    var output: VertexOutput;
    output.position = tile_corner(vertex, tile);
    output.tile = tile;
    return output;
}

const JOINED: u32 = 0u;
const CAPPED: u32 = 1u;
const START_BEVEL: u32 = 4u;
const END_BEVEL: u32 = 32u;
const SIDE_NEGATIVE: u32 = 256u;
const POINT: u32 = 2048u;
const CAP_ROUND: u32 = 1u;
const CAP_SQUARE: u32 = 2u;
const INFINITY: f32 = 3.0e38;

fn magnitude(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        return abs(y);
    }
    if y == 0.0 {
        return abs(x);
    }
    return sqrt(x * x + y * y);
}

fn segment_distance(s: Segment, h: f32, p: vec2<f32>) -> f32 {
    let f = bitcast<u32>(s.g3.z);
    let cap = (f >> 9u) & 3u;
    let rx = p.x - s.g0.x;
    let ry = p.y - s.g0.y;
    if (f & POINT) != 0u {
        if cap == CAP_SQUARE {
            return max(abs(rx), abs(ry)) - h;
        }
        return magnitude(rx, ry) - h;
    }
    let wx = p.x - s.g0.z;
    let wy = p.y - s.g0.w;
    let u = rx * s.g1.x + ry * s.g1.y;
    let v = wx * s.g1.x + wy * s.g1.y;
    let start = f & 3u;
    let end = (f >> 3u) & 3u;
    var best = INFINITY;
    if (start != JOINED || u >= 0.0) && (end != JOINED || v <= 0.0) {
        let across = s.g1.x * ry - s.g1.y * rx;
        var distance = abs(across) - h;
        if start == CAPPED {
            let e = -u;
            distance = max(distance, select(e, e - h, cap == CAP_SQUARE));
        }
        if end == CAPPED {
            let e = v;
            distance = max(distance, select(e, e - h, cap == CAP_SQUARE));
        }
        if (f & START_BEVEL) != 0u {
            distance = max(distance, rx * s.g3.x + ry * s.g3.y - s.g2.w);
        }
        if (f & END_BEVEL) != 0u {
            distance = max(distance, wx * s.g2.x + wy * s.g2.y - s.g2.z);
        }
        best = distance;
    }
    let join = (f >> 6u) & 3u;
    if join != 0u && v > 0.0 {
        let along_next = wx * s.g1.z + wy * s.g1.w;
        if along_next < 0.0 {
            var candidate: f32;
            if join == 1u {
                candidate = magnitude(wx, wy) - h;
            } else if join == 2u {
                candidate = max(magnitude(wx, wy) - h, wx * s.g2.x + wy * s.g2.y - s.g2.z);
            } else {
                let side = select(1.0, -1.0, (f & SIDE_NEGATIVE) != 0u);
                let fx = -s.g1.y * side;
                let fy = s.g1.x * side;
                let gx = -s.g1.w * side;
                let gy = s.g1.z * side;
                candidate = max(wx * fx + wy * fy, wx * gx + wy * gy) - h;
            }
            best = min(best, candidate);
        }
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

fn tile_coverage(input: VertexOutput) -> f32 {
    let tile = tiles[input.tile];
    let header = headers[tile.list.z];
    let p = input.position.xy - vec2<f32>(header.origin);
    var distance = INFINITY;
    for (var i = 0u; i < tile.list.y; i += 1u) {
        distance = min(distance, segment_distance(segments[lists[tile.list.x + i]], header.h, p));
    }
    return clamp(0.5 - distance, 0.0, 1.0) * header.thin;
}

@fragment fn color(input: VertexOutput) -> @location(0) vec4<f32> {
    let header = headers[tiles[input.tile].list.z];
    return header.color * tile_coverage(input);
}

// The coverage alone, into an R32Float target, for the parity comparison.
@fragment fn coverage(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(tile_coverage(input), 0.0, 0.0, 0.0);
}

// The quads path. One instance per segment, as triangle strips: vertices
// 0-3 cover the body, its caps and a round or bevel join's wedge, oriented
// along the segment (the wedge lies within `reach` of the end, a round
// cap's disc within `reach` of both); vertices 4-7, drawn only for strokes
// with miter joins, an axis-aligned square around a miter's wedge,
// degenerate elsewhere. Every pixel center at which the segment has a
// candidate below half a pixel lies inside them; the scissor keeps the
// stroke inside its atlas region. PAD covers the vertices' f32 rounding.
// A fragment without coverage writes zero, which the maximum blend ignores.
const MARGIN: f32 = 0.0625;
const PAD: f32 = 0.015625;

// The segment's record and its stroke's values travel as flat varyings, so
// the fragment stage reads no buffer.
struct SegmentOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) g0: vec4<f32>,
    @location(1) @interpolate(flat) g1: vec4<f32>,
    @location(2) @interpolate(flat) g2: vec4<f32>,
    @location(3) @interpolate(flat) g3: vec4<f32>,
    // h, thin, and the region's origin in the atlas.
    @location(4) @interpolate(flat) stroke: vec4<f32>,
};

@vertex fn segment_vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> SegmentOutput {
    let s = segments[index];
    let stroke = bitcast<u32>(s.g3.w);
    let h = headers[stroke].h;
    let region = vec2<f32>(regions[stroke].xy);
    let f = bitcast<u32>(s.g3.z);
    let right = (vertex & 2u) != 0u;
    let bottom = (vertex & 1u) != 0u;
    let sign = vec2<f32>(select(-1.0, 1.0, right), select(-1.0, 1.0, bottom));
    let reach = h + 0.5 + MARGIN + PAD;
    var position: vec2<f32>;
    if (f & POINT) != 0u {
        position = s.g0.xy + sign * select(reach, 0.0, vertex >= 4u);
    } else if vertex >= 4u {
        // A miter's record carries its wedge's reach in the end k.
        let miter = ((f >> 6u) & 3u) == 3u;
        position = s.g0.zw + sign * select(0.0, s.g2.z + PAD, miter);
    } else {
        let d = s.g1.xy;
        let join = (f >> 6u) & 3u;
        let start = f & 3u;
        let end = (f >> 3u) & 3u;
        // A round cap's disc reaches behind the segment's other end too.
        let disc = ((f >> 9u) & 3u) == CAP_ROUND && (start == CAPPED || end == CAPPED);
        let before = select(reach, PAD, start == JOINED && !disc);
        let after = select(reach, PAD, end == JOINED && (join == 0u || join == 3u) && !disc);
        let base = select(s.g0.xy - d * before, s.g0.zw + d * after, right);
        position = base + vec2<f32>(-d.y, d.x) * (sign.y * reach);
    }
    let placed = region + position;
    var output: SegmentOutput;
    output.position = vec4<f32>(2.0 * placed.x / frame.atlas.x - 1.0, 1.0 - 2.0 * placed.y / frame.atlas.y, 0.0, 1.0);
    output.g0 = s.g0;
    output.g1 = s.g1;
    output.g2 = s.g2;
    output.g3 = s.g3;
    output.stroke = vec4<f32>(h, headers[stroke].thin, region);
    return output;
}

@fragment fn segment_coverage(input: SegmentOutput) -> @location(0) vec4<f32> {
    let s = Segment(input.g0, input.g1, input.g2, input.g3);
    let p = input.position.xy - input.stroke.zw;
    let c = clamp(0.5 - segment_distance(s, input.stroke.x, p), 0.0, 1.0) * input.stroke.y;
    return vec4<f32>(c, 0.0, 0.0, 0.0);
}

// A tile composited from the atlas: the color and the offset from frame
// to atlas pixels travel as flat varyings.
struct CompositeOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) color: vec4<f32>,
    @location(1) @interpolate(flat) offset: vec2<i32>,
};

@vertex fn composite_vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) tile: u32) -> CompositeOutput {
    let stroke = tiles[tile].list.z;
    var output: CompositeOutput;
    output.position = tile_corner(vertex, tile);
    output.color = headers[stroke].color;
    output.offset = vec2<i32>(regions[stroke].xy) - vec2<i32>(headers[stroke].origin);
    return output;
}

fn composited(input: CompositeOutput) -> f32 {
    return textureLoad(atlas, vec2<i32>(input.position.xy) + input.offset, 0).r;
}

@fragment fn composite(input: CompositeOutput) -> @location(0) vec4<f32> {
    return input.color * composited(input);
}

@fragment fn composite_coverage(input: CompositeOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(composited(input), 0.0, 0.0, 0.0);
}
