use catalyst_core::{
    App, Plugin, SystemEvents,
    pipeline::{PhysicsPipeline},
    time::{PhysicsTime, Time},
};
use catalyst_input::physical::{DeviceKind, InputState, MouseButtonId, PhysicalInputId};
use flecs_ecs::{
    core::{WorldGet, flecs, world},
    macros::Component,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{CursorGrabMode, Window},
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

impl CatalystRunner {
    fn run_physics_loop(&mut self) {
        let mut steps_to_run = 0;
        let mut fixed_dt = 0.0;
        let max_steps_per_frame = 4; // prevents spiral-of-death

        // --------------------------------------------------------- // Determine how many physics steps to run // ---------------------------------------------------------
        self.app.world.get::<&mut PhysicsTime>(|pt| {
            fixed_dt = pt.fixed_dt;
            while pt.accumulator >= fixed_dt {
                pt.accumulator -= fixed_dt;
                steps_to_run += 1;
                if steps_to_run >= max_steps_per_frame {
                    // Clamp to avoid runaway catch-up
                    break;
                }
            }
        });

        // --------------------------------------------------------- // Run physics steps // ---------------------------------------------------------
        for _ in 0..steps_to_run {
            self.run_physics_pipeline(fixed_dt);
        }
    }

    fn run_physics_pipeline(&mut self, dt: f32) {
        // let pipeline = self.app.world.lookup("physics_pipeline");
        self.app.world.run_pipeline_time(PhysicsPipeline, dt);
    }
}

impl ApplicationHandler for CatalystRunner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.app.world.set(MainWindow(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_inner_size(LogicalSize::new(1920, 1080))
                        .with_title("Catalyst Engine"),
                )
                .map(|w| {
                    w.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    w.set_cursor_visible(false);
                    w
                })
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

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                self.app.world.get::<&mut InputState>(|input_state| {
                    input_state.mouse_delta.0 += delta.0 as f32;
                    input_state.mouse_delta.1 += delta.1 as f32;
                });
            }
            _ => {}
        }
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

        // handle inputs
        self.app.world.try_get::<&mut InputState>(|input_state| {
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
                    let pid = PhysicalInputId {
                        device: DeviceKind::Keyboard(code as u16),
                    };

                    let pressed = state == winit::event::ElementState::Pressed;
                    input_state.physical_buttons.insert(pid, pressed);
                }
                WindowEvent::MouseInput {
                    state: btn_state,
                    button,
                    ..
                } => {
                    let pid = PhysicalInputId {
                        device: DeviceKind::MouseButton(to_mouse_button_id(button)),
                    };
                    let pressed = btn_state == winit::event::ElementState::Pressed;
                    input_state.physical_buttons.insert(pid, pressed);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // optional: store absolute position }
                    let new_x = position.x as f32;
                    let new_y = position.y as f32;
                    input_state.mouse_position = (new_x, new_y);
                }

                _ => (),
            }
        });

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let dt = self.app.world.get::<&mut Time>(|time| {
                    time.update();
                    time.delta_seconds()
                });

                // --------------------------------------------------------- // 2. Accumulate physics time // ---------------------------------------------------------
                self.app.world.get::<&mut PhysicsTime>(|pt| {
                    pt.accumulator += dt;
                });

                // --------------------------------------------------------- // 3. Run physics (fixed timestep) // ---------------------------------------------------------
                self.run_physics_loop();

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

fn to_mouse_button_id(button: winit::event::MouseButton) -> MouseButtonId {
    match button {
        winit::event::MouseButton::Left => MouseButtonId::Left,
        winit::event::MouseButton::Right => MouseButtonId::Right,
        winit::event::MouseButton::Middle => MouseButtonId::Middle,
        winit::event::MouseButton::Back => MouseButtonId::Back,
        winit::event::MouseButton::Forward => MouseButtonId::Forward,
        winit::event::MouseButton::Other(n) => MouseButtonId::Other(n),
    }
}
