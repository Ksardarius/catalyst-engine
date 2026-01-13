use flecs_ecs::prelude::*;
use catalyst_core::{camera::Camera, transform::Transform};

use crate::{assets::{Handle, MeshData}, material::{MaterialData, TextureData}};

#[derive(Component, Clone, Debug)]
pub struct SceneData {
    pub meshes: Vec<Handle<MeshData>>,
    pub materials: Vec<Handle<MaterialData>>,
    pub textures: Vec<Handle<TextureData>>,
    
    // The Nodes (Entities)
    pub nodes: Vec<SceneNode>, 
    pub camera: Vec<Camera>
}

#[derive(Clone, Debug)]
pub struct SceneNode {
    pub name: String,
    pub transform: Transform,
    pub mesh_index: Option<usize>, // Index into the meshes list above
    pub material_index: Option<usize>,
    pub camera_index: Option<usize>,
    pub children: Vec<usize>,
}

