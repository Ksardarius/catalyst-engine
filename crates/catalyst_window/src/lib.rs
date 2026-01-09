use catalyst_core::{App, Input, Plugin, SystemEvents, time::Time};
use flecs_ecs::{
    core::{WorldGet, flecs},
    macros::Component,
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

#[derive(Component)]
pub struct MainWindow(pub Window);

// The State Machine that holds the App while waiting for the OS
struct CatalystRunner {
    app: App,
    // We keep track if we have started the engine yet
    initialized: bool,
}

impl CatalystRunner {
    pub fn new(app: App) -> Self {
        Self {
            app,
            initialized: false,
        }
    }
}

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        // Nothing to register yet
        app.world
            .component::<MainWindow>()
            .add_trait::<flecs::Singleton>();
    }
}

impl ApplicationHandler for CatalystRunner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app.world.set(MainWindow(
            event_loop
                .create_window(Window::default_attributes().with_title("Catalyst Engine"))
                .unwrap(),
        ));

        if !self.initialized {
            self.app.startup();
            self.initialized = true;
        }
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app.world.get::<&mut MainWindow>(|w| {
            w.0.request_redraw();
        });
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        self.app.world.get::<&mut SystemEvents>(|events| {
            events.buffer.push(event.clone());
        });

        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                self.app.world.get::<&mut Input>(|input| match state {
                    ElementState::Pressed => input.press(code),
                    ElementState::Released => input.release(code),
                });
            }
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.app.world.get::<&mut Time>(|time| {
                    time.update();
                });

                // 2. Run the Systems
                self.app.update();

                self.app.world.try_get::<&mut SystemEvents>(|events| {
                    events.clear();
                });

                self.app.world.try_get::<&mut MainWindow>(|window_res| {
                    window_res.0.request_redraw();
                });
            }
            _ => (),
        }
    }
}

pub fn run_catalyst_app(mut app: App) {
    let event_loop = EventLoop::new().unwrap();

    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events. This is ideal for games and similar applications.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut main_window = CatalystRunner::new(app);

    event_loop.run_app(&mut main_window).unwrap();
}
