struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};
@group(0) @binding(0) var coverage: texture_2d<f32>;
@group(0) @binding(1) var coverage_sampler: sampler;
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    output.uv = uv;
    return output;
}
@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(coverage, coverage_sampler, input.uv).r;
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
