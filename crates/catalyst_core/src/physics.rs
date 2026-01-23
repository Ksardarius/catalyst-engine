use flecs_ecs::macros::Component;

use crate::transform::Transform;

#[derive(Debug, Clone, Copy)]
pub enum PhysicsBody {
    Static,
    Dynamic,
    Kinematic,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum ColliderShape {
    Box { hx: f32, hy: f32, hz: f32 },
    Sphere { radius: f32 },
    Capsule { radius: f32, height: f32 },
    Convex { vertices: Vec<glam::Vec3> },
    Mesh { vertices: Vec<glam::Vec3>, indices: Vec<u32> },
}

#[derive(Component, Debug, Clone)]
pub struct RigidBodyDefinition {
    pub body_type: PhysicsBody,
    pub mass: Option<f32>,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

#[derive(Component, Debug, Clone)]
pub struct ColliderDefinition {
    pub shape: ColliderShape,
    pub is_trigger: bool,
    pub offset: Transform,
    pub layer: u32,
    pub mask: u32,
}

#[derive(Component, Debug, Clone)]
pub struct PhysicsMaterialDefinition {
    pub friction: f32,
    pub restitution: f32,
}

#[derive(Debug, Clone)]
pub struct CollisionFilterDefinition {
    pub layer: u32,
    pub mask: u32,
}
