use catalyst_core::{App, IoTaskPool, Plugin};
use flecs_ecs::prelude::*;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::{
    asset_events::{AssetLookup, AssetType, register_flush_system},
    asset_server::{AssetServer, AssetWorkerMessage}, scene::{SceneData},
};

pub mod asset_events;
pub mod asset_server;
pub mod assets;
mod components;
pub mod material;
pub mod physics;
pub mod scene;

pub use components::{MaterialDefinition, MeshDefinition};

#[derive(Component, Debug, Clone)]
#[flecs(meta)]
pub struct AssetSource {
    pub path: String,
}

#[derive(Component)]
pub struct LoadTexture;
#[derive(Component)]
pub struct LoadMesh;
#[derive(Component)]
pub struct LoadSkybox;
#[derive(Component)]
pub struct LoadScene;

#[derive(Component)]
pub struct Loading; // Tag: "I am currently busy, don't touch me"

#[derive(Component)]
pub struct AssetError(pub String); 

pub struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.world
            .component::<AssetType>()
            .add_trait::<flecs::Exclusive>();

        app.world.component::<AssetSource>();
        app.world.component::<LoadScene>();

        let io_handle = app.world.get::<&IoTaskPool>(|t| t.0.clone());
        // 2. Create the internal communication channel
        let (tx, rx) = unbounded_channel::<AssetWorkerMessage>();

        // 3. Create and Insert the AssetServer (Public API)
        let server = AssetServer::new(tx, io_handle);
        app.register_singleton(server);
        app.register_singleton_default::<AssetLookup>();
        app.register_singleton(AssetReceiver(rx));

        app.world
            .system_named::<(&AssetSource, &AssetServer)>("load_assets")
            .with(LoadScene)
            .without(Loading)
            .without(SceneData::id())
            .kind(flecs::pipeline::OnUpdate)
            .each_entity(|entity, (source, assets)| {
                assets.load_scene(&source.path, entity.id());
                entity.add(Loading);
            });

        register_flush_system(&app.world);
    }
}

// Internal wrapper to hold the receiver
#[derive(Component)]
struct AssetReceiver(UnboundedReceiver<AssetWorkerMessage>);
