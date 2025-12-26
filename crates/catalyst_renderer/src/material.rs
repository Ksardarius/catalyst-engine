use catalyst_assets::{assets::Handle, material::{MaterialData, MaterialSettings}};
use catalyst_core::Component;

#[derive(Component, Clone, Debug)] // <--- Component is essential for the Query
pub struct Material(pub Handle<MaterialData>);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuMaterialUniform {
    pub base_color: [f32; 4], // 16 bytes
    pub roughness: f32,       // 4 bytes
    pub metallic: f32,        // 4 bytes
    pub _padding: [f32; 2],   // 8 bytes (Total: 32 bytes, aligned to 16)
}

impl From<MaterialSettings> for GpuMaterialUniform {
    fn from(s: MaterialSettings) -> Self {
        Self {
            base_color: s.base_color,
            roughness: s.roughness,
            metallic: s.metallic,
            _padding: [0.0; 2],
        }
    }
}