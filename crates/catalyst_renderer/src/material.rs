use catalyst_assets::{
    MaterialDefinition,
    assets::Handle,
    material::{MaterialData, MaterialSettings},
};
use flecs_ecs::prelude::*;
use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::{
    render::{MaterialLayout, RenderContext},
    texture::GpuTexture,
};

#[derive(Component, Clone, Debug)] // <--- Component is essential for the Query
pub struct AssetMaterial;

#[derive(Component)]
pub struct GpuMaterial {
    pub bind_group: wgpu::BindGroup,
}

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

pub fn register_material_handlers(world: &World) {
    world
        .system_named::<&MaterialDefinition>("Link Material Definition to AssetMaterial")
        .without((AssetMaterial, flecs::Wildcard))
        .each_entity(|entity, definition| {
            let world = entity.world();
            if let Some(material_entity) = definition.0.try_get_entity(&world) {
                entity.add((AssetMaterial, material_entity));
            }
        });

    world
        .system_named::<(&MaterialData, &mut RenderContext, &MaterialLayout)>(
            "Init Material GPU buffers",
        )
        .without(GpuMaterial::id())
        .kind(flecs::pipeline::OnStore)
        .each_entity(|entity, (mat_data, context, mat_layout)| {
            let world = entity.world();
            let bind_group = create_material_bind_group(context, mat_layout, mat_data, &world);
            entity.set(GpuMaterial { bind_group });
        });
}

fn create_material_bind_group(
    context: &mut RenderContext,
    layout: &MaterialLayout,
    mat_data: &MaterialData,
    world: &World,
) -> wgpu::BindGroup {
    // A. Create Uniform Buffer (Settings)
    let gpu_uniform = GpuMaterialUniform::from(mat_data.settings.clone());
    let uniform_buffer = context
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Uniforms"),
            contents: bytemuck::cast_slice(&[gpu_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // B. Find Texture (or Fallback)
    let mut binding = mat_data
        .diffuse_texture
        .as_ref()
        .and_then(|tex_handle| tex_handle.try_get_entity(world))
        .and_then(|texture_entity| texture_entity.try_get::<&GpuTexture>(|tx| tx.clone()));
    let diffuse_texture = binding.get_or_insert(context.default_diffuse.clone());

    let mut binding = mat_data
        .metallic_roughness_texture
        .as_ref()
        .and_then(|tex_handle| tex_handle.try_get_entity(world))
        .and_then(|texture_entity| texture_entity.try_get::<&GpuTexture>(|tx| tx.clone()));
    let roughness_texture = binding.get_or_insert(context.default_diffuse.clone());

    let mut binding = mat_data
        .normal_texture
        .as_ref()
        .and_then(|tex_handle| tex_handle.try_get_entity(world))
        .and_then(|texture_entity| texture_entity.try_get::<&GpuTexture>(|tx| tx.clone()));
    let normal_texture = binding.get_or_insert(context.default_diffuse.clone());

    // C. Create Bind Group
    let bind_group = context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material Bind Group"),
            layout: &layout.0,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&roughness_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&roughness_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&normal_texture.sampler),
                },
            ],
        });

    bind_group
}
