use bevy_ecs::{resource::Resource};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::assets::{Handle, MeshData};
use tokio::runtime::Handle as TokioHandle;

// Internal Message (Heavy - Used only inside the plugin)
pub enum AssetWorkerMessage {
    MeshLoaded { id: Uuid, path: String, data: MeshData },
}

#[derive(Resource, Clone)]
pub struct AssetServer {
    event_sender: UnboundedSender<AssetWorkerMessage>,
    // The "Ticket" to the Async World
    io_handle: TokioHandle, 
}

impl AssetServer {
    pub fn new(event_sender: UnboundedSender<AssetWorkerMessage>, io_handle: TokioHandle) -> Self {
        Self { event_sender, io_handle }
    }

    pub fn load_mesh(&self, path: &str) -> Handle<MeshData> {
        let handle = Handle::<MeshData>::new(); // (Your generic handle)
        let id = handle.id;
        let path = path.to_owned();
        let sender = self.event_sender.clone();

        // FIX: Use the handle to spawn!
        // This works from ANY thread (Main, Rayon, etc.)
        self.io_handle.spawn(async move {
            println!("    [AssetServer] Loading: {}", path);
            
            // ... (Your existing loading logic) ...
            // Use tokio::task::spawn_blocking inside here if needed
            let path_clone = path.clone();
            let load_result = tokio::task::spawn_blocking(move || {
                tobj::load_obj(&path_clone, &tobj::LoadOptions {
                    single_index: true,
                    triangulate: true,
                    ..Default::default()
                })
            }).await;

            match load_result {
                Ok(Ok((models, _))) => {
                    let mesh = &models[0].mesh;
                    let data = MeshData {
                        positions: mesh.positions.clone(),
                        normals: mesh.normals.clone(),
                        uvs: mesh.texcoords.clone(),
                        indices: mesh.indices.clone(),
                    };

                    // Send success event back to Main Thread
                    let _ = sender.send(AssetWorkerMessage::MeshLoaded { id, path, data });
                }
                Err(e) => eprintln!("    [AssetServer] Task Join Error: {:?}", e),
                Ok(Err(e)) => eprintln!("    [AssetServer] OBJ Load Error: {:?} in {}", e, path),
            }
        });

        handle
    }
}