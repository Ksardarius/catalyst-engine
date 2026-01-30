use flecs_ecs::prelude::*;
use glam::{Mat4, Quat, Vec3};

#[derive(Component)]
#[flecs(meta)]
pub struct ReflectVec3 {
    x: f32,
    z: f32,
    y: f32,
}

#[derive(Component)]
#[flecs(meta)]
pub struct ReflectVec4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Component)]
#[flecs(meta)]
pub struct ReflectQuat { x: f32, y: f32, z: f32, w: f32 }

#[derive(Component, Clone, Copy, Debug)]
pub struct Transform {
    pub translation: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
}

// Optimization: We assume identity default to save branches
impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vec3::new(x, y, z),
            ..Default::default()
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Default::default()
        }
    }

    /// Rotates the object around the Y axis (Global Up)
    pub fn rotate_y(&mut self, angle_radians: f32) {
        let rotation = Quat::from_rotation_y(angle_radians);
        self.rotation = self.rotation * rotation;
    }

    /// Rotates around the X axis (Local Right)
    pub fn rotate_local_x(&mut self, angle_radians: f32) {
        let rotation = Quat::from_rotation_x(angle_radians);
        self.rotation = self.rotation * rotation;
    }

    /// Makes the transform look at a target position
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Self {
        let forward = (target - self.translation).normalize();
        // Custom look_at logic for object rotation
        // Or simplified: use Mat4 to extract Quat
        let mat = Mat4::look_at_rh(self.translation, target, up);
        // Invert because look_at moves the world, but we want to move the object
        self.rotation = Quat::from_mat4(&mat.inverse());
        self
    }

    // --- Matrices ---

    /// Creates the Model Matrix (Local -> World)
    /// This is what we send to the GPU Uniform Buffer
    pub fn compute_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    // --- Directions (Useful for Movement) ---

    /// Returns the "Forward" direction (-Z) relative to current rotation
    pub fn forward(&self) -> Vec3 {
        self.rotation * -Vec3::Z
    }

    /// Returns the "Right" direction (+X) relative to current rotation
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Returns the "Up" direction (+Y) relative to current rotation
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GlobalTransform(pub Mat4);

impl GlobalTransform {
    /// Extracts the scale, rotation, and translation from the transformation matrix.
    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        // We access the inner Mat4 using .0
        self.0.to_scale_rotation_translation()
    }
}

pub fn transform_propagation_system(world: &World) {
    world
        .system_named::<(&Transform, &mut GlobalTransform, Option<&GlobalTransform>)>("transform_propagation_system")
        .kind(flecs::pipeline::PostUpdate)
        .cascade()
        .term_at(2)
        .parent()
        .each(|(local, global, parent_global)| {
            if let Some(parent_global) = parent_global {
                global.0 = parent_global.0 * local.compute_matrix();
            } else {
                global.0 = local.compute_matrix();
            }
        });
}
