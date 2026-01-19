use catalyst_renderer::RenderContext;
use flecs_ecs::prelude::*;
use egui_wgpu::RendererOptions;
use winit::window::Window;

#[derive(Component)]
pub struct EguiState {
    pub context: egui::Context,
    pub state: egui_winit::State,
    pub renderer: egui_wgpu::Renderer,
}

impl EguiState {
    pub fn new(render_context: &RenderContext, window: &Window) -> Self {
        let context = egui::Context::default();

        let viewport_id = context.viewport_id();
        let state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        // 2. Create the WGPU Renderer (Handles drawing triangles)
        let renderer = egui_wgpu::Renderer::new(
            &render_context.device,
            render_context.config.format, // Output format (usually Bgra8Unorm)
            RendererOptions::PREDICTABLE, // Depth format (None for UI usually)                    // MSAA samples
        );

        Self {
            context,
            state,
            renderer,
        }
    }
}
