use catalyst_core::App;
use flecs_ecs::prelude::*;
use std::collections::HashMap;

use crate::{context::ContextId, logical::{ActionId, ActionState, AxisId, AxisState}};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u16)]
pub enum MouseButtonId {
    Left = 0,
    Right = 1,
    Middle = 2,
    Back = 3,
    Forward = 4,
    Other(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DeviceKind {
    Keyboard(u16),
    MouseButton(MouseButtonId),
    MouseAxis,
    GamepadButton,
    GamepadAxis,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PhysicalInputId {
    pub device: DeviceKind,
}

#[derive(Component, Default, Debug)]
pub struct InputState {
    pub active_contexts: Vec<ContextId>,
    
    pub physical_buttons: HashMap<PhysicalInputId, bool>,
    pub physical_axes: HashMap<PhysicalInputId, f32>,

    pub actions: HashMap<ActionId, ActionState>,
    pub axes: HashMap<AxisId, AxisState>,

    // optional: per-frame mouse delta 
    pub mouse_position: (f32, f32),
    pub mouse_delta: (f32, f32),
}

impl InputState {
    pub fn push_context(&mut self, ctx: ContextId) {
        self.active_contexts.push(ctx);
    }

    pub fn pop_context(&mut self) {
        self.active_contexts.pop();
    }

    pub fn set_context(&mut self, ctx: ContextId) {
        self.active_contexts.clear();
        self.active_contexts.push(ctx);
    }

    pub fn just_pressed(&self, action: ActionId) -> bool {
        if let Some(state) = self.actions.get(&action) {
            state.phase.contains(crate::logical::ButtonPhase::PRESSED)
        } else {
            false
        }
    }
}


pub fn register_input_systems(app: &App) {
    app.world
        .system::<&mut InputState>()
        .kind(flecs::pipeline::OnStore)
        .run(|mut iter| {
            while iter.next() {
                let mut input_state = iter.field_mut::<&mut InputState>(0);
                let input_state = input_state.get_mut(0).unwrap();

                input_state.mouse_delta = (0.0, 0.0);
            }
        });
}
