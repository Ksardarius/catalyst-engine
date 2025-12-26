use bevy_ecs::component::Component;
use glam::{Vec3, Quat, Mat4};

#[derive(Component, Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
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
        // We look from 'position' to 'target'
        // Note: Mat4::look_at_rh creates a View Matrix (inverse transform).
        // For an object Transform, we want the rotation that points -Z towards target.
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
        Mat4::from_scale_rotation_translation(
            self.scale,
            self.rotation,
            self.translation,
        )
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
