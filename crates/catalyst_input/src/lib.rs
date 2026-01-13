use catalyst_core::{App, Plugin};
use flecs_ecs::prelude::*;

use crate::{
    logical::{InputMap, register_sys_input_map},
    physical::{InputState, register_input_systems},
};

pub mod logical;
pub mod physical;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut catalyst_core::App) {
        app.register_singleton_default::<InputState>();
        app.register_singleton_default::<InputMap>();

        register_input_systems(app);
        register_sys_input_map(app);
    }
}


