use catalyst_input::{
    context::{CTX_DEBUG, CTX_GAMEPLAY}, logical::{ActionId, ButtonPhase}, physical::InputState
};
use flecs_ecs::prelude::*;
use std::collections::HashMap;
use winit::window::CursorGrabMode;

// FIX: Ensure ChildOf and Transform are imported correctly
use catalyst_core::{
    App,
    Plugin,
    SystemEvents,
    pipeline::PhaseRenderGUI,
    transform::{GlobalTransform, Transform}, // <--- Added Transform here
};
use catalyst_renderer::{GpuTexture, RenderContext, RenderTarget};
use catalyst_window::MainWindow;
use egui_wgpu::ScreenDescriptor;
use wgpu::CommandEncoderDescriptor;

use crate::{egui_state::EguiState, physics::debug_collider_render_system};

mod egui_state;
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
                let teture_handle = egui_state.renderer.register_native_texture(
                    &context.device,
                    &texture.view,
                    wgpu::FilterMode::Linear,
                );

                entity.set(DebugTexture(teture_handle));
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

        // app.add_system_to_stage(
        //     Stage::Render,
        //     debug_render_system
        //         .in_set(RenderSet::Overlay)
        //         .run_if(resource_exists::<EguiState>)
        // );
    }
}

// fn init_egui_state(
//     mut commands: Commands,
//     context: Res<RenderContext>,
//     window_res: NonSend<MainWindow>,
// ) {
//     let window = &window_res.0;
//     let egui_state = EguiState::new(&context, window);
//     commands.insert_resource(egui_state);
// }

// --- MAIN RENDER SYSTEM ---
// fn debug_render_system(world: &mut World) {
//     world.resource_scope::<EguiState, _>(|world, mut egui_state| {
//         world.resource_scope::<InspectorState, _>(|world, mut inspector_state| {

//             // 1. Build Hierarchy Map
//             let mut child_map: HashMap<Entity, Vec<Entity>> = HashMap::new();
//             let mut roots: Vec<Entity> = Vec::new();

//             for entity_ref in world.iter_entities() {
//                 let e = entity_ref.id();
//                 if let Some(child_of) = entity_ref.get::<ChildOf>() {
//                     // Note: Check if your version uses child_of.parent or child_of.0
//                     let parent = child_of.parent();
//                     child_map.entry(parent).or_insert_with(Vec::new).push(e);
//                 } else {
//                     roots.push(e);
//                 }
//             }

//             // 2. Setup
//             let system_events = world.resource::<SystemEvents>();
//             let window_res = world.non_send_resource::<MainWindow>();
//             let window = &window_res.0;
//             let current_selection = inspector_state.selected;
//             let mut new_selection = None;

//             for event in &system_events.buffer {
//                 let _ = egui_state.state.on_window_event(window, event);
//             }

//             let raw_input = egui_state.state.take_egui_input(window);
//             egui_state.context.begin_pass(raw_input);
//             let ctx = &egui_state.context;

//             // 3. Left Panel (Hierarchy)
//             egui::SidePanel::left("entity_list").show(ctx, |ui| {
//                 ui.heading("Hierarchy");
//                 ui.separator();
//                 egui::ScrollArea::vertical().show(ui, |ui| {
//                     for root in roots {
//                         draw_entity_node(ui, world, root, current_selection, &mut new_selection, &child_map);
//                     }
//                 });
//             });

//             // 4. Right Panel (Inspector)
//             egui::SidePanel::right("inspector").show(ctx, |ui| {
//                 ui.heading("Inspector");
//                 ui.separator();

//                 if let Some(e) = current_selection {
//                     if let Ok(entity_ref) = world.get_entity(e) {
//                         ui.label(format!("ID: {:?}", e));
//                         ui.separator();

//                         // --- GLOBAL TRANSFORM ---
//                         if let Some(g_transform) = entity_ref.get::<GlobalTransform>() {
//                             ui.label(egui::RichText::new("Global Transform").strong());
//                             // Decompose Mat4
//                             let (scale, rotation, translation) = g_transform.to_scale_rotation_translation();

//                             ui.vertical(|ui| {
//                                 ui.label(format!("Pos:   {:.2}, {:.2}, {:.2}", translation.x, translation.y, translation.z));
//                                 ui.label(format!("Rot:   {:.2}, {:.2}, {:.2}, {:.2}", rotation.x, rotation.y, rotation.z, rotation.w));
//                                 ui.label(format!("Scale: {:.2}, {:.2}, {:.2}", scale.x, scale.y, scale.z));
//                             });
//                             ui.separator();
//                         }

//                         // --- LOCAL TRANSFORM ---
//                         if let Some(transform) = entity_ref.get::<Transform>() {
//                             ui.label(egui::RichText::new("Local Transform").strong());

//                             // Assuming Transform struct has public fields: translation, rotation, scale
//                             ui.vertical(|ui| {
//                                 ui.label(format!("Pos:   {:.2}, {:.2}, {:.2}", transform.translation.x, transform.translation.y, transform.translation.z));
//                                 ui.label(format!("Rot:   {:.2}, {:.2}, {:.2}, {:.2}", transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w));
//                                 ui.label(format!("Scale: {:.2}, {:.2}, {:.2}", transform.scale.x, transform.scale.y, transform.scale.z));
//                             });
//                             ui.separator();
//                         }

//                         // --- HIERARCHY INFO ---
//                         if let Some(child_of) = entity_ref.get::<ChildOf>() {
//                             ui.label(format!("Parent ID: {:?}", child_of.parent()));
//                         } else {
//                             ui.label("Parent: None (Root)");
//                         }

//                         if let Some(children) = child_map.get(&e) {
//                             ui.label(format!("Children Count: {}", children.len()));
//                         }

//                     } else {
//                         ui.label("Entity Despawned");
//                     }
//                 } else {
//                     ui.label("Select an entity.");
//                 }
//             });

//             // 5. Update Selection
//             if let Some(s) = new_selection {
//                 inspector_state.selected = Some(s);
//             }

//             // 6. Render
//             let context = world.resource::<RenderContext>();
//             let target = world.resource::<RenderTarget>();

//             let view = match &target.view {
//                 Some(v) => v,
//                 None => return,
//             };
//             let full_output = egui_state.context.end_pass();
//             egui_state.state.handle_platform_output(window, full_output.platform_output);
//             let paint_jobs = egui_state.context.tessellate(full_output.shapes, egui_state.context.pixels_per_point());
//             let screen_descriptor = ScreenDescriptor {
//                 size_in_pixels: [context.config.width, context.config.height],
//                 pixels_per_point: window.scale_factor() as f32,
//             };
//             for (id, delta) in &full_output.textures_delta.set {
//                 egui_state.renderer.update_texture(&context.device, &context.queue, *id, delta);
//             }
//             let mut encoder = context.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Debug UI") });
//             egui_state.renderer.update_buffers(&context.device, &context.queue, &mut encoder, &paint_jobs, &screen_descriptor);
//             {
//                 let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//                     label: Some("Egui Pass"),
//                     color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                         view: view,
//                         resolve_target: None,
//                         depth_slice: None,
//                         ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
//                     })],
//                     depth_stencil_attachment: None,
//                     timestamp_writes: None,
//                     occlusion_query_set: None,
//                 });
//                 let mut pass = pass.forget_lifetime();
//                 egui_state.renderer.render(&mut pass, &paint_jobs, &screen_descriptor);
//             }
//             for id in &full_output.textures_delta.free { egui_state.renderer.free_texture(id); }
//             context.queue.submit(std::iter::once(encoder.finish()));
//         });
//     });
// }
