use catalyst_core::{App, Plugin};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub struct MainWindow(pub Window);

// 1. Define a Wrapper Resource for the Window
// We use a wrapper because 'Window' is not Thread-Safe (Send),
// so it must be a "NonSend" resource in Bevy ECS terms.
// #[derive(Default)]
pub struct MainApp(App);

impl MainApp {
    pub fn new(app: App) -> Self {
        Self(app)
    }
}

pub struct WindowPlugin;

impl Plugin for WindowPlugin {
    fn build(&self, _app: &mut App) {
        // Nothing to register yet
    }
}

impl ApplicationHandler for MainApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.0.world.insert_non_send_resource(MainWindow(
            event_loop
                .create_window(Window::default_attributes().with_title("Catalyst Engine"))
                .unwrap(),
        ));
    }

    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.0.world.non_send_resource::<MainWindow>().0.request_redraw();
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
                self.0.update();
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

    app.startup();

    let mut main_window = MainApp::new(app);

    event_loop.run_app(&mut main_window).unwrap();
}
