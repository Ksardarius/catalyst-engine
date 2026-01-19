use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct PhaseRender3D;

#[derive(Component)]
pub struct PhaseRenderGUI;

#[derive(Component)]
pub struct PhasePresent;

pub fn define_pipeline_stages(world: &mut World) {
    world
        .component::<PhaseRender3D>()
        .add(flecs::Phase)
        .add(flecs::pipeline::OnStore);
    world
        .component::<PhaseRenderGUI>()
        .add(flecs::Phase)
        .depends_on(PhaseRender3D);
    world
        .component::<PhasePresent>()
        .add(flecs::Phase)
        .depends_on(PhaseRenderGUI);
}
