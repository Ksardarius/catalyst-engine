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
    camera::CameraUniform,
    light::{GpuPointLight, LightUniforms},
    material::{GpuMaterialUniform, Material},
    mesh::{GpuMesh, Mesh, Vertex, prepare_mesh_transforms},
    texture::GpuTexture,
};

mod camera;
mod light;
mod material;
pub mod mesh;
mod texture;

use texture::TextureHelper;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum RenderSet {
    Start,  // Acquire swapchain image
    Scene,  // Draw the main 3D world
    Overlay, // Draw UI / Debug (Plugins insert here)
    End,    // Present to screen
}

// #[derive(Resource)]
// pub struct LayoutResource(pub wgpu::BindGroupLayout);

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

    pub camera_buffer: wgpu::Buffer,     // Created ONCE
    pub scene_data_buffer: wgpu::Buffer, // Created ONCE (for lights)

    // 2. Persistent Bind Group
    pub global_bind_group: wgpu::BindGroup, // References the buffers above
    pub mesh_bind_group_layout: wgpu::BindGroupLayout, // References the buffers above
}

#[derive(Resource, Default)]
pub struct RenderTarget {
    pub view: Option<wgpu::TextureView>,
    pub texture: Option<wgpu::SurfaceTexture>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            // Assuming you are using your custom 'Render' schedule/stage
            Stage::Render, 
            (
                RenderSet::Start,
                RenderSet::Scene,
                RenderSet::Overlay,
                RenderSet::End,
            ).chain()
        );

        app.world.init_resource::<RenderTarget>();

        // 1. Setup: Connect to GPU (Runs once at startup)
        app.add_startup_system(init_wgpu);

        app.add_system(prepare_gpu_assets);
        app.add_system(realize_render_components);

        // 2. Draw: Clear the screen (Runs every frame)
        app.add_system_to_stage(Stage::Render, start_frame.in_set(RenderSet::Start));
        app.add_system_to_stage(Stage::Render, render_frame.in_set(RenderSet::Scene));
        app.add_system_to_stage(Stage::Render, end_frame.in_set(RenderSet::End));

        app.add_system_to_stage(Stage::PostUpdate, &prepare_mesh_transforms);
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
            // --- BINDING 1: Light Uniforms ---
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
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
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        // NOW WE HAVE TWO SETS: [0: Uniforms, 1: Textures]
        bind_group_layouts: &[&bind_group_layout, &material_bind_group_layout, &mesh_bind_group_layout], // No uniforms yet
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

    let initial_camera_data = CameraUniform {
        view_proj: [[0.0; 4]; 4], // Placeholder
    };

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[initial_camera_data]),
        // USAGE: COPY_DST allows us to write to it later!
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let initial_light_data = LightUniforms {
        sun_direction: [0.0, -1.0, 0.0, 1.0],
        sun_color: [1.0, 1.0, 1.0, 0.0],
        point_lights: [GpuPointLight {
            position: [0.0; 4],
            color: [0.0; 4],
        }; 4],
        camera_pos: [0.0, 0.0, 0.0],
        active_lights: 0,
    };

    let scene_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Scene Data Buffer (Lights)"),
        contents: bytemuck::cast_slice(&[initial_light_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, // COPY_DST is critical for updates!
    });

    let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Global Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: scene_data_buffer.as_entire_binding(),
            },
        ],
    });

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

        camera_buffer,
        scene_data_buffer,
        global_bind_group,
        mesh_bind_group_layout
    });

    // world.insert_resource(LayoutResource(bind_group_layout));
    world.insert_resource(MaterialLayout(material_bind_group_layout));
}

pub fn start_frame(
    context: Res<RenderContext>,
    mut target: ResMut<RenderTarget>,
) {
    // Acquire the texture ONCE at the start of the frame
    if let Ok(frame) = context.surface.get_current_texture() {
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Store it so other plugins can see it
        target.texture = Some(frame);
        target.view = Some(view);
    }
}

// --- SYSTEM 2: RENDERING ---
fn render_frame(
    mut context: ResMut<RenderContext>,
    mut target: ResMut<RenderTarget>,
    // layout_res: Res<LayoutResource>,
    // We query for the Camera separately
    camera_q: Query<(&Camera, &Transform)>,
    // We query for Objects
    mesh_q: Query<(&Mesh, &Material, &GpuMesh), Without<Camera>>,
) {
    // 1. Calculate Camera Matrix (View * Projection)
    let view_proj = if let Ok((_cam, cam_t)) = camera_q.single() {
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

        proj * view
    } else {
        Mat4::IDENTITY
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
                view: &target.view.as_ref().unwrap(),
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

        context.queue.write_buffer(
            &context.camera_buffer,                                // Target
            0,                                                     // Offset
            bytemuck::cast_slice(&[view_proj.to_cols_array_2d()]), // Data
        );

        // ITERATE OVER ECS ENTITIES
        for (mesh, material, gpu_mesh) in &mesh_q {
            let mesh_id = mesh.0.id;

            if let (Some(v_buf), Some(i_buf), Some(index_count), Some(mat_bind_group)) = (
                context.vertex_buffers.get(&mesh_id),
                context.index_buffers.get(&mesh_id),
                context.index_counts.get(&mesh_id),
                context.material_cache.get(&material.0.id),
            ) {
                // D. Draw!
                render_pass.set_bind_group(0, &context.global_bind_group, &[]);
                render_pass.set_bind_group(1, mat_bind_group, &[]);
                render_pass.set_bind_group(2, &gpu_mesh.bind_group, &[]);

                render_pass.set_vertex_buffer(0, v_buf.slice(..));
                render_pass.set_index_buffer(i_buf.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..*index_count, 0, 0..1);
            }
        }
    } // Pass ends here (release lock)

    // 4. Submit to GPU
    context.queue.submit(std::iter::once(encoder.finish()));
}

pub fn end_frame(mut target: ResMut<RenderTarget>) {
    // Take the frame out and present it
    if let Some(frame) = target.texture.take() {
        frame.present();
    }
    target.view = None;
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
    let (diffuse_view, diffuse_sampler) = if let Some(tex_handle) = &mat_data.diffuse_texture {
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

    let (roughness_view, roughness_sampler) = if let Some(tex_handle) = &mat_data.metallic_roughness_texture {
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

    let (normal_view, normal_sampler) = if let Some(tex_handle) = &mat_data.normal_texture {
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
                    resource: wgpu::BindingResource::TextureView(diffuse_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(roughness_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(roughness_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(normal_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(normal_sampler),
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

// pub fn prepare_lights(
//     // We need the RenderContext to access the Buffer and Queue
//     context: &mut RenderContext,

//     // Queries to find lights in the scene
//     sun_query: Query<(&GlobalTransform, &DirectionalLight)>,
//     point_query: Query<(&GlobalTransform, &PointLight)>,

//     // We need Camera pos for specular calculations
//     cam_query: Query<&GlobalTransform, With<Camera>>,
// ) {
//     let mut uniforms = LightUniforms {
//         sun_direction: [0.0, -1.0, 0.0, 1.0], // Default Down
//         sun_color: [1.0, 1.0, 1.0, 0.0],
//         point_lights: [GpuPointLight {
//             position: [0.0; 4],
//             color: [0.0; 4],
//         }; 4],
//         camera_pos: [0.0, 0.0, 0.0],
//         active_lights: 0,
//     };

//     // A. Process Sun (Take the first one we find)
//     if let Some((transform, sun)) = sun_query.iter().next() {
//         // Calculate forward vector from rotation
//         let forward = transform.forward(); // Bevy GlobalTransform helper
//         uniforms.sun_direction = [forward.x, forward.y, forward.z, sun.intensity];
//         uniforms.sun_color = [sun.color[0], sun.color[1], sun.color[2], 0.0];
//     }

//     // B. Process Point Lights
//     let mut count = 0;
//     for (transform, light) in point_query.iter().take(4) {
//         // Limit to 4!
//         let pos = transform.translation();
//         uniforms.point_lights[count] = GpuPointLight {
//             position: [pos.x, pos.y, pos.z, light.intensity],
//             color: [light.color[0], light.color[1], light.color[2], light.radius],
//         };
//         count += 1;
//     }
//     uniforms.active_lights = count as u32;

//     // C. Process Camera Position
//     if let Some(cam_tf) = cam_query.iter().next() {
//         let pos = cam_tf.translation();
//         uniforms.camera_pos = [pos.x, pos.y, pos.z];
//     }

//     // D. Upload to GPU
//     // "scene_data_buffer" is the buffer you created in Binding 1 of Group 0
//     context.queue.write_buffer(
//         &context.scene_data_buffer,
//         0,
//         bytemuck::cast_slice(&[uniforms]),
//     );
// }
