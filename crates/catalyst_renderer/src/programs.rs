pub mod debug_lines_program;
pub mod pbr_program;

pub use pbr_program::PbrProgram;
pub use debug_lines_program::DebugLinesProgram;

/// Holds common WGPU references to simplify function signatures.
pub struct GpuProgramRenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub format: wgpu::TextureFormat, // The output format (Swapchain or HDR)
}

pub trait GpuProgram {
    /// Data required to initialize the pipeline (e.g., global layouts)
    type InitData;
    
    /// Data required to draw a frame (e.g., Camera, List of Entities)
    type DrawData<'a> where Self: 'a;

    /// 1. INIT: Compiles shaders, creates pipeline layouts and the pipeline itself.
    fn new(ctx: &GpuProgramRenderContext, init_data: &Self::InitData) -> Self;

    /// 2. RECORD: Encodes commands into the RenderPass.
    /// This effectively replaces "use_program" + "draw".
    fn record<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, data: Self::DrawData<'a>);
}
