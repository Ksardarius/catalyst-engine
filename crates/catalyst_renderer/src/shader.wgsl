// 1. Define the Uniform Block
// We receive a 4x4 matrix at Group 0, Binding 0
struct Globals {
    transform_matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> global: Globals;

// Vertex Shader
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // 2. Apply the Matrix!
    // Matrix Multiplication: Transform * Position = Screen Position
    out.clip_position = global.transform_matrix * vec4<f32>(model.position, 1.0);
    out.color = model.color;
    return out;
}

// Fragment Shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}