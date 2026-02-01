use catalyst_core::{
    physics::{ColliderDefinition, PhysicsBody, PhysicsMaterialDefinition, RigidBodyDefinition},
    pipeline::PhysicsPrepare,
    transform::{GlobalTransform, Transform},
};
use flecs_ecs::prelude::*;
use glam::Mat4;
use nalgebra::{Isometry, Translation};
use rapier3d::prelude::*;

use crate::{PhysicsBodyAdded, PhysicsColliderAdded, PhysicsWorld};

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
        )>("prepare_physic_bodies")
        .kind(PhysicsPrepare)
        .each_entity(|entity, (transform, rb_def, physics_handle, physics)| {
            if let Some(handle) = physics_handle {
                let body = handle.body.and_then(|body| physics.bodies.get_mut(body));

                if let Some(b) = body {
                    b.set_linear_damping(rb_def.linear_damping);
                    b.set_angular_damping(rb_def.angular_damping);
                    b.set_gravity_scale(rb_def.gravity_scale, true);

                    let iso = mat_to_iso(&transform.0);
                    b.set_position(iso, true);
                }
            } else {
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

                entity.add(PhysicsBodyAdded);

                entity.set(PhysicsHandle {
                    body: Some(body_handle),
                    collider: Some(ColliderHandle::invalid()),
                });
            }
        });

    app.world
        .system_named::<(
            &Transform,
            &ColliderDefinition,
            Option<&PhysicsMaterialDefinition>,
            Option<&PhysicsHandle>,
            &PhysicsHandle,
            Option<&GlobalTransform>,
            &mut PhysicsWorld,
        )>("prepare_physic_coliders")
        .kind(PhysicsPrepare)
        .term_at(4)
        .parent()
        .term_at(5)
        .parent()
        .each_entity(
            |entity,
             (
                local_transform,
                col_def,
                mat_def,
                collider_handle,
                parent_handle,
                parent_transform,
                physics,
            )| {
                if let Some(handle) = collider_handle {
                    let collider = handle.collider.and_then(|c| physics.colliders.get_mut(c));
                    if let Some(c) = collider {
                        // Update material
                        if let Some(mat) = mat_def {
                            c.set_friction(mat.friction);
                            c.set_restitution(mat.restitution);
                        }

                        // Update collision groups
                        let groups = InteractionGroups::new(
                            Group::from_bits(col_def.layer).unwrap(),
                            Group::from_bits(col_def.mask).unwrap(),
                            InteractionTestMode::default(),
                        );
                        c.set_collision_groups(groups);

                        // Update local offset
                        let iso = mat_to_iso(&local_transform.compute_matrix());
                        c.set_position_wrt_parent(iso);
                    }
                } else {
                    let global_scale = parent_transform
                        .map(|s| s.to_scale_rotation_translation().0)
                        .unwrap_or(Vec3::from_array([1f32, 1f32, 1f32]));

                    // Build collider shape
                    let builder = match &col_def.shape {
                        catalyst_core::physics::ColliderShape::Box { hx, hy, hz } => {
                            ColliderBuilder::cuboid(
                                *hx * global_scale.x,
                                *hy * global_scale.y,
                                *hz * global_scale.z,
                            )
                        }
                        catalyst_core::physics::ColliderShape::Sphere { radius } => {
                            ColliderBuilder::ball(*radius)
                        }
                        catalyst_core::physics::ColliderShape::Capsule { radius, height } => {
                            ColliderBuilder::capsule_y(*height * 0.5, *radius)
                        }
                        _ => todo!("Convex and Mesh shapes not supported"), // catalyst_core::physics::ColliderShape::Convex { vertices } => ColliderBuilder::convex_hull( &vertices.iter().map(|v| v.into()).collect::<Vec<_>>() ).unwrap(),
                                                                            // catalyst_core::physics::ColliderShape::Mesh { vertices, indices } => ColliderBuilder::trimesh( vertices.iter().map(|v| v.into()).collect(), indices.chunks(3).map(|c| [c[0], c[1], c[2]]).collect(), ),
                    };

                    let iso = mat_to_iso(&local_transform.compute_matrix());
                    let mut collider = builder
                        .collision_groups(InteractionGroups::new(
                            Group::from_bits(col_def.layer).unwrap(),
                            Group::from_bits(col_def.mask).unwrap(),
                            InteractionTestMode::default(),
                        ))
                        .sensor(col_def.is_trigger)
                        .position(iso)
                        .build();

                    if let Some(mat) = mat_def {
                        collider.set_friction(mat.friction);
                        collider.set_restitution(mat.restitution);
                    }

                    let collider_handle = physics.colliders.insert_with_parent(
                        collider,
                        parent_handle.body.unwrap(),
                        &mut physics.bodies,
                    );

                    entity.add(PhysicsColliderAdded);

                    entity.set(PhysicsHandle {
                        body: parent_handle.body,
                        collider: Some(collider_handle),
                    });
                }
            },
        );
}

fn mat_to_iso(gt: &Mat4) -> Pose3 {
    let (_, rotation, translation) = gt.to_scale_rotation_translation();
    Isometry::from_parts(Translation::from(translation), rotation.into()).into()
}
