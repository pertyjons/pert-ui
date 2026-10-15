struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tint: vec4<f32>,
    @location(1) uv: vec2<f32>,
};
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) tint: vec4<f32>, @location(2) uv: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.tint = tint;
    output.uv = uv;
    return output;
}
// The sRGB texture format decodes texels to linear light before filtering, and
// they are already premultiplied, so the straight-alpha tint is premultiplied too.
@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(image, image_sampler, input.uv);
    return texel * vec4<f32>(input.tint.rgb * input.tint.a, input.tint.a);
}
