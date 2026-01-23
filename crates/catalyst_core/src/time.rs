use std::time::{Duration, Instant};

use flecs_ecs::macros::Component;

#[derive(Component)]
pub struct Time {
    startup: Instant,
    last_update: Instant,
    delta: Duration,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            startup: Instant::now(),
            last_update: Instant::now(),
            delta: Duration::ZERO,
        }
    }
}

impl Time {
    /// Called by the engine loop once per frame
    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta = now - self.last_update;
        self.last_update = now;
    }

    /// Returns time in seconds since last frame (e.g., 0.016 for 60fps)
    pub fn delta_seconds(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// Returns total time since app started
    pub fn elapsed_seconds(&self) -> f32 {
        self.startup.elapsed().as_secs_f32()
    }
}

#[derive(Component)]
pub struct PhysicsTime {
    pub accumulator: f32,
    pub fixed_dt: f32,
}

impl Default for PhysicsTime {
    fn default() -> Self {
        Self { accumulator: 0.0, fixed_dt: 1.0 / 60.0 }
    }
}

impl PhysicsTime {
    pub fn should_step(&mut self, dt: f32) -> bool {
        self.accumulator += dt;
        if self.accumulator >= self.fixed_dt {
            self.accumulator -= self.fixed_dt;
            true
        } else {
            false
        }
    }
}

