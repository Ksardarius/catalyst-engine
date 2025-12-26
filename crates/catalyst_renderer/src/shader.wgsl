struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
};

// Must match GpuMaterialUniform in Rust (std140 layout)
struct MaterialUniform {
    color: vec4<f32>,   // 16 bytes
    roughness: f32,     // 4 bytes
    metallic: f32,      // 4 bytes
    // Padding (8 bytes) is implicit here to reach 16-byte alignment
};

// --- BINDINGS ----------------------------------------------------------

// Group 0: Per-Object Data (Changed every Draw Call)
@group(0) @binding(0)
var<uniform> transform: mat4x4<f32>;

// Group 1: Material Data (Shared across objects)
@group(1) @binding(0)
var<uniform> material: MaterialUniform;

@group(1) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(1) @binding(2)
var s_diffuse: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // 1. Calculate Screen Position
    out.clip_position = transform * vec4<f32>(in.position, 1.0);
    
    // 2. Pass Data to Fragment Shader
    out.uv = in.uv;
    
    // Pass Normal (Ideally, you multiply this by a NormalMatrix to handle rotation)
    // For now, passing it raw is okay for simple rotation.
    out.world_normal = in.normal;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Sample Texture
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv);

    // 2. Mix with Base Color (Multiply)
    // This allows you to tint the texture red/blue via code
    let final_albedo = tex_color * material.color;

    // 3. Simple Directional Lighting (The Sun)
    let light_dir = normalize(vec3<f32>(1.0, 2.0, 3.0)); // Light coming from top-right
    let normal = normalize(in.world_normal);

    // Dot Product: How aligned is the face to the sun?
    // max(0.0) prevents negative light (darkness)
    let diffuse_strength = max(dot(normal, light_dir), 0.0);

    // Ambient Light (So shadows aren't pitch black)
    let ambient_strength = 0.1;

    let lighting = diffuse_strength + ambient_strength;

    // 4. Final Color
    let result = final_albedo.xyz * lighting;
    
    // Re-attach Alpha
    return vec4<f32>(result, final_albedo.a);
}