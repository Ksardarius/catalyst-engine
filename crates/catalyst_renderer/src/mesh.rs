use std::mem;

use bytemuck::{Pod, Zeroable};
use catalyst_assets::assets::{Handle, MeshData};
use catalyst_core::{
    Changed, Commands, Component, Entity, Query, Res, ResMut, With, Without,
    transform::GlobalTransform,
};
use wgpu::util::DeviceExt;

use crate::RenderContext;

// #[repr(C)] ensures the compiler doesn't reorder fields.
// Pod (Plain Old Data) and Zeroable allow us to cast this struct to raw bytes safely.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct MeshUniform {
    // 1. The Model Matrix (4x4 floats)
    // Moves the object from (0,0,0) to its place in the world.
    pub model: [[f32; 4]; 4],

    // 2. The Normal Matrix (4x4 floats)
    // Used for lighting. It handles weird scaling issues.
    // (Technically 3x3 is enough, but GPUs prefer 4x4 alignment).
    pub normal_matrix: [[f32; 4]; 4],
}

impl MeshUniform {
    // Helper to calculate this from your ECS component
    pub fn from_transform(global: &GlobalTransform) -> Self {
        let model_matrix = global.0; // The Mat4 you calculated in PostUpdate
        
        // Lighting math: Transpose(Inverse(Model))
        // If you squash a sphere, the normals shouldn't squash; they should stretch.
        // This matrix fixes that.
        let normal_matrix = model_matrix.inverse().transpose();

        Self {
            model: model_matrix.to_cols_array_2d(),
            normal_matrix: normal_matrix.to_cols_array_2d(),
        }
    }
}

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

#[derive(Component)]
pub struct GpuMesh {
    pub bind_group: wgpu::BindGroup, // Passed to render_pass.set_bind_group(2, ...)
    pub buffer: wgpu::Buffer,        // Passed to queue.write_buffer(...)
}

pub fn prepare_mesh_transforms(
    mut commands: Commands,
    mut context: ResMut<RenderContext>,

    // QUERY A: "The Newcomers"
    // Find entities that have a mesh and position, but NO GPU data yet.
    new_entities: Query<(Entity, &GlobalTransform), (With<Mesh>, Without<GpuMesh>)>,

    // QUERY B: "The Movers"
    // Find entities that already have GPU data, but moved this frame.
    mut moving_entities: Query<(&GlobalTransform, &GpuMesh), Changed<GlobalTransform>>,
) {
    for (entity, global_transform) in new_entities.iter() {
        // 1. Calculate Matrices
        // We take the Position/Rotation/Scale from the ECS and turn it into
        // the 4x4 matrix the shader expects.
        let uniform = MeshUniform::from_transform(global_transform);

        // 2. Allocate VRAM (Expensive!)
        // We ask the GPU to reserve 128 bytes of memory for this specific object.
        let buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh Uniform Buffer"),
                    contents: bytemuck::cast_slice(&[uniform]),
                    // COPY_DST is crucial: it allows us to update this buffer later!
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        // 3. Create the Bind Group ( The "Signpost")
        // We create a handle that tells the shader: "When you ask for Group 2, look at THIS buffer."
        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Mesh Bind Group"),
                layout: &context.mesh_bind_group_layout, // Defined in Renderer::new()
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

        // 4. Save the Handles
        // We attach these handles back to the Entity so we can find them next frame.
        commands
            .entity(entity)
            .insert(GpuMesh { bind_group, buffer });
    }

    // 'Changed<GlobalTransform>' is the magic filter.
    // If you have 10,000 static walls, this loop runs 0 times.
    for (global_transform, gpu_mesh) in moving_entities.iter() {
        
        // 1. Recalculate Matrices
        let uniform = MeshUniform::from_transform(global_transform);
        
        // 2. Upload Data (Cheap!)
        // We don't allocate memory. We just copy 128 bytes over the PCIe bus
        // into the existing buffer we created in Phase 1.
        context.queue.write_buffer(
            &gpu_mesh.buffer, // The buffer stored in the GpuMesh component
            0,                // Offset 0 (Overwrite from start)
            bytemuck::cast_slice(&[uniform])
        );
    }
}
