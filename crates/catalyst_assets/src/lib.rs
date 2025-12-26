use catalyst_core::{
    App, IoTaskPool, Message, MessageWriter, Messages, NonSendMut, Plugin, ResMut,
};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use uuid::Uuid;

use crate::{
    asset_server::{AssetServer, AssetWorkerMessage},
    assets::{Assets, MeshData},
    material::{MaterialData, TextureData},
    scene::SceneData,
};

pub mod asset_server;
pub mod assets;
pub mod material;
pub mod scene;
mod components;

pub use components::{MeshDefinition, MaterialDefinition};

#[derive(Message, Clone, Debug)]
pub enum AssetEvent {
    MeshLoaded { id: Uuid, path: String },
    TextureLoaded { id: Uuid, path: String },
    MaterialLoaded { id: Uuid },
}

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        // Assume App trait is visible or use bevy App
        let io_handle = app.world.resource::<IoTaskPool>().0.clone();

        // 2. Create the internal communication channel
        let (tx, rx) = unbounded_channel::<AssetWorkerMessage>();

        // 3. Create and Insert the AssetServer (Public API)
        let server = AssetServer::new(tx, io_handle);
        app.world.insert_resource(server);

        // 4. Init Storage
        app.world.init_resource::<Assets<MeshData>>();
        app.world.init_resource::<Assets<SceneData>>();
        app.world.init_resource::<Assets<MaterialData>>();
        app.world.init_resource::<Assets<TextureData>>();

        app.world.insert_non_send_resource(AssetReceiver(rx));

        app.add_system(flush_asset_events);

        // 6. Register Events
        app.world.init_resource::<Messages<AssetEvent>>();
    }
}

// Internal wrapper to hold the receiver
struct AssetReceiver(UnboundedReceiver<AssetWorkerMessage>);

// The system that drains the channel and fires ECS events
fn flush_asset_events(
    mut receiver: NonSendMut<AssetReceiver>,
    mut events: MessageWriter<AssetEvent>, // Writes the Public Event
    scenes: ResMut<Assets<SceneData>>,
    meshes: ResMut<Assets<MeshData>>,
    materials: ResMut<Assets<MaterialData>>,
    textures: ResMut<Assets<TextureData>>,
) {
    while let Ok(msg) = receiver.0.try_recv() {
        match msg {
            AssetWorkerMessage::TextureLoaded { id, path, data } => {
                textures.insert_by_id(id, data);

                println!("  [AssetPlugin] Offloaded Texture: {:?}", path);

                events.write(AssetEvent::TextureLoaded { id, path });
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
                
                // 1. Unpack & Store Textures
                for (handle, data) in loaded_textures {
                    // Store the data in RAM
                    textures.insert(handle.clone(), data);
                    
                    // Notify Renderer: "Hey, a texture is ready to upload!"
                    // The renderer treats this exactly the same as if we loaded a single PNG.
                    events.write(AssetEvent::TextureLoaded { 
                        id: handle.id, 
                        path: "embedded_in_scene".to_string() 
                    });
                }

                // 2. Unpack & Store Materials
                for (handle, data) in loaded_materials {
                    materials.insert(handle.clone(), data);
                    // Notify Renderer to build Bind Groups
                    events.write(AssetEvent::MaterialLoaded { id: handle.id });
                }

                // 3. Unpack & Store Meshes
                for (handle, data) in loaded_meshes {
                    meshes.insert(handle.clone(), data);
                    // Notify Renderer to upload Vertex Buffers
                    events.write(AssetEvent::MeshLoaded { 
                        id: handle.id, 
                        path: "embedded_in_scene".to_string() 
                    });
                }

                // 4. Finally, Store the Scene Blueprint
                // This is what 'spawn_scenes' is waiting for!
                scenes.insert_by_id(id, scene);

                println!("✅ Scene '{}' fully unpacked and ready.", path);
            }
        }
    }
}
