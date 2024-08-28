pub mod gltf;
pub mod gpu;
pub mod image;
pub mod material;
pub mod mesh;
pub mod pipeline;
pub mod render_target;
pub mod renderer;
pub mod triangle;
pub mod win_types;

use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};

use gltf::GltfPlugin;
use gpu::Gpu;
use image::ImagePlugin;
use material::MaterialPlugin;
use mesh::MeshPlugin;
use pipeline::{create_pipeline_state, create_root_signature, Pipelines};
use render_target::{create_render_targets, resize_swapchains_if_needed, RenderTargetHeap};
use renderer::{render, Renderer};
use triangle::{create_vertex_buffers, Triangle, TriangleVertexBuffers};

pub struct ArcaPlugin;

impl Plugin for ArcaPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(Render);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, Render);

        let gpu = unsafe { Gpu::new(false) }.expect("Failed to initialize renderer");
        let render_target_heap = RenderTargetHeap::new(&gpu);
        let renderer = Renderer::new(&gpu);

        app.add_plugins((
            DefaultPlugins,
            GltfPlugin,
            MaterialPlugin,
            ImagePlugin,
            MeshPlugin,
        ));

        app.insert_resource(gpu)
            .insert_resource(Pipelines::new())
            .insert_resource(TriangleVertexBuffers::default())
            .insert_resource(render_target_heap)
            .insert_resource(renderer)
            .add_systems(Startup, create_triangle)
            .add_systems(
                Render,
                (
                    create_render_targets,
                    create_root_signature,
                    create_pipeline_state,
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
