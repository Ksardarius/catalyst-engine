use catalyst_core::Plugin;
use flecs_ecs::prelude::*;
use rapier3d::prelude::*;

use crate::prepare::prepare_physics_system;

mod prepare;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut catalyst_core::App) {
        app.register_singleton_default::<PhysicsWorld>();

        prepare_physics_system(&app);
    }
}

#[derive(Component)]
pub struct PhysicsWorld {
    pub pipeline: PhysicsPipeline,
    pub gravity: Vec<Real>,
    pub integration_params: IntegrationParameters,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: vec![0.0, -9.81, 0.0],
            integration_params: IntegrationParameters::default(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
        }
    }
}
