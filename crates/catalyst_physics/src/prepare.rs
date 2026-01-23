use catalyst_core::{
    physics::{PhysicsBody, RigidBodyDefinition},
    pipeline::PhysicsPrepare,
    transform::GlobalTransform,
};
use flecs_ecs::prelude::*;
use nalgebra::{Isometry, Translation};
use rapier3d::prelude::*;

use crate::PhysicsWorld;

#[derive(Component, Debug, Clone, Copy)]
pub struct PhysicsHandle {
    pub body: Option<RigidBodyHandle>,
    pub collider: Option<ColliderHandle>,
}

impl PhysicsHandle {
    pub fn new_body(handle: RigidBodyHandle) -> Self {
        Self {
            body: Some(handle),
            collider: None,
        }
    }
    pub fn new_collider(body: RigidBodyHandle, collider: ColliderHandle) -> Self {
        Self {
            body: Some(body),
            collider: Some(collider),
        }
    }
}

pub fn prepare_physics_system(app: &catalyst_core::App) {
    app.world
        .system_named::<(
            &GlobalTransform,
            &RigidBodyDefinition,
            Option<&PhysicsHandle>,
            &mut PhysicsWorld,
        )>("prepare_physics")
        .kind(PhysicsPrepare)
        .each_entity(|entity, (transform, rb_def, physics_handle, physics)| {
            if let Some(handle) = physics_handle {
                let body = handle.body.and_then(|body| physics.bodies.get_mut(body));

                if let Some(b) = body {
                    b.set_linear_damping(rb_def.linear_damping);
                    b.set_angular_damping(rb_def.angular_damping);
                    b.set_gravity_scale(rb_def.gravity_scale, true);

                    let iso = global_to_iso(transform);
                    b.set_position(iso, true);
                }
            } else {
                let world = entity.world();

                // Create new Rapier body
                let rb_type = match rb_def.body_type {
                    PhysicsBody::Dynamic => RigidBodyType::Dynamic,
                    PhysicsBody::Static => RigidBodyType::Fixed,
                    PhysicsBody::Kinematic => RigidBodyType::KinematicPositionBased,
                    PhysicsBody::Unknown => RigidBodyType::Dynamic,
                };

                let mut body = RigidBodyBuilder::new(rb_type)
                    .pose(Pose::from_mat4(transform.0))
                    .linear_damping(rb_def.linear_damping)
                    .angular_damping(rb_def.angular_damping)
                    .gravity_scale(rb_def.gravity_scale)
                    .build();

                if let Some(mass) = rb_def.mass {
                    body.set_additional_mass(mass, true);
                }

                let body_handle = physics.bodies.insert(body);

                entity.set(PhysicsHandle {
                    body: Some(body_handle),
                    collider: Some(ColliderHandle::invalid()),
                });
            }
        });
}

fn global_to_iso(gt: &GlobalTransform) -> Pose3 {
    let (translation, rotation, _) = gt.0.to_scale_rotation_translation();
    Isometry::from_parts(Translation::from(translation), rotation.into()).into()
}
