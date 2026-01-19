use flecs_ecs::prelude::*;
use crate::assets::Handle;

#[derive(Clone, Copy, Debug)]
pub enum TextureFormat {
    Rgba8Unorm, // Standard 32-bit color (0-255)
    Rgba8UnormSrgb, // Standard 32-bit color (0-255)
    Rgba32Float, // HDR data
    Gray8,      // Grayscale (used for Roughness/Metallic masks)
}

#[derive(Component, Clone, Debug)]
pub struct TextureData {
    pub name: String,
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat, // e.g., Rgba8Unorm
}

#[derive(Clone, Debug)]
pub struct MaterialSettings {
    pub base_color: [f32; 4],
    pub roughness: f32,
    pub metallic: f32,
}

impl Default for MaterialSettings {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            roughness: 0.5,
            metallic: 0.0,
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct MaterialData {
    pub settings: MaterialSettings,
    pub diffuse_texture: Option<Handle<TextureData>>,
    pub normal_texture: Option<Handle<TextureData>>,
    pub metallic_roughness_texture: Option<Handle<TextureData>>,
    pub occlusion_texture: Option<Handle<TextureData>>,
}

impl Default for MaterialData {
    fn default() -> Self {
        Self {
            settings: MaterialSettings::default(),
            diffuse_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
        }
    }
}