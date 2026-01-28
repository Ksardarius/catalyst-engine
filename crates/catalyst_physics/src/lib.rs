use catalyst_core::Plugin;
use flecs_ecs::prelude::*;
use rapier3d::prelude::*;

use crate::{prepare::prepare_physics_system, step::step_physics_system, sync::sync_physics_system};

pub mod prepare;
mod step;
mod sync;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut catalyst_core::App) {
        app.register_singleton_default::<PhysicsWorld>();

        prepare_physics_system(&app);
        step_physics_system(&app);
        sync_physics_system(&app);
    }
}

#[derive(Component)]
pub struct PhysicsBodyAdded;

#[derive(Component)]
pub struct PhysicsColliderAdded;

#[derive(Component)]
pub struct PhysicsWorld {
    pub pipeline: PhysicsPipeline,
    pub gravity: glam::Vec3,
    pub integration_parameters: IntegrationParameters,

    pub islands: IslandManager,
    pub broad_phase: BroadPhaseBvh,
    pub narrow_phase: NarrowPhase,
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
}

impl PhysicsWorld {
    pub fn step(&mut self) {
        self.pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        let bodies = RigidBodySet::new();
        let colliders = ColliderSet::new();
        let impulse_joints = ImpulseJointSet::new();
        let multibody_joints = MultibodyJointSet::new();
        let islands = IslandManager::new();
        let broad_phase = BroadPhaseBvh::new();
        let narrow_phase = NarrowPhase::new();
        let ccd_solver = CCDSolver::new();
        let integration_parameters = IntegrationParameters::default();

        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vec3::from_array([0.0, -9.81, 0.0]),
            integration_parameters,
            islands,
            broad_phase,
            narrow_phase,
            bodies,
            colliders,
            impulse_joints,
            multibody_joints,
            ccd_solver,
        }
    }
}
