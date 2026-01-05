pub use bevy_ecs::prelude::*;
use bevy_ecs::{schedule::InternedSystemSet, system::ScheduleSystem};
pub use rayon;
pub use tokio;

pub mod camera;
pub mod input;
pub mod time;
pub mod transform;

pub use input::*;

use crate::{time::Time, transform::transform_propagation_system};

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
    pub pre_update_schedule: Schedule,
    pub post_update_schedule: Schedule,
    pub render_schedule: Schedule,
    pub running: bool,
    pub io_runtime: tokio::runtime::Runtime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Startup,
    PreUpdate,
    Update,
    Render,
    PostUpdate, // <--- The new stage for Transforms/Physics
}

impl App {
    pub fn new() -> Self {
        // 1. Initialize Rayon (Global Compute Pool)
        // Rayon initializes itself globally the first time you use it.
        // But we can configure it manually if we want to reserve threads.
        // For now, we let it take over the available cores for CPU work.
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build_global()
            .ok();

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
        world.init_resource::<SystemEvents>();
        world.init_resource::<Time>();

        // 1. Initialize the Registry
        world.init_resource::<AppTypeRegistry>();

        // 2. Register standard types (f32, Vec3, etc.)
        {
            // let registry = world.resource::<AppTypeRegistry>();
            // let mut registry = registry.write();
            // bevy_reflect::std_traits::register_std_traits(&mut registry);
            // If you use glam/nalgebra, register them here too!
        }

        let mut app = Self {
            world,
            main_schedule: Schedule::default(),
            startup_schedule: Schedule::default(),
            pre_update_schedule: Schedule::default(),
            post_update_schedule: Schedule::default(),
            render_schedule: Schedule::default(),
            running: true,
            io_runtime,
        };

        app.add_system_to_stage(Stage::PostUpdate, &transform_propagation_system);

        app

        
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn add_system<M>(
        &mut self,
        system: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.main_schedule.add_systems(system);
        self
    }

    pub fn add_startup_system<M>(
        &mut self,
        system: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.startup_schedule.add_systems(system);
        self
    }

    pub fn add_system_to_stage<M>(
        &mut self,
        stage: Stage,
        system: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        match stage {
            Stage::Startup => {
                self.startup_schedule.add_systems(system);
            }
            Stage::PreUpdate => {
                self.pre_update_schedule.add_systems(system);
            }
            Stage::Update => {
                self.main_schedule.add_systems(system);
            }
            Stage::PostUpdate => {
                self.post_update_schedule.add_systems(system);
            },
            Stage::Render => {
                self.render_schedule.add_systems(system);
            }
        }
        self
    }

    pub fn configure_sets<M>(&mut self, stage: Stage, systems: impl IntoScheduleConfigs<InternedSystemSet, M>) -> &mut Self {
        match stage {
            Stage::Startup => {
                self.startup_schedule.configure_sets(systems);
            }
            Stage::PreUpdate => {
                self.pre_update_schedule.configure_sets(systems);
            }
            Stage::Update => {
                self.main_schedule.configure_sets(systems);
            }
            Stage::PostUpdate => {
                self.post_update_schedule.configure_sets(systems);
            }
            Stage::Render => {
                self.render_schedule.configure_sets(systems);
            }
        }
        self
    }

    /// ⚡ The "Better" Part: Explicit Ticks
    /// We do not have a run() function that takes over the thread.
    /// We have a tick() function that processes ONE frame.
    /// This allows the Windowing system to decide WHEN to run.
    pub fn update(&mut self) {
        if !self.running {
            return;
        }
        self.pre_update_schedule.run(&mut self.world);
        self.main_schedule.run(&mut self.world);
        self.post_update_schedule.run(&mut self.world);
        self.render_schedule.run(&mut self.world);
    }

    pub fn startup(&mut self) {
        self.startup_schedule.run(&mut self.world);
    }
}
