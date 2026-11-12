// Analytic shapes: records of prepared shapes, read from a storage buffer by
// instance index; a rectangle's solid box and the strips around it have one
// record each. The fragment stage evaluates, in f32 and in the same
// order, the formulas of `pert_ui_runtime::PreparedShape::coverage`, which is
// the reference; `shapes.rs` documents the record.
struct Shape {
    // The drawn part of the span, its corners in normalized device
    // coordinates: left, top, right, bottom.
    ndc: vec4<f32>,
    // The span's origin in frame pixels (x, y), the thin-stroke factor, unused.
    origin: vec4<f32>,
    // Premultiplied linear paints.
    fill: vec4<f32>,
    border: vec4<f32>,
    // Rectangle: outer edges. Arc: center, radius, half width. Triangle: two vertices.
    a: vec4<f32>,
    // Rectangle: outer corner sizes. Arc: middle (cos, sin), half sweep (cos, sin).
    // Triangle: the third vertex, the winding sign.
    b: vec4<f32>,
    // Rectangle: inner edges.
    c: vec4<f32>,
    // Rectangle: inner corner sizes.
    d: vec4<f32>,
    // Kind (0 rectangle, 1 arc, 2 triangle); rectangle: inner (0 no border,
    // 1 all border, 2 inner area), arc: 1 for a full circle; corner kinds,
    // two bits each from the top left clockwise (1 round, 2 cut); 1 for a
    // rectangle's solid box, which is painted without a distance.
    flags: vec4<u32>,
};
@group(0) @binding(0) var<storage, read> shapes: array<Shape>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) record: u32,
};

@vertex fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) record: u32) -> VertexOutput {
    let ndc = shapes[record].ndc;
    // Two triangles: (l,t) (l,b) (r,t) (r,t) (l,b) (r,b).
    let right = vertex == 2u || vertex == 3u || vertex == 5u;
    let bottom = vertex == 1u || vertex == 4u || vertex == 5u;
    var output: VertexOutput;
    output.position = vec4<f32>(select(ndc.x, ndc.z, right), select(ndc.y, ndc.w, bottom), 0.0, 1.0);
    output.record = record;
    return output;
}

const SQRT_2: f32 = 1.4142135623730951;

// The coverage of a distance: one pixel wide, centered on the boundary.
fn ramp(distance: f32) -> f32 {
    return clamp(0.5 - distance, 0.0, 1.0);
}

// The length of (x, y); with a zero component it is the other one's magnitude,
// without a square root, so axis-aligned distances stay exact.
fn span(x: f32, y: f32) -> f32 {
    if x == 0.0 {
        return abs(y);
    }
    if y == 0.0 {
        return abs(x);
    }
    return sqrt(x * x + y * y);
}

// The signed distance bound of a rectangle boundary with per-corner sizes.
fn boundary(p: vec2<f32>, edges: vec4<f32>, sizes: vec4<f32>, kinds: u32) -> f32 {
    let qx = max(edges.x - p.x, p.x - edges.z);
    let qy = max(edges.y - p.y, p.y - edges.w);
    var distance = min(max(qx, qy), 0.0) + span(max(qx, 0.0), max(qy, 0.0));
    for (var corner = 0u; corner < 4u; corner += 1u) {
        let size = sizes[corner];
        if size <= 0.0 {
            continue;
        }
        let east = corner == 1u || corner == 2u;
        let south = corner >= 2u;
        // How far `p` lies inward of the corner's edges, per axis.
        let ix = select(p.x - edges.x, -(p.x - edges.z), east);
        let iy = select(p.y - edges.y, -(p.y - edges.w), south);
        let kind = (kinds >> (2u * corner)) & 3u;
        if kind == 1u {
            if ix < size && iy < size {
                distance = max(distance, span(ix - size, iy - size) - size);
            }
        } else if kind == 2u {
            distance = max(distance, (size - ix - iy) / SQRT_2);
        }
    }
    return distance;
}

@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let shape = shapes[input.record];
    if shape.flags.w == 1u {
        // The solid box: an outer coverage of 1, and all border or none.
        let band = select(0.0, 1.0, shape.flags.y == 1u);
        return shape.border * band + shape.fill * (1.0 - shape.border.a * band);
    }
    // The pixel center, relative to the span's origin.
    let p = input.position.xy - shape.origin.xy;
    if shape.flags.x == 0u {
        let outer = ramp(boundary(p, shape.a, shape.b, shape.flags.z));
        var inside = outer;
        if shape.flags.y == 1u {
            inside = 0.0;
        } else if shape.flags.y == 2u {
            inside = min(ramp(boundary(p, shape.c, shape.d, shape.flags.z)), outer);
        }
        let band = outer - inside;
        return shape.border * band + shape.fill * (outer - shape.border.a * band);
    }
    if shape.flags.x == 1u {
        let dx = p.x - shape.a.x;
        let dy = p.y - shape.a.y;
        let radius = shape.a.z;
        let half_width = shape.a.w;
        var distance: f32;
        if shape.flags.y == 1u {
            distance = abs(span(dx, dy) - radius) - half_width;
        } else {
            // Rotate so the sweep's middle lies on +x, and fold.
            let px = dx * shape.b.x + dy * shape.b.y;
            let py = abs(dy * shape.b.x - dx * shape.b.y);
            if py * shape.b.z <= px * shape.b.w {
                distance = abs(span(px, py) - radius) - half_width;
            } else {
                distance = span(px - radius * shape.b.z, py - radius * shape.b.w) - half_width;
            }
        }
        return shape.fill * (ramp(distance) * shape.origin.z);
    }
    let vertices = array<vec2<f32>, 3>(shape.a.xy, shape.a.zw, shape.b.xy);
    var nearest = 3.0e38;
    var inside = true;
    for (var index = 0u; index < 3u; index += 1u) {
        let start = vertices[index];
        let edge = vertices[(index + 1u) % 3u] - start;
        let offset = p - start;
        let along = clamp((offset.x * edge.x + offset.y * edge.y) / (edge.x * edge.x + edge.y * edge.y), 0.0, 1.0);
        nearest = min(nearest, span(offset.x - edge.x * along, offset.y - edge.y * along));
        inside = inside && shape.b.z * (edge.x * offset.y - edge.y * offset.x) >= 0.0;
    }
    return shape.fill * ramp(select(nearest, -nearest, inside));
}
