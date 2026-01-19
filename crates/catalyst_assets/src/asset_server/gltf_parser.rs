use std::{any::Any, path::Path};

use catalyst_core::{
    camera::{self, Camera},
    transform::Transform,
};
use glam::Quat;
use gltf::image::Format;

use crate::{
    assets::{Handle, MeshData, Vertex},
    material::{MaterialData, MaterialSettings, TextureData, TextureFormat},
    scene::SceneData,
};

type GltfPayload = (
    SceneData,
    Vec<(Handle<TextureData>, TextureData)>,
    Vec<(Handle<MaterialData>, MaterialData)>,
    Vec<(Handle<MeshData>, MeshData)>,
);

pub fn parse_gltf(path: &str) -> Result<GltfPayload, String> {
    let base_path = Path::new(path).parent().unwrap_or(Path::new("./"));

    // A. Load Document & Buffers
    let (document, buffers, images) = gltf::import(path).map_err(|e| e.to_string())?;

    // --- STEP 1: TEXTURES ---
    let mut texture_artifacts = Vec::new();
    let mut texture_map = Vec::new(); // Maps GLTF Image Index -> Our Handle

    for image in document.images() {
        match image.source() {
            gltf::image::Source::View { view, .. } => {
                let buffer = &buffers[view.buffer().index()];
                
                let name = image.name().unwrap_or("GLTF Image");
                
                let start = view.offset();
                let end = start + view.length();
                let image_data = &buffer[start..end];

                // Decode image data using the `image` crate
                let img = image::load_from_memory(image_data)
                    .map_err(|e| format!("Failed to decode image: {}", e))?;
                

                let img = img.to_rgba8();

                let width = img.width();
                let height = img.height();
                let pixels = img.into_raw(); // Get raw pixel data

                // Create our TextureData
                let image = TextureData {
                    name: name.to_string(),
                    width,
                    height,
                    pixels,
                    format: TextureFormat::Rgba8Unorm,
                };

                // Store the texture data
                let handle = Handle::<TextureData>::new();
                texture_artifacts.push((handle.clone(), image));
                texture_map.push(handle);
            }
            gltf::image::Source::Uri { .. } => {}
        }

        // let converted_pixels = match image.format {
        //     // CASE A: It's already RGBA (Good!)
        //     Format::R8G8B8A8 => {
        //         image.pixels // No work needed, just take the bytes
        //     }

        //     // CASE B: It's RGB (The source of your crash)
        //     // We must convert 3 bytes -> 4 bytes manually
        //     Format::R8G8B8 => {
        //         let pixel_count = image.width * image.height;
        //         let mut rgba_data = Vec::with_capacity((pixel_count * 4) as usize);

        //         // Iterate over chunks of 3 bytes (R, G, B)
        //         for chunk in image.pixels.chunks_exact(3) {
        //             rgba_data.extend_from_slice(chunk); // Copy R, G, B
        //             rgba_data.push(255); // Add A (Full Opacity)
        //         }
        //         rgba_data // Return the new vector
        //     }

        //     // Handle other formats (R8, R16, etc.) if necessary
        //     _ => panic!("Unsupported texture format: {:?}", image.format),
        // };

        // // Convert GLTF Image to our format
        // // Note: gltf::import automatically decodes PNG/JPG bytes into pixels for us!
        // let tex_data = TextureData {
        //     width: image.width,
        //     height: image.height,
        //     pixels: converted_pixels,          // Raw RGBA bytes
        //     format: TextureFormat::Rgba8Unorm, // GLTF is almost always RGBA8
        // };

        // let handle = Handle::<TextureData>::new();
        // texture_artifacts.push((handle.clone(), tex_data));
        // texture_map.push(handle);
    }

    // --- STEP 2: MATERIALS ---
    let mut material_artifacts = Vec::new();
    let mut material_map = Vec::new();

    for mat in document.materials() {
        let pbr = mat.pbr_metallic_roughness();

        // 1. Resolve Texture Handle
        let diffuse_handle = pbr.base_color_texture().map(|info| {
            let idx = info.texture().source().index();
            texture_artifacts[idx].1.format = TextureFormat::Rgba8UnormSrgb;
            texture_map[idx].clone() // <--- The Link!
        });

        let roughness_handle = pbr.metallic_roughness_texture().map(|info| {
            let idx = info.texture().source().index();
            texture_artifacts[idx].1.format = TextureFormat::Rgba8Unorm;
            texture_map[idx].clone() // <--- The Link!
        });

        let normal_handle = mat.normal_texture().map(|info| {
            let idx = info.texture().source().index();
            texture_artifacts[idx].1.format = TextureFormat::Rgba8Unorm;
            texture_map[idx].clone() // <--- The Link!
        });

        let occlusion_handle = mat.occlusion_texture().map(|info| {
            let idx = info.texture().source().index();
            texture_artifacts[idx].1.format = TextureFormat::Rgba8Unorm;
            texture_map[idx].clone() // <--- The Link!
        });

        // 2. Build Material Data
        let mat_data = MaterialData {
            settings: MaterialSettings {
                base_color: pbr.base_color_factor(),
                roughness: pbr.roughness_factor(),
                metallic: pbr.metallic_factor(),
            },
            diffuse_texture: diffuse_handle,
            // For now, we skip Normal/Metallic maps to keep it simple.
            // You can add them later using the same pattern.
            normal_texture: normal_handle,
            metallic_roughness_texture: roughness_handle,
            occlusion_texture: occlusion_handle,
        };

        let handle = Handle::<MaterialData>::new();
        material_artifacts.push((handle.clone(), mat_data));
        material_map.push(handle);
    }

    // --- STEP 3: MESHES ---
    let mut mesh_artifacts = Vec::new();
    let mut mesh_map = Vec::new(); // Maps GLTF Mesh Index -> Our Handle

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            // Extract Positions
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(|iter| iter.collect())
                .ok_or("Mesh missing positions")?;

            // Extract Normals (or generate default up-vector)
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            // Extract UVs (or 0.0)
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|read| read.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            // Extract Indices
            let indices: Vec<u32> = reader
                .read_indices()
                .map(|read| read.into_u32().collect())
                .ok_or("Mesh missing indices")?;

            // Interleave vertices (Position + Normal + UV)
            let mut vertices = Vec::new();
            for i in 0..positions.len() {
                vertices.push(Vertex {
                    position: positions[i],
                    normal: normals[i],
                    uv: uvs[i],
                });
            }

            let mesh_data = MeshData { vertices, indices };
            let handle = Handle::<MeshData>::new();

            mesh_artifacts.push((handle.clone(), mesh_data));

            // Note: GLTF Meshes can have multiple "Primitives".
            // We are simplifying and assuming 1 primitive per mesh for this tutorial.
            // A robust engine would split these into multiple sub-meshes.
            mesh_map.push(handle);
        }
    }

    // --- STEP 4: NODES (The Hierarchy) ---
    let mut scene_nodes = Vec::new();

    for node in document.nodes() {
        // Position/Rotation/Scale
        let (t, r, s) = node.transform().decomposed();

        let transform = Transform {
            translation: t.into(),
            rotation: Quat::from_array(r),
            scale: s.into(),
        };

        // Link to Mesh
        let mesh_index = node.mesh().map(|m| m.index());

        // Link to Material
        // In GLTF, materials are assigned to Mesh Primitives, not Nodes directly.
        // We look up the material used by the mesh's first primitive.
        let material_index = node
            .mesh()
            .and_then(|m| m.primitives().next())
            .and_then(|p| p.material().index());

        let camera_index = node.camera().map(|cam|cam.index());

        scene_nodes.push(crate::scene::SceneNode {
            name: node.name().unwrap_or("Node").to_string(),
            transform,
            mesh_index,
            material_index,
            camera_index,
            children: node.children().map(|c| c.index()).collect(),
        });
    }

    // camera
    let cameras: Vec<_> = document
        .cameras()
        .map(|c| match c.projection() {
            gltf::camera::Projection::Orthographic(orthographic) => Camera::default(),
            gltf::camera::Projection::Perspective(perspective) => Camera {
                fov: perspective.yfov(),
                aspect_ratio: perspective.aspect_ratio().unwrap_or(1f32),
                near: perspective.znear(),
                far: perspective.zfar().unwrap_or(1f32),
            },
        })
        .collect();

    let scene_data = SceneData {
        nodes: scene_nodes,
        textures: texture_map,
        materials: material_map,
        meshes: mesh_map,
        camera: cameras,
    };

    Ok((
        scene_data,
        texture_artifacts,
        material_artifacts,
        mesh_artifacts,
    ))
}
