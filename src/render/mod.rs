mod constant_buffer;
mod descriptor_heap;
mod drawer;
mod gpu;
mod mesh_data;
mod pipelines;
mod render_target;

use bevy::{app::MainScheduleOrder, ecs::schedule::ScheduleLabel, prelude::*};

use drawer::draw;
use pipelines::{
    create_pathtracer_pipeline, PathTracerShaderHandle, PipelineStorage, PATH_TRACER_PIPELINE_ID,
};
use render_target::{create_render_targets, switch_frame, RtvHeap, FRAME_COUNT};

pub use descriptor_heap::DescriptorHeap;
pub use drawer::Drawer;
pub use gpu::Gpu;
pub use mesh_data::{MeshBuffer, MeshData};
use windows::Win32::Graphics::Direct3D12::{
    D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
};

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_schedule(RenderSchedule);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Last, RenderSchedule);

        let gpu = unsafe { Gpu::new(false) }.expect("Failed to initialize renderer");
        let drawer = Drawer::new(&gpu);

        let asset_server = app.world_mut().resource_mut::<AssetServer>();
        let shader_handle = asset_server.load("demo.hlsl");
        let rtv_heap = DescriptorHeap::new(
            &gpu,
            D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            FRAME_COUNT,
            D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
        );

        app.insert_resource(gpu)
            .insert_resource(PathTracerShaderHandle(shader_handle))
            .insert_resource(drawer)
            .insert_resource(PipelineStorage::new())
            .insert_resource(RtvHeap(rtv_heap))
            .add_event::<ResizeEvent>()
            .add_systems(
                RenderSchedule,
                (
                    create_render_targets,
                    create_pathtracer_pipeline,
                    draw::<PATH_TRACER_PIPELINE_ID>,
                    switch_frame,
                )
                    .chain(),
            );
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenderSchedule;

#[derive(Event)]
pub struct ResizeEvent {
    pub entity: Entity,
    pub width: f32,
    pub height: f32,
}
