pub use bevy_ecs::prelude::*;
use bevy_ecs::system::ScheduleSystem;
pub use tokio;
pub use rayon;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};

pub mod transform;
pub mod camera;
pub mod assets;
pub mod input;
pub mod time;

pub use assets::*;
pub use input::*;

use crate::time::Time;

// --- 1. THE MESSAGE PROTOCOL ---
// What can the background threads say to the main thread?
#[derive(Message)]
pub enum EngineEvent {
    AssetLoaded { name: String, data: Vec<u8> },
    NetworkMessage { content: String },
    // Add more types here as your engine grows
}

// --- 2. RESOURCES ---
// The ECS needs to hold the Receiver so it can check it every frame.
#[derive(Resource)]
pub struct IoReceiver(UnboundedReceiver<EngineEvent>);

// The ECS needs to hold a copy of the Sender to give to new tasks.
#[derive(Resource, Clone)]
pub struct IoSender(pub UnboundedSender<EngineEvent>);

/// The Plugin Trait
/// Every module (Renderer, Physics, Window) must implement this.
pub trait Plugin {
    fn build(&self, app: &mut App);
}

/// The Engine Application
/// Holds the ECS World and orchestrates the loop.
pub struct App {
    pub world: World,
    pub main_schedule: Schedule,
    pub startup_schedule: Schedule,
    // The Async Runtime for IO (Network, Save/Load)
    // We keep a handle to it so we can spawn tasks from anywhere.
    pub io_runtime: tokio::runtime::Runtime,
    pub running: bool
}

impl App {
    pub fn new() -> Self {
        // 1. Initialize Rayon (Global Compute Pool)
        // Rayon initializes itself globally the first time you use it.
        // But we can configure it manually if we want to reserve threads.
        // For now, we let it take over the available cores for CPU work.
        rayon::ThreadPoolBuilder::new().num_threads(4).build_global().ok();

        // 2. Initialize Tokio (Dedicated IO Pool)
        // We create a Multi-Threaded runtime dedicated to I/O.
        let io_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("catalyst-io")
            .build()
            .unwrap();

        // --- BRIDGE INITIALIZATION ---
        // Create the channel
        let (tx, rx) = unbounded_channel::<EngineEvent>();

        let mut world = World::new();
        // Store the Bridge ends in the World
        world.insert_resource(IoReceiver(rx));
        world.insert_resource(IoSender(tx));

        // Register the Event Type so systems can read it
        world.init_resource::<Messages<EngineEvent>>();
        world.init_resource::<Input>();
        world.init_resource::<Time>();

        let mut main_schedule = Schedule::default();

        // CRITICAL: Add the "Drain" system to the main loop automatically
        // This ensures messages are processed every frame.
        main_schedule.add_systems(read_async_events);

        Self {
            world,
            main_schedule,
            startup_schedule: Schedule::default(),
            io_runtime,
            running: true
        }
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn add_system<M>(&mut self, system: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
        self.main_schedule.add_systems(system);
        self
    }

    pub fn add_startup_system<M>(&mut self, system: impl IntoScheduleConfigs<ScheduleSystem, M>) -> &mut Self {
        self.startup_schedule.add_systems(system);
        self
    }

    // Updated helper: Now background tasks can get a Sender!
    pub fn spawn_io<F>(&self, task_logic: impl FnOnce(IoSender) -> F) 
    where 
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        // We clone the sender to give to the new task
        let sender = self.world.resource::<IoSender>().clone();
        self.io_runtime.spawn(task_logic(sender));
    }

    /// ⚡ The "Better" Part: Explicit Ticks
    /// We do not have a run() function that takes over the thread.
    /// We have a tick() function that processes ONE frame.
    /// This allows the Windowing system to decide WHEN to run.
    pub fn update(&mut self) {
        if !self.running { return; }
        self.main_schedule.run(&mut self.world);
    }
    
    pub fn startup(&mut self) {
        self.startup_schedule.run(&mut self.world);
    }
}

// --- 3. THE BRIDGE SYSTEM ---
// This runs on the MAIN THREAD every frame.
// It empties the mailbox and puts messages into the ECS Event Queue.
fn read_async_events(mut receiver: ResMut<IoReceiver>, mut events: MessageWriter<EngineEvent>) {
    // try_recv() is non-blocking. It grabs everything currently waiting.
    while let Ok(msg) = receiver.0.try_recv() {
        events.write(msg);
    }
}

