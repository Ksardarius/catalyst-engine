use crate::physical::{InputState, PhysicalInputId};
use catalyst_core::App;
use flecs_ecs::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ActionId(pub u32);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct AxisId(pub u32);

#[derive(Clone, Debug)]
pub enum BindingKind {
    Button { action: ActionId },
    Axis { axis: AxisId, scale: f32 },
}

#[derive(Clone, Debug)]
pub struct InputBinding {
    pub physical: PhysicalInputId,
    pub kind: BindingKind,
}

bitflags::bitflags! {
    #[derive(Clone, Debug)]
    pub struct ButtonPhase: u8 {
        const NONE     = 0;
        const PRESSED  = 1 << 0;
        const HELD     = 1 << 1;
        const RELEASED = 1 << 2;
    }
}

#[derive(Clone, Debug)]
pub struct ActionState {
    pub phase: ButtonPhase,
}

#[derive(Clone, Debug)]
pub struct AxisState {
    pub value: f32,
}

#[derive(Component, Default, Clone, Debug)]
pub struct InputMap {
    pub bindings: Vec<InputBinding>,
}

impl InputMap {
    pub fn bind_keyboard_button(&mut self, key_code: u16, action: ActionId) -> &mut Self {
        self.bindings.push(InputBinding {
            physical: PhysicalInputId {
                device: crate::physical::DeviceKind::Keyboard(key_code),
            },
            kind: BindingKind::Button { action },
        });

        self
    }
}

pub fn register_sys_input_map(app: &mut App) {
    app.world
        .system_named::<(&InputMap, &mut InputState)>("sys_input_map")
        .kind(flecs::pipeline::OnUpdate)
        .run(|mut iter| {
            while iter.next() {
                let input_map = &iter.field::<&InputMap>(0)[0];
                let input_state = &mut iter.field_mut::<&InputState>(1)[0];

                // Reset logical state
                for (_, action) in input_state.actions.iter_mut() {
                    let was_held = action.phase.contains(ButtonPhase::HELD);
                    action.phase = if was_held {
                        ButtonPhase::HELD
                    } else {
                        ButtonPhase::NONE
                    };
                }
                for (_, axis) in input_state.axes.iter_mut() {
                    axis.value = 0.0;
                }

                // Apply bindings
                for binding in &input_map.bindings {
                    match binding.kind {
                        BindingKind::Button { action } => {
                            let pressed = *input_state
                                .physical_buttons
                                .get(&binding.physical)
                                .unwrap_or(&false);
                            let entry = input_state.actions.entry(action).or_insert(ActionState {
                                phase: ButtonPhase::NONE,
                            });
                            if pressed {
                                if !entry.phase.contains(ButtonPhase::HELD) {
                                    entry.phase |= ButtonPhase::PRESSED | ButtonPhase::HELD;
                                } else {
                                    entry.phase |= ButtonPhase::HELD;
                                }
                            } else {
                                if entry.phase.contains(ButtonPhase::HELD) {
                                    entry.phase &= !ButtonPhase::HELD;
                                    entry.phase |= ButtonPhase::RELEASED;
                                }
                            }
                        }
                        BindingKind::Axis { axis, scale } => {
                            let value = input_state
                                .physical_axes
                                .get(&binding.physical)
                                .copied()
                                .unwrap_or(0.0);
                            let entry = input_state
                                .axes
                                .entry(axis)
                                .or_insert(AxisState { value: 0.0 });
                            entry.value += value * scale;
                        }
                    }
                }
            }
        });
}
