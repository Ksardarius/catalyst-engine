use catalyst_core::{App, IoTaskPool, Plugin};
use flecs_ecs::prelude::*;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::{
    asset_events::{AssetLookup, AssetType, register_flush_system},
    asset_server::{AssetServer, AssetWorkerMessage}
};

pub mod asset_events;
pub mod asset_server;
pub mod assets;
mod components;
pub mod material;
pub mod scene;

pub use components::{MaterialDefinition, MeshDefinition};

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.world.component::<AssetType>().add_trait::<flecs::Exclusive>();

        let io_handle = app.world.get::<&IoTaskPool>(|t| t.0.clone());
        // 2. Create the internal communication channel
        let (tx, rx) = unbounded_channel::<AssetWorkerMessage>();

        // 3. Create and Insert the AssetServer (Public API)
        let server = AssetServer::new(tx, io_handle);
        app.register_singleton(server);
        app.register_singleton_default::<AssetLookup>();
        app.register_singleton(AssetReceiver(rx));

        register_flush_system(&app.world);
    }
}

// Internal wrapper to hold the receiver
#[derive(Component)]
struct AssetReceiver(UnboundedReceiver<AssetWorkerMessage>);
