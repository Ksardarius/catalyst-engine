use catalyst_core::{pipeline::PhysicsSync, transform::Transform};
use flecs_ecs::prelude::*;
use rapier3d::prelude::*;

use crate::{PhysicsBodyAdded, PhysicsWorld, prepare::PhysicsHandle};

pub fn sync_physics_system(app: &catalyst_core::App) {
    app.world
        .system_named::<(&mut Transform, &PhysicsHandle, &PhysicsWorld)>("physics_synchronization")
        .kind(PhysicsSync)
        .with(PhysicsBodyAdded)
        .each(|(transform, handle, physics)| {
            if let Some(body_handle) = handle.body {
                if let Some(body) = physics.bodies.get(body_handle) {
                    let iso = body.position();
                    transform.translation = iso.translation;
                    transform.rotation = iso.rotation.into();
                }
            }
        });
}
