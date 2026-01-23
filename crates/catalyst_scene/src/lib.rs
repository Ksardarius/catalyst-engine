use catalyst_assets::{
    MaterialDefinition, MeshDefinition,
    assets::Handle,
    physics::PhysicsShape,
    scene::{SceneData, SceneNode},
};
use catalyst_core::{
    App, Plugin, Source,
    physics::{ColliderDefinition, ColliderShape, RigidBodyDefinition},
    transform::{GlobalTransform, Transform},
};
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

                        if let Some(ref p) = node.physics {
                            if let Some(body_type) = p.physics_body.clone() {
                                entity_cmd.set(RigidBodyDefinition {
                                    body_type: body_type.into(),
                                    mass: p.physics_mass,
                                    gravity_scale: p.physics_gravity_scale.unwrap_or(1.0),
                                    linear_damping: p.physics_linear_damping.unwrap_or(0.0),
                                    angular_damping: p.physics_angular_damping.unwrap_or(0.0),
                                });
                            }

                            if let Some(shape) = p.physics_shape.clone() {
                                let collider = ColliderDefinition {
                                    shape: build_collider_shape(node, shape, scene_data),
                                    is_trigger: p.physics_is_trigger.unwrap_or(false),
                                    offset: Transform::default(),
                                    layer: p.physics_layer.unwrap_or(0),
                                    mask: p.physics_mask.unwrap_or(u32::MAX),
                                };
                                entity_cmd.set(collider);
                            }

                            if let Some(material_name) = &p.physics_material {
                                if let Some(mat) = scene_data.physics_materials.get(material_name) {
                                    entity_cmd.set(mat.clone());
                                }
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

fn build_collider_shape(
    node: &SceneNode,
    shape: PhysicsShape,
    scene_data: &SceneData,
) -> ColliderShape {
    match shape {
        PhysicsShape::Box => {
            let scale = node.transform.scale;
            ColliderShape::Box {
                hx: scale.x * 0.5,
                hy: scale.y * 0.5,
                hz: scale.z * 0.5,
            }
        }
        PhysicsShape::Sphere => {
            let scale = node.transform.scale;
            ColliderShape::Sphere {
                radius: scale.max_element() * 0.5,
            }
        }
        PhysicsShape::Capsule => {
            let scale = node.transform.scale;
            ColliderShape::Capsule {
                radius: (scale.x.min(scale.z)) * 0.5,
                height: scale.y,
            }
        }
        // PhysicsShape::Convex | PhysicsShape::Mesh => {
        //     let mesh = scene_data.meshes.get(node.mesh_index.unwrap()).unwrap();
        //     let vertices = mesh.vertices.clone();
        //     let indices = mesh.indices.clone();

        //     if shape == PhysicsShape::Mesh {
        //         ColliderShape::Mesh { vertices, indices }
        //     } else {
        //         ColliderShape::Convex { vertices }
        //     }
        // }
        PhysicsShape::Convex | PhysicsShape::Mesh => {
            todo!("Should implement Convex and Mesh collider shapes")
        }
        PhysicsShape::Unknown => panic!("Unknown collider shape"),
    }
}
