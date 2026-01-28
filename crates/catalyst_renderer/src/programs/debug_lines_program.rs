use catalyst_core::App;
use flecs_ecs::prelude::*;
use wgpu::{Device, Queue, RenderPipeline, VertexFormat, util::DeviceExt};

use crate::{RenderContext, programs::GpuProgram, render::DebugDraw3D, texture::TextureHelper};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DebugLineVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl DebugLineVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<DebugLineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct DebugLinesProgram {
    pipeline: RenderPipeline,
    buffer: Option<wgpu::Buffer>,
    capacity: usize,
    draw_count: u32,
}

impl DebugLinesProgram {
    pub fn prepare(&mut self, vertexes: &Vec<DebugLineVertex>, device: &Device, queue: &Queue) {
        if vertexes.is_empty() {
            self.draw_count = 0;
            return;
        }

        match self.buffer {
            None => {
                let initial_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Debug Lines Buffer"),
                    contents: bytemuck::cast_slice(vertexes),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

                self.buffer = Some(initial_buffer);
                self.capacity = vertexes.len();
                self.draw_count = vertexes.len() as u32;
            }
            Some(ref buffer) => {
                self.draw_count = vertexes.len() as u32;

                if vertexes.len() > self.capacity {
                    self.capacity = vertexes.len().max(self.capacity * 2);
                    self.buffer = Some(device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Debug Lines Buffer (Resized)"),
                            contents: bytemuck::cast_slice(vertexes),
                            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        },
                    ));
                } else {
                    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(vertexes));
                }
            }
        };
    }
}

impl GpuProgram for DebugLinesProgram {
    type InitData = wgpu::BindGroupLayout;

    type DrawData<'a> = &'a wgpu::BindGroup;

    fn new(ctx: &super::GpuProgramRenderContext, global_layout: &Self::InitData) -> Self {
        let line_shader = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("lines.wgsl"));

        let line_pipeline_layout =
            ctx.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Debug Line Pipeline Layout"),
                    bind_group_layouts: &[global_layout], // same camera bind group
                    push_constant_ranges: &[],
                });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                cache: None,
                label: Some("Debug Line Pipeline"),
                layout: Some(&line_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &line_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[DebugLineVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &line_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: ctx.format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: TextureHelper::DEPTH_FORMAT,
                    depth_write_enabled: false, // important: do NOT write depth
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });

        Self {
            pipeline,
            buffer: None,
            capacity: 0,
            draw_count: 0,
        }
    }

    fn record<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        global_bind_group: Self::DrawData<'a>,
    ) {
        if self.draw_count == 0 {
            return;
        }

        // 1. Set Pipeline
        render_pass.set_pipeline(&self.pipeline);

        // 2. Bind Shared Data (Group 0)
        // This is the "Shared Buffer" passed in by reference
        render_pass.set_bind_group(0, global_bind_group, &[]);

        if let Some(ref buffer) = self.buffer {
            render_pass.set_vertex_buffer(0, buffer.slice(..));

            render_pass.draw(0..self.draw_count, 0..1);
        }
    }
}

pub fn register_debug_lines_program_systems(app: &mut App) {
    app.world
        .system_named::<(&mut DebugDraw3D, &mut RenderContext)>("process_debug_lines_rendering")
        .kind(flecs::pipeline::PreStore)
        .run(|mut iter| {
            while iter.next() {
                let mut debug_field = iter.field_mut::<DebugDraw3D>(0);
                let mut context_field = iter.field_mut::<RenderContext>(1);
                if let (Some(debug), Some(context)) =
                    (debug_field.get_mut(0), context_field.get_mut(0))
                {
                    context.debug_lines_program.prepare(
                        &mut debug.debug_line_vertices,
                        &context.device,
                        &context.queue,
                    );

                    debug.debug_line_vertices.clear();
                }
            }
        });
}
