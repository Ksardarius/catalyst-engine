use std::collections::HashMap;

use catalyst_core::{camera::Camera, transform::Transform, *};
use catalyst_window::MainWindow;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

use crate::{
    global_uniform::GlobalUniform,
    mesh::{Mesh, MeshData, Vertex},
};

pub mod global_uniform;
pub mod mesh;

#[derive(Resource)]
pub struct LayoutResource(pub wgpu::BindGroupLayout);

// The Resource that holds our GPU connection
#[derive(Resource)]
pub struct RenderContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffers: HashMap<u64, wgpu::Buffer>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // 1. Initialize the Asset Bank for Meshes
        // We use "init_resource" which calls Default::default()
        app.world.init_resource::<Assets<MeshData>>();

        // 1. Setup: Connect to GPU (Runs once at startup)
        app.add_startup_system(init_wgpu);

        // 2. Draw: Clear the screen (Runs every frame)
        app.add_system(render_frame);
    }
}

// --- SYSTEM 1: INITIALIZATION ---
fn init_wgpu(world: &mut World) {
    // 1. Get the Window from the World
    // We use "get_non_send_resource" because Window is main-thread only.
    let window = world
        .get_non_send_resource::<MainWindow>()
        .expect("Window not found! Did you add WindowPlugin?");

    // 2. Create the Instance (Vulkan/Metal/DX12)
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

    // 3. Create Surface (The canvas on the window)
    // UNSAFE: We must ensure the window outlives the surface. In our loop, it does.
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window.0).unwrap())
    }
    .unwrap();

    // 4. Request Adapter (Physical GPU)
    // We use 'pollster' to block on this async function inside a sync system
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("No GPU found!");

    // 5. Request Device (Logical GPU connection)
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();

    // 6. Configure the Surface
    let size = window.0.inner_size();
    let caps = surface.get_capabilities(&adapter);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: caps.formats[0], // Use the first supported format (usually sRGB)
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo, // VSync On
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // 2. Create Pipeline Layout
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout], // No uniforms yet
        push_constant_ranges: &[],
    });

    // 3. Create the Pipeline
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
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
        depth_stencil: None, // No depth buffer yet
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    println!(">>> Catalyst Renderer: Pipeline Compiled <<<");

    // 7. Store everything in the World
    world.insert_resource(RenderContext {
        device,
        queue,
        surface,
        config,
        pipeline,
        vertex_buffers: HashMap::new(),
    });

    world.insert_resource(LayoutResource(bind_group_layout));
}

// --- SYSTEM 2: RENDERING ---
fn render_frame(
    mut context: ResMut<RenderContext>,
    layout: Res<LayoutResource>,
    mesh_assets: Res<Assets<MeshData>>,
    // We query for the Camera separately
    camera_query: Query<(&Camera, &Transform), With<Camera>>,
    // We query for Objects
    mesh_query: Query<(&Mesh, &Transform), Without<Camera>>,
) {
    // 1. Get the next frame texture from the swapchain
    let frame = match context.surface.get_current_texture() {
        Ok(frame) => frame,
        Err(wgpu::SurfaceError::Outdated) => {
            // Reconfigure if resized (skip for now)
            return;
        }
        Err(_) => return,
    };

    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // 1. Calculate Camera Matrix (View * Projection)
    let (camera, cam_transform) = match camera_query.single() {
        Ok(c) => c,
        Err(_) => return, // No camera? Don't draw.
    };

    let projection = camera.compute_projection_matrix();
    // View Matrix is the INVERSE of the Camera's Transform
    let view_matrix = cam_transform.compute_matrix().inverse();
    let camera_matrix = projection * view_matrix;

    mesh_assets.with_storage(|storage| {
        for (mesh_handle_component, _) in &mesh_query {
            let id = mesh_handle_component.0.id;

            // If cache doesn't have it, AND asset storage has data -> UPLOAD
            if !context.vertex_buffers.contains_key(&id) {
                if let Some(mesh_data) = storage.get(&id) {
                    println!(">>> GPU: Uploading Mesh ID {} (One-time) <<<", id);

                    use wgpu::util::DeviceExt;
                    let buffer =
                        context
                            .device
                            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                label: Some("Cached Vertex Buffer"),
                                contents: bytemuck::cast_slice(&mesh_data.vertices),
                                usage: wgpu::BufferUsages::VERTEX,
                            });

                    context.vertex_buffers.insert(id, buffer);
                }
            }
        }
    });

    // 2. Create a Command Encoder
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    // 3. THE RENDER PASS (Clear Screen to Blue)
    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2, // Dark Blue/Slate
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Use our pipeline
        render_pass.set_pipeline(&context.pipeline);

        // ITERATE OVER ECS ENTITIES
        for (mesh_handle_component, mesh_transform) in &mesh_query {
            let id = mesh_handle_component.0.id;

            if let Some(gpu_buffer) = context.vertex_buffers.get(&id) {
                // 2. Calculate Final Matrix (MVP)
                // Model Matrix: Where the object is
                let model_matrix = mesh_transform.compute_matrix();
                // Final = Camera * Model
                let mvp_matrix = camera_matrix * model_matrix;

                // 3. Create Uniform Buffer (Direct Upload)
                // Note: In production, create this ONCE per entity, not every frame.
                let uniform_data = GlobalUniform {
                    transform_matrix: mvp_matrix,
                };

                // 1. Create a temporary buffer for this mesh (The "Direct" way)
                // In a pro engine, this buffer lives in a Component, not created every frame.
                use wgpu::util::DeviceExt;

                let uniform_buffer =
                    context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Uniform Buffer"),
                            contents: bytemuck::bytes_of(&uniform_data),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                // 4. Create Bind Group
                // This connects the buffer to the shader slot 0
                let bind_group = context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Transform Bind Group"),
                        layout: &layout.0,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        }],
                    });

                // 5. Draw
                render_pass.set_bind_group(0, &bind_group, &[]);

                render_pass.set_vertex_buffer(0, gpu_buffer.slice(..));

                // We need to know the count. 
                // Since we don't have the MeshData here easily without locking again, 
                // we should probably store vertex_count in the Cache or MeshData handle.
                // Hack for now: Ask the buffer size (size / stride)
                let count = gpu_buffer.size() as u32 / std::mem::size_of::<Vertex>() as u32;
                render_pass.draw(0..count, 0..1);
            }
        }
    } // Pass ends here (release lock)

    // 4. Submit to GPU
    context.queue.submit(std::iter::once(encoder.finish()));
    frame.present();
}
