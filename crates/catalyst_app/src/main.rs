use catalyst_assets::{AssetPlugin, asset_server::AssetServer, assets::Handle, scene::SceneData};
// Component to track a heavy calculation
use catalyst_core::{camera::Camera, time::Time, transform::Transform, *};
use catalyst_debug::DebugPlugin;
use catalyst_renderer::{
    RenderPlugin,
    mesh::{Mesh},
};
use catalyst_scene::ScenePlugin;
use catalyst_window::{WindowPlugin, run_catalyst_app};
use glam::{Quat, Vec3};

#[derive(Component)]
struct MathJob;

fn main() {
    let mut app = App::new();

    app.add_plugin(WindowPlugin);
    app.add_plugin(AssetPlugin);
    app.add_plugin(ScenePlugin);
    app.add_plugin(RenderPlugin);
    app.add_plugin(DebugPlugin);

    // Spawn the Triangle
    app.add_startup_system(setup_scene);

    app.add_system(spin_model);
    app.add_system(move_camera);

    run_catalyst_app(app)
}

#[derive(Component)]
pub struct SceneRoot(pub Handle<SceneData>);

/// -------------------------------------------------------------------
/// SYSTEM: Setup Scene
/// -------------------------------------------------------------------
fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Requesting Mesh Load...");
    // let mesh_handle = asset_server.load_mesh("assets/monkey.obj");

    // B. Spawn the Mesh Entity
    // commands.spawn((
    //     Mesh(mesh_handle),       // The Render Component
    //     Transform::from_xyz(0.0, 0.0, 0.0), // Position at center
    // ));

    let scene_handle = asset_server.load_scene("assets/scene2.glb");
    commands.spawn(catalyst_scene::SceneRoot(scene_handle));

    // commands.spawn((
    //     SceneRoot(asset_server.load_scene("assets/scene1.glb")),
    //     Transform::default(),
    // ));

    // C. Spawn the Camera
    commands.spawn((
        Camera::default(),
        Transform::from_xyz(0.0, 2.0, 5.0) // Up 2, Back 5
            .looking_at(Vec3::ZERO, Vec3::Y), // Look at center
    ));
}

fn rotate_triangle(mut query: Query<(&mut Transform, &Mesh)>) {
    for (mut transform, _) in &mut query {
        // Rotate around Z axis (Spinning)
        transform.rotation *= Quat::from_rotation_y(0.02);
    }
}

// A simple Fly Cam
fn move_camera(
    time: Res<Time>, // <--- Request Time
    input: Res<Input>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    let speed = 5.0 * time.delta_seconds();

    if let Ok(mut transform) = query.single_mut() {
        let forward = transform.rotation * -Vec3::Z;
        let right = transform.rotation * Vec3::X;
        let up = Vec3::Y; // Global Up

        if input.is_pressed(KeyCode::KeyW) {
            transform.translation += forward * speed;
        }
        if input.is_pressed(KeyCode::KeyS) {
            transform.translation -= forward * speed;
        }
        if input.is_pressed(KeyCode::KeyA) {
            transform.translation -= right * speed;
        }
        if input.is_pressed(KeyCode::KeyD) {
            transform.translation += right * speed;
        }
        if input.is_pressed(KeyCode::Space) {
            transform.translation += up * speed;
        }
        if input.is_pressed(KeyCode::ShiftLeft) {
            transform.translation -= up * speed;
        }
    }
}

/// -------------------------------------------------------------------
/// SYSTEM: Spin Model
/// -------------------------------------------------------------------
fn spin_model(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Mesh>>,
) {
    for mut transform in &mut query {
        // Rotate 45 degrees per second around Y axis
        transform.rotate_y(1.0 * time.delta_seconds());
    }
}
