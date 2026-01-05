use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use glam::{Mat4, Quat, Vec3};

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

#[derive(Component, Clone, Copy, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct GlobalTransform(pub Mat4);

impl GlobalTransform {
    /// Extracts the scale, rotation, and translation from the transformation matrix.
    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        // We access the inner Mat4 using .0
        self.0.to_scale_rotation_translation()
    }
}

pub fn transform_propagation_system(
    // 1. changed_roots: Roots that moved THIS frame
    // Roots are entities WITHOUT a 'ChildOf' component
    changed_roots: Query<
        (Entity, &Transform, Option<&Children>),
        (Without<ChildOf>, Changed<Transform>), // <--- CHANGED
    >,

    // 2. changed_children: Children who moved relative to parent THIS frame
    changed_local: Query<
        (Entity, &ChildOf, &Transform, Option<&Children>), // <--- CHANGED
        (With<ChildOf>, Changed<Transform>),               // <--- CHANGED
    >,

    // 3. Random access to read/write globals
    mut global_transforms: Query<&mut GlobalTransform>,

    // 4. Hierarchy query
    hierarchy_query: Query<(&Transform, Option<&Children>)>,
) {
    // A. Handle Roots
    for (entity, transform, children) in changed_roots.iter() {
        let matrix = transform.compute_matrix();

        if let Ok(mut global) = global_transforms.get_mut(entity) {
            global.0 = matrix;
        }

        if let Some(children) = children {
            propagate_recursive(matrix, children, &hierarchy_query, &mut global_transforms);
        }
    }

    // B. Handle Children (Local Updates)
    for (entity, parent, transform, children) in changed_local.iter() {
        // 'parent' is now of type '&ChildOf'.
        // We use parent.get() or **parent to get the Entity ID.
        if let Ok(parent_global) = global_transforms.get(parent.parent()) {
            let parent_matrix = parent_global.0;
            let new_matrix = parent_matrix * transform.compute_matrix();

            if let Ok(mut global) = global_transforms.get_mut(entity) {
                global.0 = new_matrix;
            }

            if let Some(children) = children {
                propagate_recursive(
                    new_matrix,
                    children,
                    &hierarchy_query,
                    &mut global_transforms,
                );
            }
        }
    }
}

// Recursive function remains mostly the same
fn propagate_recursive(
    parent_matrix: Mat4,
    children: &Children,
    hierarchy_query: &Query<(&Transform, Option<&Children>)>,
    global_query: &mut Query<&mut GlobalTransform>,
) {
    for &child_entity in children {
        if let Ok((transform, grand_children)) = hierarchy_query.get(child_entity) {
            let new_matrix = parent_matrix * transform.compute_matrix();

            if let Ok(mut global) = global_query.get_mut(child_entity) {
                global.0 = new_matrix;
            }

            if let Some(grand_children) = grand_children {
                propagate_recursive(new_matrix, grand_children, hierarchy_query, global_query);
            }
        }
    }
}
