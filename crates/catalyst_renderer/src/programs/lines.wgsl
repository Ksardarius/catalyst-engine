struct VSIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VSOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// --- LIGHTING ---
struct PointLight {
    position: vec4<f32>, // .xyz = position, .w = intensity
    color: vec4<f32>,    // .xyz = color,    .w = radius (or unused)
};

struct LightUniforms {
    sun_direction: vec4<f32>, // .xyz = direction, .w = intensity
    sun_color: vec4<f32>,     // .xyz = color,     .w = padding
    lights: array<PointLight, 4>,
    camera_pos: vec3<f32>,
    active_lights: u32,       // How many point lights to loop over
};

struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> scene_data: LightUniforms;

@vertex
fn vs_main(input: VSIn) -> VSOut {
    var out: VSOut;
    out.clip_pos = camera.view_proj * vec4<f32>(input.position, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VSOut) -> @location(0) vec4<f32> {
    return input.color;
}
