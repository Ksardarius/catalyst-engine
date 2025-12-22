pub use bevy_ecs::prelude::*;
use bevy_ecs::system::ScheduleSystem;
pub use tokio;
pub use rayon;

pub mod transform;
pub mod camera;
pub mod input;
pub mod time;

pub use input::*;

use crate::time::Time;

#[derive(Resource, Clone)]
pub struct IoTaskPool(pub tokio::runtime::Handle);

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
    pub running: bool,
    pub io_runtime: tokio::runtime::Runtime,
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

        let mut world = World::new();
        // Store the Bridge ends in the World
        world.insert_resource(IoTaskPool(io_runtime.handle().clone()));

        // Register the Event Type so systems can read it
        world.init_resource::<Input>();
        world.init_resource::<Time>();

        let main_schedule = Schedule::default();

        Self {
            world,
            main_schedule,
            startup_schedule: Schedule::default(),
            running: true,
            io_runtime
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
