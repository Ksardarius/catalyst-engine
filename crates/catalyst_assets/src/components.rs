use catalyst_core::Component;

use crate::{assets::{Handle, MeshData}, material::MaterialData};

// "Please render this Mesh"
#[derive(Component, Clone, Debug)]
pub struct MeshDefinition(pub Handle<MeshData>);

// "Please apply this Material"
#[derive(Component, Clone, Debug)]
pub struct MaterialDefinition(pub Handle<MaterialData>);
