use catalyst_renderer::render::DebugDraw3D;
use flecs_ecs::prelude::*;
use glam::{Vec3, Vec4};

// Configuration
const GRID_SIZE: i32 = 20; // 20x20 grid
const GRID_STEP: f32 = 1.0; // 1 meter cells
const COLOR_GRAY: [f32; 4] = [0.5, 0.5, 0.5, 0.4]; // Gray, slightly transparent
const COLOR_RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0]; // X Axis
const COLOR_BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0]; // Z Axis

pub fn debug_greed_system(app: &mut catalyst_core::App) {
    app.world
        .system_named::<()>("debug_greed_render")
        .kind(flecs::pipeline::OnUpdate)
        .run(|iter| {
            iter.world().get::<&mut DebugDraw3D>(|debug| {
                let half_size = (GRID_SIZE / 2) as f32 * GRID_STEP;

                // 2. Iterate X Lines (Lines parallel to Z-axis)
                for x in -GRID_SIZE / 2..=GRID_SIZE / 2 {
                    let pos_x = x as f32 * GRID_STEP;

                    // Choose color: Axis line is Blue/Red, others are Gray
                    let color = if x == 0 { COLOR_BLUE } else { COLOR_GRAY };

                    // Start point (front), End point (back)
                    let start = [pos_x, 0.0, -half_size];
                    let end = [pos_x, 0.0, half_size];

                    debug.push_line(
                        Vec3::from_array(start),
                        Vec3::from_array(end),
                        Vec4::from_array(color),
                    );
                }

                // 3. Iterate Z Lines (Lines parallel to X-axis)
                for z in -GRID_SIZE / 2..=GRID_SIZE / 2 {
                    let pos_z = z as f32 * GRID_STEP;

                    let color = if z == 0 { COLOR_RED } else { COLOR_GRAY };

                    // Start point (left), End point (right)
                    let start = [-half_size, 0.0, pos_z];
                    let end = [half_size, 0.0, pos_z];

                    debug.push_line(
                        Vec3::from_array(start),
                        Vec3::from_array(end),
                        Vec4::from_array(color),
                    );
                }
            });
        });
}
