use catalyst_core::{App, Plugin};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
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
        self.app.world.non_send_resource::<MainWindow>().0.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.app.update();
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                // self.0.as_ref().unwrap().request_redraw();
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
