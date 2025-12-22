// Vertex Output / Fragment Input
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

// The Uniform (Set 0, Binding 0)
// This receives the MVP matrix we calculated in Rust
@group(0) @binding(0)
var<uniform> transform: mat4x4<f32>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    // Apply the matrix multiplication!
    out.clip_position = transform * vec4<f32>(position, 1.0);
    
    // Simple lighting fake: use Normal as color
    out.color = normal * 0.5 + 0.5; 
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}