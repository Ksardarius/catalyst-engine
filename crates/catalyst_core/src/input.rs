use std::collections::HashSet;

use flecs_ecs::macros::Component;
use winit::event::WindowEvent;
pub use winit::keyboard::KeyCode;

#[derive(Component, Default)]
pub struct Input {
    pressed: HashSet<KeyCode>,
}

impl Input {
    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn press(&mut self, key: KeyCode) {
        self.pressed.insert(key);
    }

    pub fn release(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
    }
}

// needed to pass event to GUI systems
#[derive(Component, Default)]
pub struct SystemEvents {
    pub buffer: Vec<WindowEvent>,
}

impl SystemEvents {
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

