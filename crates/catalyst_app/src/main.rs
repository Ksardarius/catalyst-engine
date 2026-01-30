use catalyst_assets::{AssetPlugin, asset_server::AssetServer};
use catalyst_core::{
    App,
    camera::Camera,
    time::Time,
    transform::{GlobalTransform, Transform},
};
use catalyst_debug::{ACTION_ENABLE_DEBUG, DebugPlugin};
use catalyst_input::{
    InputPlugin,
    context::CTX_GAMEPLAY,
    logical::{ActionId, AxisId, ButtonPhase, InputBinding, InputMap},
    physical::{DeviceKind, InputState, PhysicalInputId},
};
use catalyst_physics::PhysicsPlugin;
use catalyst_renderer::RenderPlugin;
use catalyst_scene::{ScenePlugin, SceneRoot};
use catalyst_window::{WindowPlugin, run_catalyst_app};
use flecs_ecs::{
    addons::stats,
    core::{World, WorldGet, flecs},
    prelude::{ComponentId, QueryBuilderImpl, SystemAPI},
};
use glam::{Mat4, Quat, Vec3};

pub const ACTION_MOVE_FORWARD: ActionId = ActionId(1);
pub const ACTION_MOVE_BACKWARD: ActionId = ActionId(2);
pub const ACTION_MOVE_LEFT: ActionId = ActionId(3);
pub const ACTION_MOVE_RIGHT: ActionId = ActionId(4);

pub const AXIS_LOOK_X: AxisId = AxisId(100);
pub const AXIS_LOOK_Y: AxisId = AxisId(101);

fn main() {
    let mut app = App::new();

    app.add_plugin(InputPlugin);
    app.add_plugin(WindowPlugin);
    app.add_plugin(AssetPlugin);
    app.add_plugin(ScenePlugin);
    app.add_plugin(RenderPlugin);
    app.add_plugin(PhysicsPlugin);

    // debug plugin must be last
    app.add_plugin(DebugPlugin);

    app.world
        .system::<&mut AssetServer>()
        .kind(flecs::pipeline::OnStart)
        .run(|iter| {
            // Inject the world here
            setup_scene(&iter.world());
        });

    app.world
        .system_named::<(&mut Transform, &Time, &InputState)>("movement_system")
        .with(Camera::id())
        .kind(flecs::pipeline::OnUpdate)
        .each(|(transform, time, input)| {
            let speed = 5.0 * time.delta_seconds();

            let go_forward = input
                .actions
                .get(&ACTION_MOVE_FORWARD)
                .map(|a| a.phase.contains(ButtonPhase::HELD))
                .unwrap_or(false);

            let go_backward = input
                .actions
                .get(&ACTION_MOVE_BACKWARD)
                .map(|a| a.phase.contains(ButtonPhase::HELD))
                .unwrap_or(false);

            let go_left = input
                .actions
                .get(&ACTION_MOVE_LEFT)
                .map(|a| a.phase.contains(ButtonPhase::HELD))
                .unwrap_or(false);

            let go_right = input
                .actions
                .get(&ACTION_MOVE_RIGHT)
                .map(|a| a.phase.contains(ButtonPhase::HELD))
                .unwrap_or(false);

            let forward = transform.rotation * -Vec3::Z;
            let right = transform.rotation * Vec3::X;
            let up = Vec3::Y; // Global Up

            if go_forward {
                transform.translation += forward * speed;
            }

            if go_backward {
                transform.translation -= forward * speed;
            }

            if go_left {
                transform.translation -= right * speed;
            }

            if go_right {
                transform.translation += right * speed;
            }
        });

    app.world
        .system_named::<(&mut Transform, &InputState)>("camera_movement_system")
        .with(Camera::id())
        .kind(flecs::pipeline::OnUpdate)
        .each(|(transform, input)| {
            let sensitivity = 0.002;

            // Raw mouse delta
            let (dx, dy) = input.mouse_delta; // Apply sensitivity and delta time 
            let yaw = -dx * sensitivity;
            let pitch = -dy * sensitivity; // Convert to quaternions 
            let yaw_q = Quat::from_rotation_y(yaw);
            let pitch_q = Quat::from_rotation_x(pitch);

            transform.rotation = yaw_q * transform.rotation;
            transform.rotation = transform.rotation * pitch_q;
        });

    app.world.import::<stats::Stats>();
    app.world.set(flecs::rest::Rest::default());

    run_catalyst_app(app)
}

// /// -------------------------------------------------------------------
// /// SYSTEM: Setup Scene
// /// -------------------------------------------------------------------
fn setup_scene(world: &World) {
    world.get::<&AssetServer>(|asset_server| {
        println!("Requesting Mesh Load...");

        let scene_handle = asset_server.load_scene("assets/box5.glb");
        world
            .entity()
            .set(SceneRoot(scene_handle.clone()))
            .set(Transform::from_xyz(0.0, 0.0, 0.0))
            .set(GlobalTransform::default());

        world.get::<&mut InputState>(|input_state| {
            input_state.push_context(CTX_GAMEPLAY);
        });

        world.get::<&mut InputMap>(|input_map| {
            input_map
                .bind_keyboard_button(winit::keyboard::KeyCode::KeyW as u16, ACTION_MOVE_FORWARD);
            input_map
                .bind_keyboard_button(winit::keyboard::KeyCode::KeyS as u16, ACTION_MOVE_BACKWARD);
            input_map.bind_keyboard_button(winit::keyboard::KeyCode::KeyA as u16, ACTION_MOVE_LEFT);
            input_map
                .bind_keyboard_button(winit::keyboard::KeyCode::KeyD as u16, ACTION_MOVE_RIGHT);

            input_map
                .bind_keyboard_button(winit::keyboard::KeyCode::Tab as u16, ACTION_ENABLE_DEBUG);
            input_map.bind_keyboard_button_with_context(
                winit::keyboard::KeyCode::Tab as u16,
                ACTION_ENABLE_DEBUG,
                catalyst_input::context::CTX_DEBUG,
            );
        });

        // world
        //     .entity()
        //     .set(SceneRoot(scene_handle))
        //     .set(Transform::from_xyz(10.0, 0.0, 0.0))
        //     .set(GlobalTransform(Mat4::default().mul_scalar(10.0)));

        // world
        //     .entity()
        //     .set_name("camera")
        //     .set(Camera::default())
        //     .set(
        //         Transform::from_xyz(0.0, 2.0, 5.0) // Up 2, Back 5
        //             .looking_at(Vec3::ZERO, Vec3::Y),
        //     );
    });
}
