#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PhysicsBody {
    Static,
    Dynamic,
    Kinematic,
    #[serde(other)]
    Unknown,
}

impl Into<catalyst_core::physics::PhysicsBody> for PhysicsBody {
    fn into(self) -> catalyst_core::physics::PhysicsBody {
        match self {
            PhysicsBody::Static => catalyst_core::physics::PhysicsBody::Static,
            PhysicsBody::Dynamic => catalyst_core::physics::PhysicsBody::Dynamic,
            PhysicsBody::Kinematic => catalyst_core::physics::PhysicsBody::Kinematic,
            PhysicsBody::Unknown => catalyst_core::physics::PhysicsBody::Unknown,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PhysicsShape {
    Box,
    Sphere,
    Capsule,
    Convex,
    Mesh,
    #[serde(other)]
    Unknown,
}

#[derive(serde::Deserialize, Clone, Debug)]
pub struct PhysicsExtras {
    pub physics_body: Option<PhysicsBody>,
    pub physics_shape: Option<PhysicsShape>,
    pub physics_layer: Option<u32>,
    pub physics_mask: Option<u32>,
    pub physics_is_trigger: Option<bool>,
    pub physics_mass: Option<f32>,
    pub physics_gravity_scale: Option<f32>,
    pub physics_linear_damping: Option<f32>,
    pub physics_angular_damping: Option<f32>,
    pub physics_material: Option<String>,
}
