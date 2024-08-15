mod gpu;
mod pipeline;
mod swapchain;
mod triangle;

use crate::{gpu::Gpu, swapchain::create_render_targets};
use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};
use pipeline::{
    create_command_list, create_pipeline_state, create_root_signature, render, Pipelines,
};
use swapchain::{resize_swapchains_if_needed, switch_swapchain_buffers, RenderTargetHeap};
use triangle::{create_vertex_buffers, Triangle, TriangleVertexBuffers};

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(Render);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, Render);

        let gpu = unsafe { Gpu::new(false) }.expect("Failed to initialize renderer");
        let render_target_heap = RenderTargetHeap::new(&gpu);

        app.insert_resource(gpu)
            .insert_resource(Pipelines::new())
            .insert_resource(TriangleVertexBuffers::default())
            .insert_resource(render_target_heap)
            .add_systems(Startup, create_triangle)
            .add_systems(
                Render,
                (
                    create_render_targets,
                    create_root_signature,
                    create_pipeline_state,
                    create_command_list,
                    create_vertex_buffers, // TODO: separate
                    render,
                    resize_swapchains_if_needed,
                    switch_swapchain_buffers,
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
