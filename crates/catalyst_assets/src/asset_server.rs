use flecs_ecs::macros::Component;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::{
    assets::{Handle, MeshData},
    material::{MaterialData, TextureData, TextureFormat},
    scene::SceneData,
};
use tokio::runtime::Handle as TokioHandle;

mod gltf_parser;

// Internal Message (Heavy - Used only inside the plugin)
pub enum AssetWorkerMessage {
    TextureLoaded {
        id: Uuid,
        path: String,
        data: TextureData,
    },
    SceneLoaded {
        id: Uuid,
        path: String,
        // The Payload:
        scene: SceneData,
        // We carry the actual data alongside the scene description
        textures: Vec<(Handle<TextureData>, TextureData)>,
        materials: Vec<(Handle<MaterialData>, MaterialData)>,
        meshes: Vec<(Handle<MeshData>, MeshData)>,
    },
}

#[derive(Component, Clone)]
pub struct AssetServer {
    event_sender: UnboundedSender<AssetWorkerMessage>,
    // The "Ticket" to the Async World
    io_handle: TokioHandle,
}

impl AssetServer {
    pub fn new(event_sender: UnboundedSender<AssetWorkerMessage>, io_handle: TokioHandle) -> Self {
        Self {
            event_sender,
            io_handle,
        }
    }

    pub fn load_texture(&self, path: &str) -> Handle<TextureData> {
        let handle = Handle::<TextureData>::new();
        let id = handle.id;
        let path = path.to_owned();
        let sender = self.event_sender.clone();

        self.io_handle.spawn(async move {
            let path_clone = path.clone();

            // Blocking load via 'image' crate
            let load_result = tokio::task::spawn_blocking(move || {
                // Open file
                let img = image::open(&path_clone).map_err(|e| e.to_string())?;
                // Convert to RGBA8 (Standard for GPU)
                let rgba = img.to_rgba8();

                Ok::<TextureData, String>(TextureData {
                    name: path_clone.clone(),
                    width: rgba.width(),
                    height: rgba.height(),
                    pixels: rgba.into_raw(),
                    format: TextureFormat::Rgba8Unorm, // Use sRGB for colors!
                })
            })
            .await;

            match load_result {
                Ok(Ok(data)) => {
                    let _ = sender.send(AssetWorkerMessage::TextureLoaded { id, path, data });
                }
                _ => eprintln!("Failed to load texture: {}", path),
            }
        });

        handle
    }

    pub fn load_scene(&self, path: &str) -> Handle<SceneData> {
        let handle = Handle::<SceneData>::new();
        let id = handle.id;
        let path = path.to_owned();
        let sender = self.event_sender.clone();

        // Spawn background task
        self.io_handle.spawn(async move {
            let path_clone = path.clone();
            // Run blocking parser
            let result = tokio::task::spawn_blocking(move || gltf_parser::parse_gltf(&path_clone)).await;

            match result {
                Ok(Ok(payload)) => {
                    // Send the "Big Payload" back to main thread
                    let _ = sender.send(AssetWorkerMessage::SceneLoaded {
                        id,
                        path,
                        scene: payload.0,
                        textures: payload.1,
                        materials: payload.2,
                        meshes: payload.3,
                    });
                }
                Err(e) => eprintln!("GLTF Task Error: {:?}", e),
                Ok(Err(e)) => eprintln!("Failed to parse GLTF '{}': {}", path, e),
            }
        });

        handle
    }
}
