use flecs_ecs::macros::Component;
use winit::event::WindowEvent;
pub use winit::keyboard::KeyCode;

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

