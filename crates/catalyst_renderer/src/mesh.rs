use std::mem;

use bytemuck::{Pod, Zeroable};
use catalyst_assets::assets::{Handle, MeshData};
use catalyst_core::{Component};

// 1. The GPU-Compatible Vertex
// #[repr(C)] ensures C-like memory layout (needed for graphics drivers)
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3], // X, Y, Z
    pub normal: [f32; 3],
    pub uv: [f32; 2],
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
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// // 1. Rename the old Mesh struct to "MeshData" (The heavy part)
// pub struct MeshData {
//     pub vertices: Vec<Vertex>,
//     pub indices: Vec<u32>
// }

// 2. The Component is now just a Handle!
#[derive(Component, Clone)]
pub struct Mesh(pub Handle<MeshData>);