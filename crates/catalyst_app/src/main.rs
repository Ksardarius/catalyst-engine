// Component to track a heavy calculation
use catalyst_core::{camera::Camera, time::Time, transform::Transform, *};
use catalyst_renderer::{
    RenderPlugin,
    mesh::{Mesh, MeshData, Vertex},
};
use catalyst_window::{WindowPlugin, run_catalyst_app};
use glam::{Quat, Vec3};

#[derive(Component)]
struct MathJob;

fn main() {
    let mut app = App::new();

    app.add_plugin(WindowPlugin);
    app.add_plugin(RenderPlugin);

    // Spawn the Triangle
    app.add_startup_system(setup_3d_scene);

    // 1. SETUP: Define reactions
    app.add_system(handle_asset_loaded);
    app.add_system(rotate_triangle);
    app.add_system(move_camera);

    // 2. ACTION: Spawn Async Task
    println!("Main: Spawning Async Request...");
    app.spawn_io(|sender| async move {
        println!("    [Tokio] Downloading 'texture.png'...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Send the result back!
        let _ = sender.0.send(EngineEvent::AssetLoaded {
            name: "texture.png".to_string(),
            data: vec![0, 255, 0, 255],
        });
        println!("    [Tokio] Sent 'AssetLoaded' event.");
    });

    run_catalyst_app(app)

    // 3. LOOP
    // app.startup_schedule.run(&mut app.world);

    // for i in 0..10 {
    //     app.main_schedule.run(&mut app.world);
    //     std::thread::sleep(std::time::Duration::from_millis(200));
    // }
}

fn setup_3d_scene(mut commands: Commands, mut mesh_assets: ResMut<Assets<MeshData>>) {
    // Define a Quad (Rectangle) reusing vertices!
    let quad_handle = mesh_assets.add(MeshData {
        vertices: vec![
            // 0: Top Left (Red)
            Vertex {
                position: [-0.5, 0.5, 0.0],
                color: [1.0, 0.0, 0.0],
            },
            // 1: Top Right (Green)
            Vertex {
                position: [0.5, 0.5, 0.0],
                color: [0.0, 1.0, 0.0],
            },
            // 2: Bottom Left (Blue)
            Vertex {
                position: [-0.5, -0.5, 0.0],
                color: [0.0, 0.0, 1.0],
            },
            // 3: Bottom Right (Yellow)
            Vertex {
                position: [0.5, -0.5, 0.0],
                color: [1.0, 1.0, 0.0],
            },
        ],
        // The Magic: We define 2 triangles pointing to the 4 vertices above
        indices: vec![
            0, 1, 2, // Triangle 1 (Top-Left, Top-Right, Bottom-Left)
            1, 3, 2, // Triangle 2 (Top-Right, Bottom-Right, Bottom-Left)
        ],
    });

    commands.spawn((Camera::default(), Transform::from_xyz(0.0, 0.0, 3.0)));

    // Spawn the Quad
    commands.spawn((Mesh(quad_handle), Transform::default()));
}

fn rotate_triangle(mut query: Query<(&mut Transform, &Mesh)>) {
    for (mut transform, _) in &mut query {
        // Rotate around Z axis (Spinning)
        transform.rotation *= Quat::from_rotation_y(0.02);
    }
}

fn handle_asset_loaded(mut events: MessageReader<EngineEvent>) {
    for event in events.read() {
        match event {
            EngineEvent::AssetLoaded { name, data } => {
                println!(
                    "  [ECS System] WOW! Received '{}' size: {}",
                    name,
                    data.len()
                );
            }
            _ => {}
        }
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
            transform.position += forward * speed;
        }
        if input.is_pressed(KeyCode::KeyS) {
            transform.position -= forward * speed;
        }
        if input.is_pressed(KeyCode::KeyA) {
            transform.position -= right * speed;
        }
        if input.is_pressed(KeyCode::KeyD) {
            transform.position += right * speed;
        }
        if input.is_pressed(KeyCode::Space) {
            transform.position += up * speed;
        }
        if input.is_pressed(KeyCode::ShiftLeft) {
            transform.position -= up * speed;
        }
    }
}
