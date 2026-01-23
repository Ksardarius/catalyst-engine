use flecs_ecs::prelude::*;

#[derive(Component)]
pub struct PhaseRender3D;

#[derive(Component)]
pub struct PhaseRenderGUI;

#[derive(Component)]
pub struct PhasePresent;

// PHYSICS
#[derive(Component)]
pub struct PhysicsPipelinePhase;

#[derive(Component)]
pub struct PhysicsPipeline;

#[derive(Component)]
pub struct PhysicsPrepare;

#[derive(Component)]
pub struct PhysicsStep;

#[derive(Component)]
pub struct PhysicsSync;

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

    world
        .pipeline_type::<PhysicsPipeline>()
        .with(flecs::system::System)
        .with(PhysicsPipelinePhase)
        .cascade_id(flecs::DependsOn)
        .without(flecs::Disabled)
        .up_id(flecs::DependsOn)
        .without(flecs::Disabled)
        .up_id(flecs::ChildOf)
        .build();

    world
        .component::<PhysicsPrepare>()
        .add(PhysicsPipelinePhase);

    // PhysicsStep depends on Prepare
    world
        .component::<PhysicsStep>()
        .add(PhysicsPipelinePhase)
        .depends_on(PhysicsPrepare);

    // PhysicsSync depends on Step
    world
        .component::<PhysicsSync>()
        .add(PhysicsPipelinePhase)
        .depends_on(PhysicsStep);
}
