mod gpu;
mod pipeline;
mod swapchain;
mod triangle;

use crate::{gpu::Gpu, swapchain::create_render_targets};
use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};
use pipeline::{
    create_command_list, create_fence, create_pipeline_state, create_root_signature, render,
    Pipelines,
};
use swapchain::resize_swapchains_if_needed;
use triangle::{create_vertex_buffers, Triangle, TriangleVertexBuffers};

pub struct RtxPlugin;

impl Plugin for RtxPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(Render);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, Render);

        let gpu = unsafe { Gpu::new(false) }.expect("Failed to initialize renderer");

        app.insert_resource(gpu)
            .insert_resource(Pipelines::new())
            .insert_resource(TriangleVertexBuffers::default())
            .add_systems(Startup, create_triangle)
            .add_systems(
                Render,
                (
                    create_render_targets,
                    create_root_signature,
                    create_pipeline_state,
                    create_command_list,
                    create_fence,
                    create_vertex_buffers, // TODO: separate
                    render,
                    resize_swapchains_if_needed,
                )
                    .chain(),
            );
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct Render;

fn create_triangle(mut commands: Commands) {
    commands.spawn(Triangle);
}
