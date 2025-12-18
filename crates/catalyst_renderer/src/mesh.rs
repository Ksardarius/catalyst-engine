use bytemuck::{Pod, Zeroable};
use catalyst_core::{Component, Handle};

// 1. The GPU-Compatible Vertex
// #[repr(C)] ensures C-like memory layout (needed for graphics drivers)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3], // X, Y, Z
    pub color: [f32; 3],    // R, G, B
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0, // @location(0) in shader
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1, // @location(1) in shader
                    format: wgpu::VertexFormat::Float32x3, // color
                },
            ],
        }
    }
}

// 1. Rename the old Mesh struct to "MeshData" (The heavy part)
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>
}

// 2. The Component is now just a Handle!
#[derive(Component, Clone)]
pub struct Mesh(pub Handle<MeshData>);