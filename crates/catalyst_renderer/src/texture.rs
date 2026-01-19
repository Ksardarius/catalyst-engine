use catalyst_assets::material::TextureData;
use flecs_ecs::prelude::*;
use wgpu::{
    Device, Extent3d, SurfaceConfiguration, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};

use crate::render::RenderContext;

#[derive(Component, Clone)]
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl GpuTexture {
    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &TextureData,
        label: Option<&str>,
    ) -> Self {
        let wgpu_format = match data.format {
            catalyst_assets::material::TextureFormat::Rgba8UnormSrgb => {
                wgpu::TextureFormat::Rgba8UnormSrgb
            }
            catalyst_assets::material::TextureFormat::Rgba8Unorm => {
                wgpu::TextureFormat::Rgba8Unorm
            }
            catalyst_assets::material::TextureFormat::Rgba32Float => {
                wgpu::TextureFormat::Rgba32Float
            }
            catalyst_assets::material::TextureFormat::Gray8 => wgpu::TextureFormat::R8Unorm,
        };

        let size = wgpu::Extent3d {
            width: data.width,
            height: data.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format, // Use the translated format
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // 2. Upload Pixels
        queue.write_texture(
            // RENAMED: ImageCopyTexture -> TexelCopyTextureInfo
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data.pixels,
            // RENAMED: ImageDataLayout -> TexelCopyBufferLayout
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * data.width),
                rows_per_image: Some(data.height),
            },
            size,
        );

        // 3. Create View (How the shader sees it)
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 4. Create Sampler (How to filter pixels - Linear/Nearest)
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat, // Wrap texture
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear, // Smooth close up
            min_filter: wgpu::FilterMode::Linear, // Smooth far away
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

pub struct TextureHelper;

impl TextureHelper {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float; // Standard depth format

    pub fn create_depth_texture(
        device: &Device,
        config: &SurfaceConfiguration,
        label: &str,
    ) -> wgpu::TextureView {
        let size = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }
}

pub fn register_texture_handlers(world: &World) {
    world
        .system_named::<(&TextureData, &mut RenderContext)>("Init Texture GPU buffers")
        .without(GpuTexture::id())
        .kind(flecs::pipeline::OnStore)
        .each_entity(|entity, (texture_data, context)| {
            let gpu_tex =
                GpuTexture::from_image(&context.device, &context.queue, &texture_data, None);

            entity.set(gpu_tex);
        });
}
