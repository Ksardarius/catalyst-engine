#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    pub sun_direction: [f32; 4], // .w = intensity
    pub sun_color: [f32; 4],     // .w = padding
    pub point_lights: [GpuPointLight; 4], // Fixed array of 4
    pub camera_pos: [f32; 3],
    pub active_lights: u32,      // Count
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPointLight {
    pub position: [f32; 4], // .w = intensity
    pub color: [f32; 4],    // .w = radius (unused in shader currently but good for padding)
}