use std::collections::HashSet;

use bevy_ecs::resource::Resource;
pub use winit::keyboard::KeyCode;

#[derive(Resource, Default)]
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

