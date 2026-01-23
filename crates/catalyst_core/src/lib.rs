use flecs_ecs::prelude::*;
pub use rayon;
pub use tokio;

pub mod camera;
pub mod input;
pub mod time;
pub mod transform;
pub mod pipeline;
pub mod physics;

pub use input::*;

use crate::{
    pipeline::define_pipeline_stages, time::{PhysicsTime, Time}, transform::{
        GlobalTransform, ReflectQuat, ReflectVec3, ReflectVec4, Transform, transform_propagation_system
    }
};

#[derive(Component, Clone)]
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
    pub running: bool,
    pub io_runtime: tokio::runtime::Runtime,
}

#[derive(Component)]
pub struct Source;

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
        world
            .component::<IoTaskPool>()
            .add_trait::<flecs::Singleton>()
            .set(IoTaskPool(io_runtime.handle().clone()));

        // Register the Event Type so systems can read it
        world
            .component::<SystemEvents>()
            .add_trait::<flecs::Singleton>()
            .set(SystemEvents::default());
        world
            .component::<Time>()
            .add_trait::<flecs::Singleton>();
        world.set(Time::default());

        world
            .component::<PhysicsTime>()
            .add_trait::<flecs::Singleton>();
        world.set(PhysicsTime::default());

        define_pipeline_stages(&mut world);

        let vec3_id = world.component_id::<ReflectVec3>();
        let vec4_id = world.component_id::<ReflectVec4>();
        let quat_id = world.component_id::<ReflectQuat>();
        // C. Register Transform
        // We manually say: "Field 'translation' has type 'vec3_id'"
        world
            .component::<Transform>()
            .member(quat_id, "rotation")
            .member(vec3_id, "translation")
            .member(vec3_id, "scale");

        world
            .component::<GlobalTransform>()
            .member(vec4_id, "x_axis")
            .member(vec4_id, "y_axis")
            .member(vec4_id, "z_axis")
            .member(vec4_id, "w_axis");

        let mut app = Self {
            world,
            running: true,
            io_runtime,
        };

        transform_propagation_system(&mut app.world);

        app
    }

    pub fn add_plugin<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        plugin.build(self);
        self
    }

    pub fn update(&mut self) {
        if !self.running {
            return;
        }

        self.world.progress();
    }

    pub fn register_singleton<T: ComponentId + DataComponent + ComponentType<Struct>>(&mut self, component: T) -> &mut Self {
        self.world
            .component::<T>()
            .add_trait::<flecs::Singleton>();

        self.world.set::<T>(component);

        self
    }

    pub fn register_singleton_default<T: ComponentId + DataComponent + ComponentType<Struct> + Default>(&mut self) -> &mut Self {
        self.register_singleton(T::default())
    }

    pub fn startup(&mut self) {
        println!("App Startup");
    }
}
