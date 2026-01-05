use std::collections::HashMap;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::common_conditions::{resource_exists, not};
use bevy_ecs::entity::Entity;
// FIX: Ensure ChildOf and Transform are imported correctly
use catalyst_core::{
    App, Plugin, Stage, SystemEvents, 
    transform::{GlobalTransform, Transform} // <--- Added Transform here
};
use catalyst_renderer::{RenderContext, RenderSet, RenderTarget};
use catalyst_window::MainWindow;
use egui_wgpu::ScreenDescriptor;
use wgpu::CommandEncoderDescriptor;

use crate::egui_state::EguiState;

mod egui_state;

#[derive(Resource, Default)]
pub struct InspectorState {
    pub selected: Option<Entity>,
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.world.init_resource::<InspectorState>();

        app.add_system_to_stage(
            Stage::PreUpdate, 
            init_egui_state.run_if(
                resource_exists::<RenderContext>
                    .and(not(resource_exists::<EguiState>))
            )
        );

        app.add_system_to_stage(
            Stage::Render,
            debug_render_system
                .in_set(RenderSet::Overlay)
                .run_if(resource_exists::<EguiState>)
        );
    }
}

fn init_egui_state(
    mut commands: Commands,
    context: Res<RenderContext>,
    window_res: NonSend<MainWindow>,
) {
    let window = &window_res.0;
    let egui_state = EguiState::new(&context, window);
    commands.insert_resource(egui_state);
}

// --- HIERARCHY DRAWER ---
fn draw_entity_node(
    ui: &mut egui::Ui,
    world: &World,
    entity: Entity,
    selected: Option<Entity>,
    new_selection: &mut Option<Entity>,
    child_map: &HashMap<Entity, Vec<Entity>>,
) {
    let entity_ref = match world.get_entity(entity) {
        Ok(e) => e,
        Err(_) => return, 
    };

    let suffix = if entity_ref.contains::<GlobalTransform>() { " (T)" } else { "" };
    let label = format!("Entity {:?}{}", entity, suffix);
    let is_selected = selected == Some(entity);

    if let Some(children) = child_map.get(&entity) {
        let id = ui.make_persistent_id(entity);
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
            .show_header(ui, |ui| {
                if ui.selectable_label(is_selected, label).clicked() {
                    *new_selection = Some(entity);
                }
            })
            .body(|ui| {
                for &child in children {
                    draw_entity_node(ui, world, child, selected, new_selection, child_map);
                }
            });
    } else {
        if ui.selectable_label(is_selected, label).clicked() {
            *new_selection = Some(entity);
        }
    }
}

// --- MAIN RENDER SYSTEM ---
fn debug_render_system(world: &mut World) {
    world.resource_scope::<EguiState, _>(|world, mut egui_state| {
        world.resource_scope::<InspectorState, _>(|world, mut inspector_state| {
            
            // 1. Build Hierarchy Map
            let mut child_map: HashMap<Entity, Vec<Entity>> = HashMap::new();
            let mut roots: Vec<Entity> = Vec::new();

            for entity_ref in world.iter_entities() {
                let e = entity_ref.id();
                if let Some(child_of) = entity_ref.get::<ChildOf>() {
                    // Note: Check if your version uses child_of.parent or child_of.0
                    let parent = child_of.parent(); 
                    child_map.entry(parent).or_insert_with(Vec::new).push(e);
                } else {
                    roots.push(e);
                }
            }

            // 2. Setup
            let system_events = world.resource::<SystemEvents>();
            let window_res = world.non_send_resource::<MainWindow>();
            let window = &window_res.0;
            let current_selection = inspector_state.selected;
            let mut new_selection = None; 

            for event in &system_events.buffer {
                let _ = egui_state.state.on_window_event(window, event);
            }

            let raw_input = egui_state.state.take_egui_input(window);
            egui_state.context.begin_pass(raw_input);
            let ctx = &egui_state.context;

            // 3. Left Panel (Hierarchy)
            egui::SidePanel::left("entity_list").show(ctx, |ui| {
                ui.heading("Hierarchy");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for root in roots {
                        draw_entity_node(ui, world, root, current_selection, &mut new_selection, &child_map);
                    }
                });
            });

            // 4. Right Panel (Inspector)
            egui::SidePanel::right("inspector").show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();

                if let Some(e) = current_selection {
                    if let Ok(entity_ref) = world.get_entity(e) {
                        ui.label(format!("ID: {:?}", e));
                        ui.separator();

                        // --- GLOBAL TRANSFORM ---
                        if let Some(g_transform) = entity_ref.get::<GlobalTransform>() {
                            ui.label(egui::RichText::new("Global Transform").strong());
                            // Decompose Mat4
                            let (scale, rotation, translation) = g_transform.to_scale_rotation_translation();
                            
                            ui.vertical(|ui| {
                                ui.label(format!("Pos:   {:.2}, {:.2}, {:.2}", translation.x, translation.y, translation.z));
                                ui.label(format!("Rot:   {:.2}, {:.2}, {:.2}, {:.2}", rotation.x, rotation.y, rotation.z, rotation.w));
                                ui.label(format!("Scale: {:.2}, {:.2}, {:.2}", scale.x, scale.y, scale.z));
                            });
                            ui.separator();
                        }

                        // --- LOCAL TRANSFORM ---
                        if let Some(transform) = entity_ref.get::<Transform>() {
                            ui.label(egui::RichText::new("Local Transform").strong());
                            
                            // Assuming Transform struct has public fields: translation, rotation, scale
                            ui.vertical(|ui| {
                                ui.label(format!("Pos:   {:.2}, {:.2}, {:.2}", transform.translation.x, transform.translation.y, transform.translation.z));
                                ui.label(format!("Rot:   {:.2}, {:.2}, {:.2}, {:.2}", transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w));
                                ui.label(format!("Scale: {:.2}, {:.2}, {:.2}", transform.scale.x, transform.scale.y, transform.scale.z));
                            });
                            ui.separator();
                        }

                        // --- HIERARCHY INFO ---
                        if let Some(child_of) = entity_ref.get::<ChildOf>() {
                            ui.label(format!("Parent ID: {:?}", child_of.parent()));
                        } else {
                            ui.label("Parent: None (Root)");
                        }
                        
                        if let Some(children) = child_map.get(&e) {
                            ui.label(format!("Children Count: {}", children.len()));
                        }

                    } else {
                        ui.label("Entity Despawned");
                    }
                } else {
                    ui.label("Select an entity.");
                }
            });

            // 5. Update Selection
            if let Some(s) = new_selection {
                inspector_state.selected = Some(s);
            }

            // 6. Render
            let context = world.resource::<RenderContext>();
            let target = world.resource::<RenderTarget>();
            
            let view = match &target.view {
                Some(v) => v,
                None => return,
            };
            let full_output = egui_state.context.end_pass();
            egui_state.state.handle_platform_output(window, full_output.platform_output);
            let paint_jobs = egui_state.context.tessellate(full_output.shapes, egui_state.context.pixels_per_point());
            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: [context.config.width, context.config.height],
                pixels_per_point: window.scale_factor() as f32,
            };
            for (id, delta) in &full_output.textures_delta.set {
                egui_state.renderer.update_texture(&context.device, &context.queue, *id, delta);
            }
            let mut encoder = context.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Debug UI") });
            egui_state.renderer.update_buffers(&context.device, &context.queue, &mut encoder, &paint_jobs, &screen_descriptor);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Egui Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let mut pass = pass.forget_lifetime(); 
                egui_state.renderer.render(&mut pass, &paint_jobs, &screen_descriptor);
            }
            for id in &full_output.textures_delta.free { egui_state.renderer.free_texture(id); }
            context.queue.submit(std::iter::once(encoder.finish()));
        });
    });
}