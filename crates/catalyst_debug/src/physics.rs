use catalyst_core::{physics::ColliderDefinition, transform::GlobalTransform};
use catalyst_physics::{PhysicsWorld, prepare::PhysicsHandle};
use catalyst_renderer::render::DebugDraw3D;
use flecs_ecs::prelude::*;
use glam::Vec3;

pub fn debug_collider_render_system(app: &mut catalyst_core::App) {
    app.world
        .system_named::<(
            &ColliderDefinition,
            &PhysicsHandle,
            &PhysicsWorld,
            Option<&GlobalTransform>,
            &mut DebugDraw3D,
        )>("debug_collider_render")
        .kind(flecs::pipeline::OnUpdate)
        .term_at(3)
        .parent()
        .each(|(col_def, handle, physics, global, debug)| {
            if let Some(collider_handle) = handle.collider {
                if let Some(collider) = physics.colliders.get(collider_handle) {
                    // World transform of collider
                    let iso = collider.position(); // nalgebra Isometry 
                    let pos: glam::Vec3 = iso.translation;
                    let rot: glam::Quat = iso.rotation;

                    let color = glam::vec4(0.0, 1.0, 0.0, 1.0); // green wireframe

                    let global_scale = global
                        .map(|s| s.to_scale_rotation_translation().0)
                        .unwrap_or(Vec3::from_array([1f32, 1f32, 1f32]));

                    match &col_def.shape {
                        catalyst_core::physics::ColliderShape::Box { hx, hy, hz } => {
                            draw_box(
                                debug,
                                pos,
                                rot,
                                *hx * global_scale.x,
                                *hy * global_scale.y,
                                *hz * global_scale.z,
                                color,
                            );
                        }
                        _ => todo!(),
                    }
                }
            }
        });
}

fn draw_box(
    debug: &mut DebugDraw3D,
    pos: glam::Vec3,
    rot: glam::Quat,
    hx: f32,
    hy: f32,
    hz: f32,
    color: glam::Vec4,
) {
    let corners = [
        glam::Vec3::new(-hx, -hy, -hz),
        glam::Vec3::new(hx, -hy, -hz),
        glam::Vec3::new(hx, hy, -hz),
        glam::Vec3::new(-hx, hy, -hz),
        glam::Vec3::new(-hx, -hy, hz),
        glam::Vec3::new(hx, -hy, hz),
        glam::Vec3::new(hx, hy, hz),
        glam::Vec3::new(-hx, hy, hz),
    ];

    let corners: Vec<glam::Vec3> = corners.iter().map(|c| pos + rot * *c).collect();

    let edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];

    for (a, b) in edges {
        debug.push_line(corners[a], corners[b], color);
    }
}
