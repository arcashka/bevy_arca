mod drawer;
mod gpu;
mod pipeline;
mod render_target;

use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};

use drawer::draw;
use pipeline::{
    create_pathtracer_pipeline, PathTracerShaderHandle, PipelineStorage, PATH_TRACER_PIPELINE_ID,
};
use render_target::{create_render_targets, resize_swapchains_if_needed};

pub use drawer::Drawer;
pub use gpu::Gpu;
pub use render_target::RenderTargetHeap;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(RenderSchedule);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, RenderSchedule);

        let gpu = unsafe { Gpu::new(false) }.expect("Failed to initialize renderer");
        let drawer = Drawer::new(&gpu);
        let render_target_heap = RenderTargetHeap::new(&gpu);

        let asset_server = app.world_mut().resource_mut::<AssetServer>();
        let shader_handle = asset_server.load("demo.hlsl");

        app.insert_resource(gpu)
            .insert_resource(PathTracerShaderHandle(shader_handle))
            .insert_resource(render_target_heap)
            .insert_resource(drawer)
            .insert_resource(PipelineStorage::new())
            .add_systems(
                RenderSchedule,
                (
                    create_render_targets,
                    create_pathtracer_pipeline,
                    draw::<PATH_TRACER_PIPELINE_ID>,
                    resize_swapchains_if_needed,
                )
                    .chain(),
            );
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderSchedule;
