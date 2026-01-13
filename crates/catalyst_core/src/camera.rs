use flecs_ecs::macros::Component;
use glam::Mat4;

#[derive(Component, Clone, Debug)]
pub struct Camera {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 45.0f32.to_radians(),
            aspect_ratio: 16.0 / 9.0, // Standard monitor
            near: 0.1,
            far: 100.0,
        }
    }
}

impl Camera {
    /// Computes the "Projection Matrix" (World -> Screen)
    pub fn compute_projection_matrix(&self) -> Mat4 {
        // Perspective projection (things get smaller as they move away)
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.near, self.far)
    }
}
