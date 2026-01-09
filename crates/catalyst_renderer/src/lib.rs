use catalyst_core::{App, Plugin};

use crate::{
    material::register_material_handlers, mesh::register_mesh_handlers, render::register_renderings, texture::register_texture_handlers,
};

mod camera;
mod light;
mod material;
pub mod mesh;
mod render;
mod texture;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        register_renderings(&app.world);
        register_mesh_handlers(&app.world);
        register_material_handlers(&app.world);
        register_texture_handlers(&app.world);
    }
}

// pub fn prepare_lights(
//     // We need the RenderContext to access the Buffer and Queue
//     context: &mut RenderContext,

//     // Queries to find lights in the scene
//     sun_query: Query<(&GlobalTransform, &DirectionalLight)>,
//     point_query: Query<(&GlobalTransform, &PointLight)>,

//     // We need Camera pos for specular calculations
//     cam_query: Query<&GlobalTransform, With<Camera>>,
// ) {
//     let mut uniforms = LightUniforms {
//         sun_direction: [0.0, -1.0, 0.0, 1.0], // Default Down
//         sun_color: [1.0, 1.0, 1.0, 0.0],
//         point_lights: [GpuPointLight {
//             position: [0.0; 4],
//             color: [0.0; 4],
//         }; 4],
//         camera_pos: [0.0, 0.0, 0.0],
//         active_lights: 0,
//     };

//     // A. Process Sun (Take the first one we find)
//     if let Some((transform, sun)) = sun_query.iter().next() {
//         // Calculate forward vector from rotation
//         let forward = transform.forward(); // Bevy GlobalTransform helper
//         uniforms.sun_direction = [forward.x, forward.y, forward.z, sun.intensity];
//         uniforms.sun_color = [sun.color[0], sun.color[1], sun.color[2], 0.0];
//     }

//     // B. Process Point Lights
//     let mut count = 0;
//     for (transform, light) in point_query.iter().take(4) {
//         // Limit to 4!
//         let pos = transform.translation();
//         uniforms.point_lights[count] = GpuPointLight {
//             position: [pos.x, pos.y, pos.z, light.intensity],
//             color: [light.color[0], light.color[1], light.color[2], light.radius],
//         };
//         count += 1;
//     }
//     uniforms.active_lights = count as u32;

//     // C. Process Camera Position
//     if let Some(cam_tf) = cam_query.iter().next() {
//         let pos = cam_tf.translation();
//         uniforms.camera_pos = [pos.x, pos.y, pos.z];
//     }

//     // D. Upload to GPU
//     // "scene_data_buffer" is the buffer you created in Binding 1 of Group 0
//     context.queue.write_buffer(
//         &context.scene_data_buffer,
//         0,
//         bytemuck::cast_slice(&[uniforms]),
//     );
// }
