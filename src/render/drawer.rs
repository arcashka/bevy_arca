use bevy::prelude::*;
use windows::{
    core::Interface,
    Win32::Graphics::{
        Direct3D12::{
            ID3D12GraphicsCommandList, ID3D12Resource, D3D12_COMMAND_LIST_TYPE_DIRECT,
            D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
            D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_STATES,
            D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_TRANSITION_BARRIER,
        },
        Dxgi::DXGI_PRESENT,
    },
};

use crate::core::Camera;

use super::{gpu::Gpu, pipeline::PipelineStorage, render_target::WindowRenderTarget};

#[derive(Resource)]
pub struct Drawer {
    command_list: ID3D12GraphicsCommandList,
}

impl Drawer {
    pub fn new(gpu: &Gpu) -> Self {
        let command_list: ID3D12GraphicsCommandList = unsafe {
            gpu.device.CreateCommandList(
                0,
                D3D12_COMMAND_LIST_TYPE_DIRECT,
                &gpu.command_allocator,
                None,
            )
        }
        .expect("CreateCommandList failed");
        unsafe {
            command_list.Close().expect("Failed to close command list");
        };

        Self { command_list }
    }
}

pub fn draw<const PIPELINE_ID: usize>(
    mut pipelines: ResMut<PipelineStorage>,
    gpu: Res<Gpu>,
    cameras: Query<(&Camera, &GlobalTransform, &Transform)>,
    mut render_targets: Query<&mut WindowRenderTarget>,
    mut drawer: ResMut<Drawer>,
) {
    if render_targets.is_empty() {
        return;
    }

    let pipeline = pipelines.get_mut(&PIPELINE_ID);
    if pipeline.is_none() {
        return;
    }
    let pipeline = pipeline.unwrap();

    unsafe {
        gpu.command_allocator.Reset().unwrap();
        drawer
            .command_list
            .Reset(&gpu.command_allocator, pipeline.state())
            .unwrap();
    }

    let (camera_settings, camera_global_transform, camera_transform) = cameras
        .get_single()
        .expect("only 1 camera is supported right now");
    for mut render_target in render_targets.iter_mut() {
        unsafe {
            drawer
                .command_list
                .RSSetViewports(&[render_target.viewport]);
            drawer.command_list.RSSetScissorRects(&[render_target.rect]);
        }

        let back_buffer = render_target.back_buffer();
        let barrier = transition_barrier(
            back_buffer,
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        unsafe { drawer.command_list.ResourceBarrier(&[barrier]) };

        let rtv_handle = render_target.back_buffer_handle();
        unsafe {
            drawer
                .command_list
                .OMSetRenderTargets(1, Some(&rtv_handle), false, None)
        };

        unsafe {
            drawer.command_list.ClearRenderTargetView(
                render_target.back_buffer_handle(),
                &[0.0_f32, 0.2_f32, 0.4_f32, 1.0_f32],
                None,
            );
        }

        pipeline.write_camera_data(&camera_global_transform, &camera_settings);
        pipeline.populate_command_list(&mut drawer.command_list);

        unsafe {
            drawer.command_list.ResourceBarrier(&[transition_barrier(
                back_buffer,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_PRESENT,
            )]);
        }

        unsafe {
            drawer
                .command_list
                .Close()
                .expect("Failed to close command list");
        }

        let command_list = drawer.command_list.cast().ok();
        unsafe { gpu.queue.ExecuteCommandLists(&[command_list]) };

        unsafe { render_target.swapchain.Present(1, DXGI_PRESENT(0)) }
            .ok()
            .unwrap();
        render_target.signal_end_present(&gpu.queue);
    }
}

fn transition_barrier(
    resource: &ID3D12Resource,
    state_before: D3D12_RESOURCE_STATES,
    state_after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: unsafe { std::mem::transmute_copy(resource) },
                StateBefore: state_before,
                StateAfter: state_after,
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            }),
        },
    }
}
