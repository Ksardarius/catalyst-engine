use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4], // View-Projection matrix
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    pub sun_direction: [f32; 4],          // .w = intensity
    pub sun_color: [f32; 4],              // .w = padding
    pub point_lights: [GpuPointLight; 4], // Fixed array of 4
    pub camera_pos: [f32; 3],
    pub active_lights: u32, // Count
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuPointLight {
    pub position: [f32; 4], // .w = intensity
    pub color: [f32; 4],    // .w = radius (unused in shader currently but good for padding)
}

pub struct GlobalResources {
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    cam_buffer: wgpu::Buffer,
    lights_buffer: wgpu::Buffer,
}

impl GlobalResources {
    pub fn new(device: &wgpu::Device) -> Self {
        let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                // --- BINDING 0: MVP Matrix ---
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // --- BINDING 1: Light Uniforms ---
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let initial_camera_data = CameraUniform {
            view_proj: [[0.0; 4]; 4], // Placeholder
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[initial_camera_data]),
            // USAGE: COPY_DST allows us to write to it later!
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let initial_light_data = LightUniforms {
            sun_direction: [0.0, -1.0, 0.0, 1.0],
            sun_color: [1.0, 1.0, 1.0, 0.0],
            point_lights: [GpuPointLight {
                position: [0.0; 4],
                color: [0.0; 4],
            }; 4],
            camera_pos: [0.0, 0.0, 0.0],
            active_lights: 4,
        };

        let scene_data_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Data Buffer (Lights)"),
            contents: bytemuck::cast_slice(&[initial_light_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, // COPY_DST is critical for updates!
        });

        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Global Bind Group"),
            layout: &global_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: scene_data_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            layout: global_layout,
            bind_group: global_bind_group,
            cam_buffer: camera_buffer,
            lights_buffer: scene_data_buffer,
        }
    }

    // This is the key method you were missing!
    pub fn update_camera(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        queue.write_buffer(
            &self.cam_buffer,                                      // Target
            0,                                                     // Offset
            bytemuck::cast_slice(&[view_proj.to_cols_array_2d()]), // Data
        );
    }

    pub fn update_lights(
        &self,
        queue: &wgpu::Queue,
        uniform: LightUniforms
    ) {
        queue.write_buffer(&self.lights_buffer, 0, bytemuck::bytes_of(&uniform));
    }
}
