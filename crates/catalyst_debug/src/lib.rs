use catalyst_input::{
    context::{CTX_DEBUG, CTX_GAMEPLAY},
    logical::ActionId,
    physical::InputState,
};
use flecs_ecs::prelude::*;
use winit::window::CursorGrabMode;

use catalyst_core::{App, Plugin, SystemEvents, pipeline::PhaseRenderGUI};
use catalyst_renderer::{GpuTexture, RenderContext, RenderTarget};
use catalyst_window::MainWindow;
use egui_wgpu::ScreenDescriptor;
use wgpu::CommandEncoderDescriptor;

use crate::{
    egui_state::EguiState, greed::debug_greed_system, physics::debug_collider_render_system,
};

mod egui_state;
mod greed;
mod physics;

pub const ACTION_ENABLE_DEBUG: ActionId = ActionId(201);

#[derive(Component, Clone, Copy, Debug, Hash)]
pub struct DebugTexture(pub egui::epaint::TextureId);

#[derive(Component, Default, Clone, Copy, Debug, Hash)]
pub struct GuiState {
    pub enabled: bool,
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .component::<EguiState>()
            .add_trait::<flecs::Singleton>();

        app.register_singleton_default::<GuiState>();

        debug_collider_render_system(app);
        debug_greed_system(app);

        app.world
            .system_named::<(&mut GuiState, &mut InputState)>("debug_inputs")
            .kind(flecs::pipeline::OnUpdate)
            .run(|mut iter| {
                while iter.next() {
                    let mut gui_state_field = iter.field_mut::<GuiState>(0);
                    let mut input_state_field = iter.field_mut::<InputState>(1);

                    if let (Some(gui_state), Some(input)) =
                        (gui_state_field.get_mut(0), input_state_field.get_mut(0))
                    {
                        if input.just_pressed(ACTION_ENABLE_DEBUG) {
                            gui_state.enabled = !gui_state.enabled;
                            println!("Debug GUI Enabled: {}", gui_state.enabled);

                            if gui_state.enabled {
                                input.set_context(CTX_DEBUG);
                            } else {
                                input.set_context(CTX_GAMEPLAY);
                            }
                        }
                    }
                }
            });

        app.world
            .system::<(&RenderContext, &MainWindow)>()
            .kind(flecs::pipeline::OnUpdate)
            .without(EguiState::id())
            .run(|mut iter| {
                let world = iter.world();

                while iter.next() {
                    let context_field = iter.field::<RenderContext>(0);
                    let window_field = iter.field::<MainWindow>(1);

                    if let (Some(context), Some(window)) =
                        (context_field.get(0), window_field.get(0))
                    {
                        println!("Context is ready. Initializing EguiState...");

                        let egui_state = EguiState::new(&context, &window.0);

                        world.set(egui_state);
                    }
                }
            });

        app.world
            .observer::<flecs::OnSet, (&GpuTexture, &mut EguiState, &RenderContext)>()
            .each_entity(|entity, (texture, egui_state, context)| {
                if texture.texture.size().depth_or_array_layers == 1 {
                    let teture_handle = egui_state.renderer.register_native_texture(
                        &context.device,
                        &texture.view,
                        wgpu::FilterMode::Linear,
                    );

                    entity.set(DebugTexture(teture_handle));
                }
            });

        let textures_to_debug = app
            .world
            .query_named::<&DebugTexture>("textures_to_debug")
            .set_cached()
            .build();

        app.world
            .system_named::<(
                &mut EguiState,
                &SystemEvents,
                &MainWindow,
                &RenderTarget,
                &RenderContext,
                &GuiState,
            )>("render_debug_ui")
            .kind(PhaseRenderGUI)
            .run(move |mut iter| {
                while iter.next() {
                    let mut egui_state_field = iter.field_mut::<EguiState>(0);
                    let system_events_field = iter.field::<SystemEvents>(1);
                    let window_field = iter.field::<MainWindow>(2);
                    let target_field = iter.field::<RenderTarget>(3);
                    let context_field = iter.field::<RenderContext>(4);
                    let gui_state_field = iter.field::<GuiState>(5);

                    if let (
                        Some(egui_state),
                        Some(window),
                        Some(target),
                        Some(context),
                        Some(gui_state),
                    ) = (
                        egui_state_field.get_mut(0),
                        window_field.get(0),
                        target_field.get(0),
                        context_field.get(0),
                        gui_state_field.get(0),
                    ) {
                        for event in &system_events_field[0].buffer {
                            let _ = egui_state.state.on_window_event(&window.0, event);
                        }

                        let raw_input = egui_state.state.take_egui_input(&window.0);
                        egui_state.context.begin_pass(raw_input);
                        let ctx = &egui_state.context;

                        if !gui_state.enabled {
                            return;
                        }

                        window.0.set_cursor_grab(CursorGrabMode::None).unwrap();
                        window.0.set_cursor_visible(true);

                        let mut valid_textures = Vec::new();

                        textures_to_debug.each_entity(|entity, texture| {
                            valid_textures.push((entity.name(), texture.clone()));
                        });

                        egui::Window::new("Debug Texture").show(ctx, |ui| {
                            ui.heading("Debug Texture Window");

                            if valid_textures.is_empty() {
                                ui.label("No textures found with registered Egui IDs.");
                                return;
                            }

                            if let Some(t) = valid_textures.get(1) {
                                ui.image((t.1.0, egui::vec2(512.0, 512.0)));
                            }
                        });

                        // 6. Render
                        let view = match &target.view {
                            Some(v) => v,
                            None => return,
                        };
                        let full_output = egui_state.context.end_pass();
                        egui_state
                            .state
                            .handle_platform_output(&window.0, full_output.platform_output);
                        let paint_jobs = egui_state
                            .context
                            .tessellate(full_output.shapes, egui_state.context.pixels_per_point());
                        let screen_descriptor = ScreenDescriptor {
                            size_in_pixels: [context.config.width, context.config.height],
                            pixels_per_point: window.0.scale_factor() as f32,
                        };
                        for (id, delta) in &full_output.textures_delta.set {
                            egui_state.renderer.update_texture(
                                &context.device,
                                &context.queue,
                                *id,
                                delta,
                            );
                        }
                        let mut encoder =
                            context
                                .device
                                .create_command_encoder(&CommandEncoderDescriptor {
                                    label: Some("Debug UI"),
                                });
                        egui_state.renderer.update_buffers(
                            &context.device,
                            &context.queue,
                            &mut encoder,
                            &paint_jobs,
                            &screen_descriptor,
                        );
                        {
                            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Egui Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: view,
                                    resolve_target: None,
                                    depth_slice: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                            let mut pass = pass.forget_lifetime();
                            egui_state
                                .renderer
                                .render(&mut pass, &paint_jobs, &screen_descriptor);
                        }
                        for id in &full_output.textures_delta.free {
                            egui_state.renderer.free_texture(id);
                        }
                        context.queue.submit(std::iter::once(encoder.finish()));
                    }
                }
            });
    }
}
