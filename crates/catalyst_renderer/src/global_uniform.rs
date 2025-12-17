use bytemuck::{Pod, Zeroable};
use glam::Mat4;

// The data we send to the GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlobalUniform {
    pub transform_matrix: Mat4,
}

