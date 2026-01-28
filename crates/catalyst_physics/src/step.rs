use catalyst_core::pipeline::PhysicsStep;
use flecs_ecs::prelude::*;

use crate::PhysicsWorld;

pub fn step_physics_system(app: &catalyst_core::App) {
    app.world
        .system_named::<&mut PhysicsWorld>("physics_evaluation")
        .kind(PhysicsStep)
        .run(|mut iter| {
            while iter.next() {
                let mut physics_comp = iter.field_mut::<PhysicsWorld>(0);
                if let Some(physics) = physics_comp.get_mut(0) {
                    physics.step();
                }
            }
        });
}
