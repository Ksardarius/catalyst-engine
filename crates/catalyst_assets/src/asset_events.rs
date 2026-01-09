use std::collections::HashMap;

use catalyst_core::Source;
use flecs_ecs::prelude::*;
use uuid::Uuid;

use crate::{AssetReceiver, asset_server::AssetWorkerMessage};

#[derive(Component, Default)]
pub struct AssetLookup {
    pub map: HashMap<Uuid, Entity>,
}

#[derive(Component)]
pub struct AssetType;

#[derive(Component)]
pub struct TextureAsset;

#[derive(Component)]
pub struct SceneAsset;

#[derive(Component)]
pub struct MaterialAsset;

#[derive(Component)]
pub struct MeshAsset;

impl AssetLookup {
    pub fn entity(&mut self, id: Uuid, world: &World) -> Entity {
        *self.map.entry(id).or_insert_with(|| {
            // If new, create a blank entity
            *world.entity()
        })
    }
}

pub fn register_flush_system(world: &World) {
    world
        .system::<(&mut AssetReceiver, &mut AssetLookup)>()
        .kind(flecs::pipeline::OnUpdate)
        .write(AssetReceiver::id())
        .write(AssetLookup::id())
        .run(|mut iter| {
            let world = iter.world();

            while iter.next() {
                let mut receivers = iter.field_mut::<AssetReceiver>(0);
                let mut lookups = iter.field_mut::<AssetLookup>(1);

                if let (Some(receiver), Some(lookup)) = (receivers.get_mut(0), lookups.get_mut(0)) {
                    while let Ok(msg) = receiver.0.try_recv() {
                        // Handle message...while let Ok(msg) = receiver.0.try_recv() {
                        match msg {
                            AssetWorkerMessage::TextureLoaded { id, path, data } => {
                                println!("  [AssetPlugin] Offloaded Texture: {:?}", path);

                                let entity = lookup.entity(id, &world);
                                world.entity_from_id(entity).set_name(&path).set(data);
                            }
                            AssetWorkerMessage::SceneLoaded {
                                id,
                                path,
                                scene,
                                textures: loaded_textures,
                                materials: loaded_materials,
                                meshes: loaded_meshes,
                            } => {
                                println!("  [AssetPlugin] Offloaded Scene: {:?}", path);

                                let entity = lookup.entity(id, &world);
                                let scene_entity = world
                                    .entity_from_id(entity)
                                    .set_name(&path)
                                    .set(scene)
                                    .add((AssetType, SceneAsset));

                                // 1. Unpack & Store Textures
                                for (handle, data) in loaded_textures {
                                    let entity = lookup.entity(handle.id, &world);
                                    world
                                        .entity_from_id(entity)
                                        .add((Source, scene_entity))
                                        .set(data)
                                        .add((AssetType, TextureAsset));
                                }

                                // 2. Unpack & Store Materials
                                for (handle, data) in loaded_materials {
                                    let entity = lookup.entity(handle.id, &world);
                                    world
                                        .entity_from_id(entity)
                                        .add((Source, scene_entity))
                                        .add((AssetType, MaterialAsset))
                                        .set(data);
                                }

                                // 3. Unpack & Store Meshes
                                for (handle, data) in loaded_meshes {
                                    let entity = lookup.entity(handle.id, &world);
                                    
                                    world
                                        .entity_from_id(entity)
                                        .add((Source, scene_entity))
                                        .add((AssetType, MeshAsset))
                                        .set(data);
                                }

                                println!("✅ Scene '{}' fully unpacked and ready.", path);
                            }
                        }
                    }
                }
            }
        });
}
