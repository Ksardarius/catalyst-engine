use catalyst_assets::{AssetPlugin, asset_server::AssetServer};
// use catalyst_assets::{AssetPlugin, asset_server::AssetServer, assets::Handle, scene::SceneData};
// Component to track a heavy calculation
// use flecs_ecs::prelude::*;
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
use catalyst_renderer::RenderPlugin;
use catalyst_scene::{ScenePlugin, SceneRoot};
// use catalyst_debug::DebugPlugin;
// use catalyst_renderer::{
//     RenderPlugin,
//     mesh::{Mesh},
// };
// use catalyst_scene::ScenePlugin;
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
    app.add_plugin(DebugPlugin);

    // Spawn the Triangle
    // app.add_startup_system(setup_scene);

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

    // app.add_system(spin_model);
    // app.add_system(move_camera);

    app.world.import::<stats::Stats>();
    app.world.set(flecs::rest::Rest::default());

    run_catalyst_app(app)
}

// #[derive(Component)]
// pub struct SceneRoot(pub Handle<SceneData>);

// /// -------------------------------------------------------------------
// /// SYSTEM: Setup Scene
// /// -------------------------------------------------------------------
fn setup_scene(world: &World) {
    world.get::<&AssetServer>(|asset_server| {
        println!("Requesting Mesh Load...");

        let scene_handle = asset_server.load_scene("assets/simple2.glb");
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
// fn setup_scene(asset_server: &mut AssetServer/*, commands: &mut Commands, asset_server: &Res<AssetServer>*/) {
//     println!("Requesting Mesh Load...");

//     let scene_handle = asset_server.load_scene("assets/scene2.glb");
//     .set(catalyst_scene::SceneRoot(scene_handle))

//     // Optional: Give it a position!
//     // If you don't set this, it spawns at (0,0,0)
//     .set(Transform::from_xyz(0.0, 0.0, 0.0));

//     // commands.spawn(catalyst_scene::SceneRoot(scene_handle));

//     // // commands.spawn((
//     // //     SceneRoot(asset_server.load_scene("assets/scene1.glb")),
//     // //     Transform::default(),
//     // // ));

//     // // C. Spawn the Camera
//     // commands.spawn((
//     //     Camera::default(),
//     //     Transform::from_xyz(0.0, 2.0, 5.0) // Up 2, Back 5
//     //         .looking_at(Vec3::ZERO, Vec3::Y), // Look at center
//     // ));
// }

// fn rotate_triangle(mut query: Query<(&mut Transform, &Mesh)>) {
//     for (mut transform, _) in &mut query {
//         // Rotate around Z axis (Spinning)
//         transform.rotation *= Quat::from_rotation_y(0.02);
//     }
// }

// // A simple Fly Cam
// fn move_camera(
//     time: Res<Time>, // <--- Request Time
//     input: Res<Input>,
//     mut query: Query<&mut Transform, With<Camera>>,
// ) {
//     let speed = 5.0 * time.delta_seconds();

//     if let Ok(mut transform) = query.single_mut() {
//         let forward = transform.rotation * -Vec3::Z;
//         let right = transform.rotation * Vec3::X;
//         let up = Vec3::Y; // Global Up

//         if input.is_pressed(KeyCode::KeyW) {
//             transform.translation += forward * speed;
//         }
//         if input.is_pressed(KeyCode::KeyS) {
//             transform.translation -= forward * speed;
//         }
//         if input.is_pressed(KeyCode::KeyA) {
//             transform.translation -= right * speed;
//         }
//         if input.is_pressed(KeyCode::KeyD) {
//             transform.translation += right * speed;
//         }
//         if input.is_pressed(KeyCode::Space) {
//             transform.translation += up * speed;
//         }
//         if input.is_pressed(KeyCode::ShiftLeft) {
//             transform.translation -= up * speed;
//         }
//     }
// }

// /// -------------------------------------------------------------------
// /// SYSTEM: Spin Model
// /// -------------------------------------------------------------------
// fn spin_model(
//     time: Res<Time>,
//     mut query: Query<&mut Transform, With<Mesh>>,
// ) {
//     for mut transform in &mut query {
//         // Rotate 45 degrees per second around Y axis
//         transform.rotate_y(1.0 * time.delta_seconds());
//     }
// }
