use catalyst_core::{App, IoTaskPool, Message, MessageWriter, Messages, NonSendMut, Plugin, ResMut};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use uuid::Uuid;

use crate::{asset_server::{AssetServer, AssetWorkerMessage}, assets::{Assets, MeshData}};

pub mod asset_server;
pub mod assets;

#[derive(Message, Clone, Debug)]
pub enum AssetEvent {
    MeshLoaded { id: Uuid, path: String }, 
    // No 'data' field here!
}

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) { // Assume App trait is visible or use bevy App
        let io_handle = app.world.resource::<IoTaskPool>().0.clone();

        // 2. Create the internal communication channel
        let (tx, rx) = unbounded_channel::<AssetWorkerMessage>();

        // 3. Create and Insert the AssetServer (Public API)
        let server = AssetServer::new(tx, io_handle);
        app.world.insert_resource(server);

        // 4. Init Storage
        app.world.init_resource::<Assets<MeshData>>();

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
    meshes: ResMut<Assets<MeshData>>, // We can handle insertion right here!
) {
    while let Ok(msg) = receiver.0.try_recv() {
        match msg {
            AssetWorkerMessage::MeshLoaded { id, path, data } => {
                // 1. Move Data to Storage (ZERO COPY)
                // We take ownership of 'data' and put it directly into the map.
                meshes.insert_by_id(id, data);
                
                println!("  [AssetPlugin] Offloaded Mesh: {:?}", path);

                // 2. Send Notification (Lightweight)
                // We just copy the ID and Path strings, which is cheap.
                events.write(AssetEvent::MeshLoaded { id, path });
            }
        }
    }
}
