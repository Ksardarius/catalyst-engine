use catalyst_core::transform::Transform;

use crate::{assets::{Handle, MeshData}, material::{MaterialData, TextureData}};

#[derive(Clone, Debug)]
pub struct SceneData {
    pub meshes: Vec<Handle<MeshData>>,
    pub materials: Vec<Handle<MaterialData>>,
    pub textures: Vec<Handle<TextureData>>,
    
    // The Nodes (Entities)
    pub nodes: Vec<SceneNode>, 
}

#[derive(Clone, Debug)]
pub struct SceneNode {
    pub name: String,
    pub transform: Transform,
    pub mesh_index: Option<usize>, // Index into the meshes list above
    pub material_index: Option<usize>,
    pub children: Vec<usize>,
}

