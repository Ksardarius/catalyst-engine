use catalyst_assets::{MaterialDefinition, MeshDefinition, assets::{Assets, Handle}, scene::SceneData};
use catalyst_core::{App, Children, Commands, Component, Entity, Plugin, Query, Res, Without};

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // Register the component so the ECS knows about it
        // (Optional in some Bevy versions, but good practice for reflection)
        // app.world.register_type::<SceneRoot>(); 

        // Register the system that makes it work
        app.add_system(spawn_scenes);
    }
}

#[derive(Component)]
pub struct SceneRoot(pub Handle<SceneData>);

pub fn spawn_scenes(
    mut commands: Commands,
    scene_roots: Query<(Entity, &SceneRoot), Without<Children>>, 
    scenes: Res<Assets<SceneData>>,
) {
    for (root_entity, root) in &scene_roots {
        if let Some(scene_data) = scenes.get(&root.0) {
            println!("Asset arrived! Spawning nodes now...");
            let mut node_entities = Vec::with_capacity(scene_data.nodes.len());

            for node in &scene_data.nodes {
                // 1. Base Transform
                let mut entity_cmd = commands.spawn(node.transform);

                // 2. Attach Generic Definitions
                if let Some(mesh_idx) = node.mesh_index {
                    let mesh_handle = scene_data.meshes[mesh_idx].clone();
                    entity_cmd.insert(MeshDefinition(mesh_handle));
                    
                    if let Some(mat_idx) = node.material_index {
                        let mat_handle = scene_data.materials[mat_idx].clone();
                        entity_cmd.insert(MaterialDefinition(mat_handle));
                    }
                }

                let entity = entity_cmd.id();
                node_entities.push(entity);

                // Default: Attach everything to the SceneRoot initially.
                // We will move the children to their real parents in Pass 2.
                commands.entity(root_entity).add_child(entity);
            }

            for (i, node) in scene_data.nodes.iter().enumerate() {
                // The entity we just spawned for this node
                let parent_entity = node_entities[i];

                // Loop through the children indices stored in the GLTF data
                for &child_index in &node.children {
                    let child_entity = node_entities[child_index];

                    // CRITICAL: Re-parenting
                    // This command removes 'child_entity' from 'root_entity' 
                    // and adds it to 'parent_entity'.
                    commands.entity(parent_entity).add_child(child_entity);
                }
            }
        }
    }
}