// Component to track a heavy calculation
use catalyst_core::{camera::Camera, transform::Transform, *};
use catalyst_renderer::{RenderPlugin, mesh::{Mesh, MeshData, Vertex}};
use catalyst_window::{WindowPlugin, run_catalyst_app};
use glam::Quat;

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
    // 1. Create the heavy data ONCE
    let triangle_data = MeshData {
        vertices: vec![
            Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
        ],
    };

    // 2. Add it to the bank, get a Handle
    let triangle_handle = mesh_assets.add(triangle_data);

    
    // 1. Spawn a Camera (Looking at 0,0,0)
    commands.spawn((
        Camera::default(),
        Transform::from_xyz(0.0, 0.0, 5.0), // Move back 5 units
    ));

    // 3. Spawn Entity 1 (Using the Handle)
    commands.spawn((
        Mesh(triangle_handle.clone()), // Cheap clone of ID
        Transform::from_xyz(-1.0, 0.0, 0.0), // Left
    ));

    // 4. Spawn Entity 2 (Sharing the SAME data!)
    commands.spawn((
        Mesh(triangle_handle), // Reuse handle
        Transform::from_xyz(1.0, 0.0, 0.0), // Right
    ));
}

fn rotate_triangle(mut query: Query<&mut Transform, With<Mesh>>) {
    for mut transform in &mut query {
        // Rotate around Z axis (Spinning)
        transform.rotation *= Quat::from_rotation_z(0.01);
    }
}

fn handle_asset_loaded(mut events: MessageReader<EngineEvent>) {
    for event in events.read() {
        match event {
            EngineEvent::AssetLoaded { name, data } => {
                println!("  [ECS System] WOW! Received '{}' size: {}", name, data.len());
            },
            _ => {}
        }
    }
}

