use catalyst_assets::material::{TextureData, TextureFormat};
use catalyst_core::{
    App,
    camera::Camera,
    pipeline::{PhasePresent, PhaseRender3D},
    transform::GlobalTransform,
};
use catalyst_window::MainWindow;
use flecs_ecs::prelude::*;
use glam::{Mat4, Vec3, Vec4};
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};

use crate::{
    global_resources::{GlobalResources, GpuPointLight, LightUniforms},
    material::AssetMaterial,
    mesh::{AssetMesh, MeshInstance},
    programs::{
        self, DebugLinesProgram, GpuProgram, PbrProgram, debug_lines_program::DebugLineVertex,
    },
    texture::{GpuTexture, TextureHelper},
};

#[derive(Component)]
pub struct RenderContext {
    pub device: Device,
    pub queue: Queue,
    pub surface: Surface<'static>,
    pub config: SurfaceConfiguration,
    pub depth_texture: wgpu::TextureView,

    pub default_diffuse: GpuTexture,

    pub global_resources: GlobalResources,

    pub pbr_program: PbrProgram,
    pub debug_lines_program: DebugLinesProgram,
}

#[derive(Component, Default)]
pub struct DebugDraw3D {
    pub debug_line_vertices: Vec<DebugLineVertex>,
}

impl DebugDraw3D {
    pub fn push_line(&mut self, start: Vec3, end: Vec3, color: Vec4) {
        self.debug_line_vertices.push(DebugLineVertex {
            position: start.into(),
            color: color.into(),
        });
        self.debug_line_vertices.push(DebugLineVertex {
            position: end.into(),
            color: color.into(),
        });
    }
}

#[derive(Component, Default)]
pub struct RenderTarget {
    pub view: Option<wgpu::TextureView>,
    pub texture: Option<wgpu::SurfaceTexture>,
}

#[derive(Component)]
pub struct MaterialLayout(pub wgpu::BindGroupLayout);

pub fn register_renderings(app: &mut App) {
    app.register_singleton_default::<DebugDraw3D>();

    app.world
        .component::<RenderTarget>()
        .add_trait::<flecs::Singleton>()
        .set(RenderTarget::default());
    app.world
        .component::<RenderContext>()
        .add_trait::<flecs::Singleton>();
    app.world
        .component::<MaterialLayout>()
        .add_trait::<flecs::Singleton>();

    app.world
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

                    let depth_texture =
                        TextureHelper::create_depth_texture(&device, &config, "Depth Texture");

                    let global_resources = GlobalResources::new(&device);

                    let render_context = programs::GpuProgramRenderContext {
                        device: &device,
                        queue: &queue,
                        format: config.format,
                    };

                    let pbr_program = PbrProgram::new(&render_context, &global_resources.layout);
                    let debug_lines_program =
                        DebugLinesProgram::new(&render_context, &global_resources.layout);

                    //let line_draw_pipeline = create_line_draw_pipeline(&device, &bind_group_layout, &config);

                    // --- CREATE DEFAULT TEXTURE (1x1 White Pixel) ---
                    // We create this manually so we don't depend on an asset file existing
                    let white_pixel = GpuTexture::from_image(
                        &device,
                        &queue,
                        &TextureData {
                            name: "Default White Pixel".to_string(),
                            width: 1,
                            height: 1,
                            // RGBA: (255, 255, 255, 255) -> Solid White
                            pixels: vec![0, 111, 255, 255],
                            format: TextureFormat::Rgba8Unorm,
                        },
                        Some("Default White Texture"),
                    );

                    println!(">>> Catalyst Renderer: Pipeline Compiled <<<");

                    world.set(MaterialLayout(pbr_program.material_layout.clone()));

                    world.set(RenderContext {
                        device,
                        queue,
                        surface,
                        config,
                        depth_texture,

                        global_resources,
                        default_diffuse: white_pixel,

                        pbr_program,
                        debug_lines_program,
                    });

                    // world.insert_resource(LayoutResource(bind_group_layout));
                }
            }
        });

    app.world
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

    let mesh_query = app
        .world
        .query::<&MeshInstance>()
        .with((AssetMesh, flecs::Wildcard))
        .with((AssetMaterial, flecs::Wildcard))
        .without(Camera::id()) // Example filter
        .group_by(AssetMaterial)
        // .order_by::<Material>(|_e1, m1: &Material, _e2, m2: &Material| m1.0.cmp(&m2.0) as i32)
        .set_cached()
        .build();

    app.world
        .system::<(
            &Camera,
            &GlobalTransform,
            &mut RenderContext,
            &mut RenderTarget,
        )>() // <()> = Run once (no entity matching)
        .named("Render Frame")
        .kind(PhaseRender3D)
        //.write(RenderContext::id()) // Declare access intent
        //.write(RenderTarget::id())
        .each(move |(_cam, cam_t, context, target)| {
            let view_proj = {
                // A: View Matrix (Inverse of Camera Transform)
                // Move the world opposite to the camera
                let eye = cam_t.0.transform_point3(Vec3::ZERO);
                let forward = -cam_t.0.z_axis.truncate(); // camera looks down -Z let up = m.y_axis.truncate();
                let up = cam_t.0.y_axis.truncate();

                let view = Mat4::look_at_rh(eye, eye + forward, up);

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

                let debug_light = GpuPointLight {
                    position: [2.0, 2.0, 2.0, 0.0], // .w can be ignored or used for radius
                    color: [1.0, 0.2, 0.2, 10.0],   // Red color, High Intensity (10.0)
                };

                // Create 3 empty lights
                let empty_light = GpuPointLight {
                    position: [0.0; 4],
                    color: [0.0; 4],
                };

                // Update lights
                let light_data = LightUniforms {
                    sun_direction: [0.0, -1.0, -0.5, 5.0],
                    sun_color: [1.0, 1.0, 1.0, 0.0],
                    point_lights: [debug_light, empty_light, empty_light, empty_light],
                    // camera_pos: [0.0, 0.0, 0.0, 0.0],
                    //camera_pos: cam_t.0.w_axis.to_array(),
                    camera_pos: cam_t.0.transform_point3(Vec3::ZERO).to_array(),
                    active_lights: 1,
                };

                context
                    .global_resources
                    .update_camera(&context.queue, view_proj);
                context
                    .global_resources
                    .update_lights(&context.queue, light_data);

                context.pbr_program.record(
                    &mut render_pass,
                    (&context.global_resources.bind_group, &mesh_query),
                );

                context
                    .debug_lines_program
                    .record(&mut render_pass, &context.global_resources.bind_group);

                // context
                //     .debug_lines_program
                //     .record(&mut render_pass, (&context.global_resources.bind_group));
            }

            context.queue.submit(std::iter::once(encoder.finish()));
        });

    app.world
        .system_named::<&mut RenderTarget>("end frame")
        .kind(PhasePresent)
        .each(|target| {
            if let Some(frame) = target.texture.take() {
                frame.present();
            }
            target.view = None;
        });
}
