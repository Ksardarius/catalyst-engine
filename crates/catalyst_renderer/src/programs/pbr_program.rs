use flecs_ecs::prelude::*;
use wgpu::RenderPipeline;

use crate::{
    material::GpuMaterial,
    mesh::{GpuGeometry, MeshInstance, Vertex},
    programs::{GpuProgram, GpuProgramRenderContext},
    texture::TextureHelper,
};

pub struct PbrProgram {
    pipeline: RenderPipeline,
    pub material_layout: wgpu::BindGroupLayout,
    pub mesh_layout: wgpu::BindGroupLayout,
}

impl GpuProgram for PbrProgram {
    type InitData = wgpu::BindGroupLayout;
    type DrawData<'a> = (
        &'a wgpu::BindGroup,         // Global (Camera/Lights) - Group 0
        &'a Query<&'a MeshInstance>, // The Meshes - Group 1 & 2
    );

    fn new(ctx: &GpuProgramRenderContext, global_layout: &Self::InitData) -> Self {
        let shader = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let material_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Material Bind Group Layout"),
                    entries: &[
                        // --- BINDING 0: Material Settings (Uniform Buffer) ---
                        // The error happened because this was likely set to 'Texture' or missing!
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // --- BINDING 1: Diffuse Texture ---
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // --- BINDING 2: Sampler ---
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // --- METALLIC-ROUGHNESS MAP ---
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // SAMPLER
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        // --- NORMAL MAP ---
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // SAMPLER
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });

        let mesh_bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mesh Bind Group Layout"),
                    entries: &[
                        // --- BINDING 0: MVP Matrix ---
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        // 2. Create Pipeline Layout
        let render_pipeline_layout =
            ctx.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("PBR Pipeline Layout"),
                    // NOW WE HAVE TWO SETS: [0: Uniforms, 1: Textures]
                    bind_group_layouts: &[
                        global_layout,
                        &material_bind_group_layout,
                        &mesh_bind_group_layout,
                    ], // No uniforms yet
                    push_constant_ranges: &[],
                });

        // 3. Create the Pipeline
        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Vertex::desc()], // <--- Use our Vertex layout!
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureHelper::DEPTH_FORMAT,
                    depth_write_enabled: true, // Write Z-values
                    depth_compare: wgpu::CompareFunction::Less, // Closer pixels win
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to Fill means "draw filled triangles"
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        Self {
            pipeline,
            material_layout: material_bind_group_layout,
            mesh_layout: mesh_bind_group_layout,
        }
    }

    fn record<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, data: Self::DrawData<'a>) {
        let (global_bind_group, mesh_query) = data;

        // 1. Set Pipeline
        render_pass.set_pipeline(&self.pipeline);

        // 2. Bind Shared Data (Group 0)
        // This is the "Shared Buffer" passed in by reference
        render_pass.set_bind_group(0, global_bind_group, &[]);
        // 3. Draw Loop
        mesh_query.run(|mut iter| {
            let world = iter.world();
            let mut current_index_count: u32 = 0;

            while iter.next() {
                let instances = iter.field::<MeshInstance>(0);
                let material_entity = world.entity_from_id(iter.group_id());
                let mesh_pair = iter.pair(1);
                let mesh_entity = mesh_pair.second_id();

                material_entity.get::<&GpuMaterial>(|gpu_material| {
                    render_pass.set_bind_group(1, &gpu_material.bind_group, &[]);
                });

                mesh_entity.try_get::<&GpuGeometry>(|gpu_geometry| {
                    render_pass.set_vertex_buffer(0, gpu_geometry.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        gpu_geometry.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint32,
                    );

                    //last_mesh_uuid = Some(current_mesh_uuid);
                    current_index_count = gpu_geometry.index_count;
                });

                for i in iter.iter() {
                    render_pass.set_bind_group(2, &instances[i].bind_group, &[]);
                    render_pass.draw_indexed(0..current_index_count, 0, 0..1);
                }
            }
        });
    }
}
