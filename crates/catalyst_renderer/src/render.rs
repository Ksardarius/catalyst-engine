
use catalyst_assets::{
    material::{TextureData, TextureFormat},
};
use catalyst_core::{
    camera::Camera,
    transform::{GlobalTransform},
};
use catalyst_window::MainWindow;
use flecs_ecs::prelude::*;
use glam::{Mat4, Vec3};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, util::DeviceExt};

use crate::{
    camera::CameraUniform,
    light::{GpuPointLight, LightUniforms},
    material::{AssetMaterial, GpuMaterial},
    mesh::{AssetMesh, GpuGeometry, MeshInstance, Vertex},
    texture::{GpuTexture, TextureHelper},
};

#[derive(Component)]
pub struct RenderContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
    pub depth_texture: wgpu::TextureView,

    pub default_diffuse: GpuTexture,

    pub camera_buffer: wgpu::Buffer,     // Created ONCE
    pub scene_data_buffer: wgpu::Buffer, // Created ONCE (for lights)

    // 2. Persistent Bind Group
    pub global_bind_group: wgpu::BindGroup, // References the buffers above
    pub mesh_bind_group_layout: wgpu::BindGroupLayout, // References the buffers above
}

#[derive(Component, Default)]
pub struct RenderTarget {
    pub view: Option<wgpu::TextureView>,
    pub texture: Option<wgpu::SurfaceTexture>,
}

#[derive(Component)]
pub struct MaterialLayout(pub wgpu::BindGroupLayout);

pub fn register_renderings(world: &World) {
    world
        .component::<RenderTarget>()
        .add_trait::<flecs::Singleton>()
        .set(RenderTarget::default());
    world
        .component::<RenderContext>()
        .add_trait::<flecs::Singleton>();
    world
        .component::<MaterialLayout>()
        .add_trait::<flecs::Singleton>();

    world
        .system_named::<&MainWindow>("init renderer")
        .kind(flecs::pipeline::OnStart)
        .write(MainWindow::id())
        .run(|mut iter| {
            let world = iter.world();
            while iter.next() {
                let windows = iter.field::<MainWindow>(0);
                if let Some(window) = windows.get(0) {
                    println!(">>> Catalyst Renderer: Initializing GPU <<<");

                    // 2. Create the Instance (Vulkan/Metal/DX12)
                    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

                    // 3. Create Surface (The canvas on the window)
                    // UNSAFE: We must ensure the window outlives the surface. In our loop, it does.
                    let surface = unsafe {
                        instance.create_surface_unsafe(
                            wgpu::SurfaceTargetUnsafe::from_window(&window.0).unwrap(),
                        )
                    }
                    .unwrap();

                    // 4. Request Adapter (Physical GPU)
                    // We use 'pollster' to block on this async function inside a sync system
                    let adapter = pollster::block_on(instance.request_adapter(
                        &wgpu::RequestAdapterOptions {
                            power_preference: wgpu::PowerPreference::HighPerformance,
                            compatible_surface: Some(&surface),
                            force_fallback_adapter: false,
                        },
                    ))
                    .expect("No GPU found!");

                    // 5. Request Device (Logical GPU connection)
                    let (device, queue) = pollster::block_on(
                        adapter.request_device(&wgpu::DeviceDescriptor::default()),
                    )
                    .unwrap();

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

                    let depth_texture =
                        TextureHelper::create_depth_texture(&device, &config, "Depth Texture");

                    let bind_group_layout =
                        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: true,
                                        },
                                    },
                                    count: None,
                                },
                                // --- BINDING 2: Sampler ---
                                wgpu::BindGroupLayoutEntry {
                                    binding: 2,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
                                    count: None,
                                },
                                // --- METALLIC-ROUGHNESS MAP ---
                                wgpu::BindGroupLayoutEntry {
                                    binding: 3,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Texture {
                                        multisampled: false,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: true,
                                        },
                                    },
                                    count: None,
                                },
                                // SAMPLER
                                wgpu::BindGroupLayoutEntry {
                                    binding: 4,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
                                    count: None,
                                },
                                // --- NORMAL MAP ---
                                wgpu::BindGroupLayoutEntry {
                                    binding: 5,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Texture {
                                        multisampled: false,
                                        view_dimension: wgpu::TextureViewDimension::D2,
                                        sample_type: wgpu::TextureSampleType::Float {
                                            filterable: true,
                                        },
                                    },
                                    count: None,
                                },
                                // SAMPLER
                                wgpu::BindGroupLayoutEntry {
                                    binding: 6,
                                    visibility: wgpu::ShaderStages::FRAGMENT,
                                    ty: wgpu::BindingType::Sampler(
                                        wgpu::SamplerBindingType::Filtering,
                                    ),
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
                    let render_pipeline_layout =
                        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("Render Pipeline Layout"),
                            // NOW WE HAVE TWO SETS: [0: Uniforms, 1: Textures]
                            bind_group_layouts: &[
                                &bind_group_layout,
                                &material_bind_group_layout,
                                &mesh_bind_group_layout,
                            ], // No uniforms yet
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

                    let camera_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

                    let scene_data_buffer =
                        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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

                    world.set(RenderContext {
                        device,
                        queue,
                        surface,
                        config,
                        pipeline,
                        depth_texture,

                        default_diffuse: white_pixel,

                        camera_buffer,
                        scene_data_buffer,
                        global_bind_group,
                        mesh_bind_group_layout,
                    });

                    // world.insert_resource(LayoutResource(bind_group_layout));
                    world.set(MaterialLayout(material_bind_group_layout));
                }
            }
        });

    world
        .system_named::<(&RenderContext, &mut RenderTarget)>("start frame")
        .kind(flecs::pipeline::PreStore)
        .each(|(context, target)| {
            if let Ok(frame) = context.surface.get_current_texture() {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Store it so other plugins can see it
                target.texture = Some(frame);
                target.view = Some(view);
            }
        });

    let mesh_query = world
        .query::<&MeshInstance>()
        .with((AssetMesh, flecs::Wildcard))
        .with((AssetMaterial, flecs::Wildcard))
        .without(Camera::id()) // Example filter
        .group_by(AssetMaterial)
        // .order_by::<Material>(|_e1, m1: &Material, _e2, m2: &Material| m1.0.cmp(&m2.0) as i32)
        .set_cached()
        .build();

    world
        .system::<(
            &Camera,
            &GlobalTransform,
            &mut RenderContext,
            &mut RenderTarget,
        )>() // <()> = Run once (no entity matching)
        .named("Render Frame")
        .kind(flecs::pipeline::OnStore)
        //.write(RenderContext::id()) // Declare access intent
        //.write(RenderTarget::id())
        .each_entity(move |entity, (_cam, cam_t, context, target)| {
            let world = entity.world();

            let view_proj = {
                // A: View Matrix (Inverse of Camera Transform)
                // Move the world opposite to the camera
                let eye = cam_t.0.transform_point3(Vec3::ZERO);
                let forward = -cam_t.0.z_axis.truncate(); // camera looks down -Z let up = m.y_axis.truncate();
                let up = cam_t.0.y_axis.truncate();

                let view = Mat4::look_at_rh( eye, eye + forward, up, );

                // let view = Mat4::look_at_rh(
                //     cam_t.translation,                               // Eye
                //     cam_t.translation + (cam_t.rotation * -Vec3::Z), // Target (Forward is -Z)
                //     Vec3::Y,                                         // Up
                // );

                // B: Projection Matrix (Perspective)
                let aspect = context.config.width as f32 / context.config.height as f32;
                let proj = Mat4::perspective_rh(
                    45.0f32.to_radians(), // FOV
                    aspect,
                    0.1,   // Near Plane
                    100.0, // Far Plane
                );

                proj * view
            };

            // 2. Create a Command Encoder
            let mut encoder =
                context
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

                render_pass.set_bind_group(0, &context.global_bind_group, &[]);

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

                // mesh_query.run(|mut iter| {
                //     let mut last_material_uuid: Option<Uuid> = None;
                //     let mut current_material_is_valid = false;
                //     let mut last_mesh_uuid: Option<Uuid> = None;
                //     let mut current_index_count: u32 = 0;

                //     while iter.next() {
                //         let meshes = iter.field::<Mesh>(0);
                //         let materials = iter.field::<Material>(1);
                //         let instances = iter.field::<MeshInstance>(2);

                //         for i in iter.iter() {
                //             // change material if needed
                //             let current_mat_uuid = materials[i].0.id;
                //             let current_mesh_uuid = meshes[i].0.id;

                //             if last_material_uuid != Some(current_mat_uuid) {
                //                 current_material_is_valid = false;

                //                 if let Some(material_entity) = materials[i].0.try_get_entity(&world)
                //                 {
                //                     material_entity
                //                         .try_get::<&GpuMaterial>(|gpu_material| {
                //                             render_pass.set_bind_group(
                //                                 1,
                //                                 &gpu_material.bind_group,
                //                                 &[],
                //                             );
                //                         })
                //                         .and_then(|_r| {
                //                             last_material_uuid = Some(current_mat_uuid);
                //                             current_material_is_valid = true;
                //                             Some(())
                //                         });
                //                 }
                //             }

                //             if !current_material_is_valid {
                //                 continue;
                //             }

                //             if last_mesh_uuid != Some(current_mesh_uuid) {
                //                 if let Some(mesh_entity) = meshes[i].0.try_get_entity(&world) {
                //                     mesh_entity.try_get::<&GpuGeometry>(|gpu_geometry| {
                //                         render_pass.set_vertex_buffer(
                //                             0,
                //                             gpu_geometry.vertex_buffer.slice(..),
                //                         );
                //                         render_pass.set_index_buffer(
                //                             gpu_geometry.index_buffer.slice(..),
                //                             wgpu::IndexFormat::Uint32,
                //                         );

                //                         last_mesh_uuid = Some(current_mesh_uuid);
                //                         current_index_count = gpu_geometry.index_count;
                //                     });
                //                 }
                //             }

                //             if last_mesh_uuid != Some(current_mesh_uuid) {
                //                 continue;
                //             }

                //             render_pass.set_bind_group(2, &instances[i].bind_group, &[]);

                //             render_pass.draw_indexed(0..current_index_count, 0, 0..1);
                //         }
                //     }
                // });
            }

            context.queue.submit(std::iter::once(encoder.finish()));
        });

    world
        .system_named::<&mut RenderTarget>("end frame")
        .kind(flecs::pipeline::OnStore)
        .each(|target| {
            if let Some(frame) = target.texture.take() {
                frame.present();
            }
            target.view = None;
        });
}
