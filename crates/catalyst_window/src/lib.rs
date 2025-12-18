use catalyst_core::{App, Input, Plugin, time::Time};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

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
    fn build(&self, _app: &mut App) {
        // Nothing to register yet
    }
}

impl ApplicationHandler for CatalystRunner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app.world.insert_non_send_resource(MainWindow(
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
        self.app
            .world
            .non_send_resource::<MainWindow>()
            .0
            .request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
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
                let mut input = self.app.world.resource_mut::<Input>();

                match state {
                    ElementState::Pressed => input.press(code),
                    ElementState::Released => input.release(code),
                }
            }
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // 1. Tick the Clock manually
                if let Some(mut time) = self.app.world.get_resource_mut::<Time>() {
                    time.update();
                }

                // 2. Run the Systems
                self.app.update();

                // 3. Request next frame
                if let Some(window_res) = self.app.world.get_non_send_resource::<MainWindow>() {
                    window_res.0.request_redraw();
                }
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
