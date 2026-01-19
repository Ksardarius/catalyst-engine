use catalyst_assets::{
    MaterialDefinition, MeshDefinition, assets::Handle, scene::SceneData,
};
use catalyst_core::{App, Plugin, Source, transform::GlobalTransform};
use flecs_ecs::prelude::*;

pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        // Register the component so the ECS knows about it
        // (Optional in some Bevy versions, but good practice for reflection)
        // app.world.register_type::<SceneRoot>();

        // Register the system that makes it work
        register_spawn_scenes(&app.world);
    }
}

#[derive(Component)]
pub struct SceneRoot(pub Handle<SceneData>);

pub fn register_spawn_scenes(world: &World) {
    world
        .system_named::<&SceneRoot>("Spawn Scenes")
        .kind(flecs::pipeline::OnUpdate)
        .each_entity(|root_entity, root| {
            let world = root_entity.world();

            if let Some(entity) = root.0.try_get_entity(&world) {
                entity.try_get::<&SceneData>(|scene_data| {
                    println!("Asset arrived! Spawning nodes now...");
                    root_entity.remove(SceneRoot::id()).add((Source, entity));

                    let mut node_entities = Vec::with_capacity(scene_data.nodes.len());

                    for node in &scene_data.nodes {
                        let entity_cmd = world
                            .entity()
                            // .set_name(&node.name)
                            .child_of(root_entity)
                            .set(node.transform)
                            .set(GlobalTransform::default());

                        // 2. Attach Generic Definitions
                        if let Some(mesh_idx) = node.mesh_index {
                            if let Some(mesh) = scene_data.meshes.get(mesh_idx) {
                                entity_cmd.set(MeshDefinition(mesh.clone()));
                            }

                            if let Some(mat_idx) = node.material_index {
                                if let Some(mat) = scene_data.materials.get(mat_idx) {
                                    entity_cmd.set(MaterialDefinition(mat.clone()));
                                }
                            }
                        }

                        if let Some(camera_idx) = node.camera_index {
                            if let Some(camera) = scene_data.camera.get(camera_idx) {
                                entity_cmd.set(camera.clone());
                            }
                        }

                        node_entities.push(entity_cmd);
                    }

                    for (i, node) in scene_data.nodes.iter().enumerate() {
                        // The entity we just spawned for this node
                        let parent_entity = node_entities[i];

                        // Loop through the children indices stored in the GLTF data
                        for &child_index in &node.children {
                            let child_entity = node_entities[child_index];
                            child_entity.child_of(parent_entity);
                        }
                    }
                });
            }
        });
}
