// Component to track a heavy calculation
use catalyst_core::*;
use catalyst_window::{WindowPlugin, run_catalyst_app};

#[derive(Component)]
struct MathJob;

fn main() {
    let mut app = App::new();

    app.add_plugin(WindowPlugin);

    // 1. SETUP: Define reactions
    app.add_system(handle_asset_loaded);

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

