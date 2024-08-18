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

use crate::{
    gpu::Gpu,
    pipeline::{Pipelines, THE_ONLY_PIPELINE},
    render_target::WindowRenderTarget,
    triangle::{Triangle, TriangleVertexBuffers},
};

#[derive(Resource)]
pub struct Renderer {
    command_list: ID3D12GraphicsCommandList,
}

impl Renderer {
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

pub fn render(
    pipelines: Res<Pipelines>,
    gpu: Res<Gpu>,
    triangles: Query<Entity, With<Triangle>>,
    vertex_buffers: Res<TriangleVertexBuffers>,
    mut render_targets: Query<&mut WindowRenderTarget>,
    mut renderer: ResMut<Renderer>,
) {
    if render_targets.is_empty() {
        return;
    }

    let pipeline = pipelines.storage.get(&THE_ONLY_PIPELINE).unwrap();
    // ?????
    unsafe {
        gpu.command_allocator.Reset().unwrap();
    }
    // ?????
    unsafe {
        renderer
            .command_list
            .Reset(&gpu.command_allocator, pipeline.state.as_ref())
            .unwrap();
    }

    for mut render_target in render_targets.iter_mut() {
        unsafe {
            renderer
                .command_list
                .RSSetViewports(&[render_target.viewport]);
            renderer
                .command_list
                .RSSetScissorRects(&[render_target.rect]);
        }

        let back_buffer = render_target.back_buffer();
        let barrier = transition_barrier(
            back_buffer,
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        unsafe { renderer.command_list.ResourceBarrier(&[barrier]) };

        let rtv_handle = render_target.back_buffer_handle();
        unsafe {
            renderer
                .command_list
                .OMSetRenderTargets(1, Some(&rtv_handle), false, None)
        };

        unsafe {
            renderer.command_list.ClearRenderTargetView(
                render_target.back_buffer_handle(),
                &[0.0_f32, 0.2_f32, 0.4_f32, 1.0_f32],
                None,
            );
        }

        for triangle in triangles.iter() {
            let vertex_buffer = vertex_buffers.get(&triangle).unwrap();
            pipeline.populate_command_list(&mut renderer.command_list, vertex_buffer);
        }

        unsafe {
            renderer.command_list.ResourceBarrier(&[transition_barrier(
                back_buffer,
                D3D12_RESOURCE_STATE_RENDER_TARGET,
                D3D12_RESOURCE_STATE_PRESENT,
            )]);
        }

        unsafe {
            renderer
                .command_list
                .Close()
                .expect("Failed to close command list");
        }

        let command_list = renderer.command_list.cast().ok();
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
