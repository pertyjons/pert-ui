// A stroke's covered tiles, composited from its region of a coverage page.
// The vertices are the scene's ordinary ones; `offset` carries, the same in
// every vertex of a quad, the step from a frame pixel to its texel in the
// page.
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) @interpolate(flat) offset: vec2<f32>,
};
@group(0) @binding(0) var page: texture_2d<f32>;
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>, @location(2) offset: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    output.offset = offset;
    return output;
}
@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(input.position.xy) + vec2<i32>(input.offset);
    let coverage = textureLoad(page, texel, 0).r;
    return vec4<f32>(input.color.rgb, input.color.a * coverage);
}
