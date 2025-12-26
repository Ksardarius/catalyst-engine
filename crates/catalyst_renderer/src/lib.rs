use std::collections::HashMap;

use catalyst_assets::{
    AssetEvent, MaterialDefinition, MeshDefinition,
    assets::{Assets, Handle, MeshData},
    material::{MaterialData, TextureData, TextureFormat},
};
use catalyst_core::{camera::Camera, transform::Transform, *};
use catalyst_window::MainWindow;
use glam::{Mat4, Vec3};
use uuid::Uuid;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, util::DeviceExt};

use crate::{
    material::{GpuMaterialUniform, Material},
    mesh::{Mesh, Vertex},
    texture::GpuTexture,
};

pub mod global_uniform;
mod material;
pub mod mesh;
mod texture;

use texture::TextureHelper;

#[derive(Resource)]
pub struct LayoutResource(pub wgpu::BindGroupLayout);

#[derive(Resource)]
pub struct MaterialLayout(pub wgpu::BindGroupLayout);

// The Resource that holds our GPU connection
#[derive(Resource)]
pub struct RenderContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub depth_texture: wgpu::TextureView,

    pub vertex_buffers: HashMap<Uuid, wgpu::Buffer>,
    pub index_buffers: HashMap<Uuid, wgpu::Buffer>,
    pub index_counts: HashMap<Uuid, u32>,

    // material
    pub texture_cache: HashMap<Uuid, GpuTexture>,
    pub material_cache: HashMap<Uuid, wgpu::BindGroup>,

    pub default_diffuse: GpuTexture,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        // 1. Setup: Connect to GPU (Runs once at startup)
        app.add_startup_system(init_wgpu);

        app.add_system(prepare_gpu_assets);
        app.add_system(realize_render_components);

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

    let depth_texture = TextureHelper::create_depth_texture(&device, &config, "Depth Texture");

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

    let material_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            ],
        });

    // 2. Create Pipeline Layout
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        // NOW WE HAVE TWO SETS: [0: Uniforms, 1: Textures]
        bind_group_layouts: &[&bind_group_layout, &material_bind_group_layout], // No uniforms yet
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
        depth_stencil: Some(wgpu::DepthStencilState {
            format: TextureHelper::DEPTH_FORMAT,
            depth_write_enabled: true,                  // Write Z-values
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

    // --- CREATE DEFAULT TEXTURE (1x1 White Pixel) ---
    // We create this manually so we don't depend on an asset file existing
    let white_pixel = GpuTexture::from_image(
        &device,
        &queue,
        &TextureData {
            width: 1,
            height: 1,
            // RGBA: (255, 255, 255, 255) -> Solid White
            pixels: vec![255, 255, 255, 255],
            format: TextureFormat::Rgba8Unorm,
        },
        Some("Default White Texture"),
    );

    println!(">>> Catalyst Renderer: Pipeline Compiled <<<");

    // 7. Store everything in the World
    world.insert_resource(RenderContext {
        device,
        queue,
        surface,
        config,
        pipeline,
        depth_texture,
        vertex_buffers: HashMap::new(),
        index_buffers: HashMap::new(),
        index_counts: HashMap::new(),
        texture_cache: HashMap::new(),
        material_cache: HashMap::new(),

        default_diffuse: white_pixel,
    });

    world.insert_resource(LayoutResource(bind_group_layout));
    world.insert_resource(MaterialLayout(material_bind_group_layout));
}

// --- SYSTEM 2: RENDERING ---
fn render_frame(
    mut context: ResMut<RenderContext>,
    layout_res: Res<LayoutResource>,
    // We query for the Camera separately
    camera_q: Query<(&Camera, &Transform)>,
    // We query for Objects
    mesh_q: Query<(&Mesh, &Material, &Transform), Without<Camera>>,
) {
    // 1. Get the next frame texture from the swapchain
    let output = match context.surface.get_current_texture() {
        Ok(texture) => texture,
        Err(wgpu::SurfaceError::Lost) => {
            // Surface lost (e.g., window minimized), reconfigure next frame
            context.surface.configure(&context.device, &context.config);
            return;
        }
        Err(e) => {
            eprintln!("Render error: {:?}", e);
            return;
        }
    };

    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    // 1. Calculate Camera Matrix (View * Projection)
    let (view_proj, camera_pos) = if let Ok((_cam, cam_t)) = camera_q.single() {
        // A: View Matrix (Inverse of Camera Transform)
        // Move the world opposite to the camera
        let view = Mat4::look_at_rh(
            cam_t.translation,                               // Eye
            cam_t.translation + (cam_t.rotation * -Vec3::Z), // Target (Forward is -Z)
            Vec3::Y,                                         // Up
        );

        // B: Projection Matrix (Perspective)
        let aspect = context.config.width as f32 / context.config.height as f32;
        let proj = Mat4::perspective_rh(
            45.0f32.to_radians(), // FOV
            aspect,
            0.1,   // Near Plane
            100.0, // Far Plane
        );

        (proj * view, cam_t.translation)
    } else {
        (Mat4::IDENTITY, Vec3::ZERO)
    };

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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &context.depth_texture, // The texture we created
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0), // Clear to "Far" (1.0)
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        // Use our pipeline
        render_pass.set_pipeline(&context.pipeline);

        // ITERATE OVER ECS ENTITIES
        for (mesh, material, transform) in &mesh_q {
            let mesh_id = mesh.0.id;

            if let (Some(v_buf), Some(i_buf), Some(index_count), Some(mat_bind_group)) = (
                context.vertex_buffers.get(&mesh_id),
                context.index_buffers.get(&mesh_id),
                context.index_counts.get(&mesh_id),
                context.material_cache.get(&material.0.id),
            ) {
                // A. Calculate MVP Matrix (Model-View-Projection)
                // Model: Local -> World
                let model_matrix =
                    Mat4::from_rotation_translation(transform.rotation, transform.translation);

                // Final Matrix: Local -> Clip Space
                let mvp_matrix = view_proj * model_matrix;

                // B. Create Uniform Buffer for THIS object
                // (Note: For 1000s of objects, use DynamicUniformBuffer. For now, this is fine.)
                let mvp_ref = mvp_matrix.to_cols_array_2d();
                let uniform_buffer =
                    context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Uniform Buffer"),
                            contents: bytemuck::cast_slice(&[mvp_ref]), // cast [[f32;4];4] to bytes
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                // C. Create Bind Group
                let bind_group = context
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &layout_res.0,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform_buffer.as_entire_binding(),
                        }],
                        label: Some("Object Bind Group"),
                    });

                // D. Draw!
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.set_bind_group(1, mat_bind_group, &[]);

                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..*index_count, 0, 0..1);
            }
        }
    } // Pass ends here (release lock)

    // 4. Submit to GPU
    context.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}

fn prepare_gpu_assets(
    mat_layout: Res<MaterialLayout>,
    mut context: ResMut<RenderContext>,
    mut events: MessageReader<AssetEvent>,
    assets: Res<Assets<MeshData>>, // Read CPU data
    assets_tex: Res<Assets<TextureData>>,
    assets_mat: Res<Assets<MaterialData>>,
) {
    for event in events.read() {
        match event {
            AssetEvent::MeshLoaded { id, .. } => {
                let handle = Handle::<MeshData>::from_id(*id);

                if let Some(mesh_data) = assets.get(&handle) {
                    println!(">>> GPU: Uploading Mesh {:?}", id);

                    let (v_buf, i_buf, count) = create_gpu_buffer(&context.device, &mesh_data);

                    // 3. Store in the Context Cache
                    context.vertex_buffers.insert(*id, v_buf);
                    context.index_buffers.insert(*id, i_buf);
                    context.index_counts.insert(*id, count);
                }
            }
            AssetEvent::TextureLoaded { id, .. } => {
                let handle = Handle::<TextureData>::from_id(*id);
                if let Some(texture_data) = assets_tex.get(&handle) {
                    println!(">>> GPU: Uploading Texture {:?}", id);

                    let gpu_tex = GpuTexture::from_image(
                        &context.device,
                        &context.queue,
                        &texture_data,
                        None,
                    );

                    context.texture_cache.insert(*id, gpu_tex);

                    // for (mat_id, mat_data) in assets_mat.iter() {
                    //     let uses_this_texture = mat_data.diffuse_texture
                    //         .as_ref()
                    //         .map_or(false, |h| h.id == *id);

                    //     if uses_this_texture {
                    //         println!("    -> Updating Material {:?} with new texture", mat_id);
                    //         // Call our helper to rebuild this specific material
                    //         create_material_bind_group(
                    //             &mut context,
                    //             &mat_layout,
                    //             *mat_id,
                    //             mat_data
                    //         );
                    //     }
                    // }
                }
            }
            // ----------------------------------------------------
            // CASE B: A Material is ready to be built
            // ----------------------------------------------------
            AssetEvent::MaterialLoaded { id } => {
                let handle = Handle::<MaterialData>::from_id(*id);

                if let Some(mat_data) = assets_mat.get(&handle) {
                    println!(">>> GPU: Building Material {:?}", id);

                    create_material_bind_group(&mut context, &mat_layout, *id, &mat_data);
                }
            }
        }
    }
}

fn create_gpu_buffer(device: &wgpu::Device, data: &MeshData) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    // 1. Interleave Data (SoA -> AoS)
    // We combine pos, normal, uv into a single 'Vertex' struct list
    let vertex_count = data.vertices.len();
    let mut vertices = Vec::with_capacity(vertex_count);

    for i in 0..vertex_count {
        vertices.push(Vertex {
            position: data.vertices[i].position,
            normal: data.vertices[i].normal,
            uv: data.vertices[i].uv,
        });
    }

    use wgpu::util::DeviceExt;

    let v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let i_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh Index Buffer"),
        contents: bytemuck::cast_slice(&data.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    (v_buffer, i_buffer, data.indices.len() as u32)
}

fn create_material_bind_group(
    context: &mut RenderContext,
    layout: &MaterialLayout,
    mat_id: Uuid,
    mat_data: &MaterialData,
) {
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
    let (view, sampler) = if let Some(tex_handle) = &mat_data.diffuse_texture {
        if let Some(gpu_tex) = context.texture_cache.get(&tex_handle.id) {
            // Success: Real Texture
            (&gpu_tex.view, &gpu_tex.sampler)
        } else {
            // Fallback: Texture loading... use White Pixel for now
            (
                &context.default_diffuse.view,
                &context.default_diffuse.sampler,
            )
        }
    } else {
        // Fallback: No texture assigned
        (
            &context.default_diffuse.view,
            &context.default_diffuse.sampler,
        )
    };

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
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

    // D. Store/Overwrite Cache
    context.material_cache.insert(mat_id, bind_group);
}

pub fn realize_render_components(
    mut commands: Commands,
    // Query 1: Find entities asking for a Mesh
    mesh_requests: Query<(Entity, &MeshDefinition), Added<MeshDefinition>>,
    // Query 2: Find entities asking for a Material
    mat_requests: Query<(Entity, &MaterialDefinition), Added<MaterialDefinition>>,
) {
    // 1. Inflate Meshes
    for (entity, definition) in mesh_requests.iter() {
        // We take the handle from the definition and create the internal Render Component
        commands.entity(entity).insert(Mesh(definition.0.clone()));
    }

    // 2. Inflate Materials
    for (entity, definition) in mat_requests.iter() {
        commands
            .entity(entity)
            .insert(Material(definition.0.clone()));
    }
}
