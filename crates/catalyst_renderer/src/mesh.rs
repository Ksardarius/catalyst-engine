use flecs_ecs::{core::flecs::Wildcard, prelude::*};
use std::mem;

use bytemuck::{Pod, Zeroable};
use catalyst_assets::{
    MeshDefinition,
    assets::{Handle, MeshData},
};
use catalyst_core::transform::GlobalTransform;
use wgpu::util::DeviceExt;

use crate::render::RenderContext;

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

// 2. The Component is now just a Handle!
#[derive(Component, Clone)]
pub struct AssetMesh;

#[derive(Component)]
pub struct MeshInstance {
    pub bind_group: wgpu::BindGroup, // Passed to render_pass.set_bind_group(2, ...)
    pub buffer: wgpu::Buffer,        // Passed to queue.write_buffer(...)
}

#[derive(Component)]
pub struct GpuGeometry {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

pub fn register_mesh_handlers(world: &World) {
    world
        .system_named::<(&MeshData, &mut RenderContext)>("Init Mesh GPU buffers")
        .without(GpuGeometry::id())
        .kind(flecs::pipeline::OnStore)
        .each_entity(|entity, (mesh_data, context)| {
            let (v_buf, i_buf, count) = create_gpu_buffer(&context.device, mesh_data);

            entity.set(GpuGeometry {
                vertex_buffer: v_buf,
                index_buffer: i_buf,
                index_count: count,
            });
        });

    world
        .system_named::<&MeshDefinition>("Link Mesh Definition to AssetMesh")
        .without((AssetMesh, Wildcard))
        .each_entity(|entity, definition| {
            let world = entity.world();
            if let Some(mesh_entity) = definition.0.try_get_entity(&world) {
                entity.add((AssetMesh, mesh_entity));
            }
        });

    // Find entities that have a mesh and position, but NO GPU data yet.
    world
        .system_named::<(&GlobalTransform, &mut RenderContext)>("Setup Meshes in GPU")
        .with((AssetMesh, Wildcard))
        .without(MeshInstance::id())
        .kind(flecs::pipeline::OnUpdate)
        .each_entity(|entity, (global_transform, context)| {
            // 1. Calculate Matrices
            // We take the Position/Rotation/Scale from the ECS and turn it into
            // the 4x4 matrix the shader expects.
            let uniform = MeshUniform::from_transform(global_transform);

            // 2. Allocate VRAM (Expensive!)
            // We ask the GPU to reserve 128 bytes of memory for this specific object.
            let buffer = context
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

            entity.set(MeshInstance { bind_group, buffer });
        });

    world
        .system_named::<(&GlobalTransform, &MeshInstance, &mut RenderContext)>(
            "Setup Mesh in GPU on change",
        )
        .kind(flecs::pipeline::PostUpdate)
        .detect_changes()
        .each(|(global_transform, gpu_mesh, context)| {
            let uniform = MeshUniform::from_transform(global_transform);

            // 2. Upload Data (Cheap!)
            // We don't allocate memory. We just copy 128 bytes over the PCIe bus
            // into the existing buffer we created in Phase 1.
            context.queue.write_buffer(
                &gpu_mesh.buffer, // The buffer stored in the GpuMesh component
                0,                // Offset 0 (Overwrite from start)
                bytemuck::cast_slice(&[uniform]),
            );
        });
}

fn create_gpu_buffer(device: &wgpu::Device, data: &MeshData) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    // 1. Interleave Data (SoA -> AoS)
    // We combine pos, normal, uv into a single 'Vertex' struct list
    let vertex_count = data.vertices.len();
    let mut vertices = Vec::with_capacity(vertex_count);

    for i in 0..vertex_count {
        vertices.push(Vertex {
            position: data.vertices[i].position,
            normal: data.vertices[i].normal,
            uv: data.vertices[i].uv,
        });
    }

    use wgpu::util::DeviceExt;

    let v_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let i_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Mesh Index Buffer"),
        contents: bytemuck::cast_slice(&data.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    (v_buffer, i_buffer, data.indices.len() as u32)
}
