use crate::gpu::Gpu;
use bevy::{
    prelude::*,
    utils::HashMap,
    window::{RawHandleWrapperHolder, WindowMode},
};
use raw_window_handle::RawWindowHandle;
use smallvec::SmallVec;
use std::hash::{Hash, Hasher};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D12::*,
            Dxgi::{
                Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
                *,
            },
        },
    },
};

pub const FRAME_COUNT: usize = 2;

#[derive(Resource, Deref)]
pub struct RenderTargetHeap(pub ID3D12DescriptorHeap);

#[derive(Deref, DerefMut, PartialEq, Eq, Default, Copy, Clone)]
pub struct HashableDescriptorHandle(pub D3D12_CPU_DESCRIPTOR_HANDLE);

impl Hash for HashableDescriptorHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderTargetStorage(pub HashMap<HashableDescriptorHandle, ID3D12Resource>);

#[derive(Component)]
pub struct WindowRenderTarget {
    pub swapchain: IDXGISwapChain4,
    pub rtv_handles: [HashableDescriptorHandle; FRAME_COUNT],
    frame_index: u32,
    pub viewport: D3D12_VIEWPORT,
}

pub fn create_render_targets(
    mut windows: Query<(Entity, &Window, &RawHandleWrapperHolder), Without<WindowRenderTarget>>,
    mut commands: Commands,
    gpu: Res<Gpu>,
    render_target_heap: Res<RenderTargetHeap>,
    mut render_target_storage: ResMut<RenderTargetStorage>,
) {
    for (entity, window, window_handle) in &mut windows {
        let swapchain_desc = create_swapchain_desc(window);
        let swapchain = create_swapchain(swapchain_desc, get_hwnd(window_handle), &gpu);
        let frame_index = unsafe { swapchain.GetCurrentBackBufferIndex() };
        let viewport = create_viewport(window);
        let rtv_handles = create_rtvs(
            &gpu.device,
            &swapchain,
            &render_target_heap,
            &mut render_target_storage,
        );

        commands.entity(entity).insert(WindowRenderTarget {
            swapchain,
            rtv_handles,
            frame_index,
            viewport,
        });
    }
}

pub fn resize_swapchains_if_needed(
    mut windows: Query<(&Window, &mut WindowRenderTarget)>,
    gpu: Res<Gpu>,
    render_target_heap: Res<RenderTargetHeap>,
    mut render_target_storage: ResMut<RenderTargetStorage>,
) {
    for (window, mut render_target) in &mut windows {
        let new_swapchain_desc = create_swapchain_desc(window);
        let old_swapchain_desc = unsafe { render_target.swapchain.GetDesc1() }.unwrap();
        if new_swapchain_desc == old_swapchain_desc {
            continue;
        }

        for handle in render_target.rtv_handles {
            render_target_storage.remove(&handle);
        }
        render_target.rtv_handles = [HashableDescriptorHandle::default(); FRAME_COUNT];

        unsafe {
            render_target.swapchain.ResizeBuffers(
                new_swapchain_desc.BufferCount,
                new_swapchain_desc.Width,
                new_swapchain_desc.Height,
                new_swapchain_desc.Format,
                DXGI_SWAP_CHAIN_FLAG(new_swapchain_desc.Flags as i32),
            )
        }
        .expect("ResizeBuffers failed");

        render_target.rtv_handles = create_rtvs(
            &gpu.device,
            &render_target.swapchain,
            &render_target_heap,
            &mut render_target_storage,
        );
        render_target.frame_index = unsafe { render_target.swapchain.GetCurrentBackBufferIndex() };
    }
}

fn create_swapchain_desc(window: &Window) -> DXGI_SWAP_CHAIN_DESC1 {
    DXGI_SWAP_CHAIN_DESC1 {
        Width: window.physical_width(),
        Height: window.physical_height(),
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: FRAME_COUNT as u32,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        Flags: DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0 as u32,
        ..Default::default()
    }
}

fn create_swapchain(desc: DXGI_SWAP_CHAIN_DESC1, hwnd: HWND, gpu: &Gpu) -> IDXGISwapChain4 {
    unsafe {
        gpu.factory.CreateSwapChainForHwnd(
            &gpu.queue, hwnd, &desc, None, // ??
            None,
        )
    }
    .expect("failed to create swapchain")
    .cast::<IDXGISwapChain4>()
    .expect("failed to cast swapchain to IDXGISwapChain4")
}

fn create_rtvs(
    device: &ID3D12Device9,
    swapchain: &IDXGISwapChain4,
    rtv_heap: &RenderTargetHeap,
    render_targets: &mut RenderTargetStorage,
) -> [HashableDescriptorHandle; FRAME_COUNT] {
    let mut handles = [HashableDescriptorHandle::default(); FRAME_COUNT];

    let heap_increment =
        unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) } as usize;
    let mut handle = unsafe { rtv_heap.GetCPUDescriptorHandleForHeapStart() };

    (0..FRAME_COUNT).for_each(|i| {
        let rtv = unsafe { swapchain.GetBuffer::<ID3D12Resource>(i as u32) }.unwrap();
        unsafe { device.CreateRenderTargetView(&rtv, None, handle) };
        let hashable_handle = HashableDescriptorHandle(handle);

        render_targets.insert(hashable_handle, rtv);
        handles[i] = hashable_handle;

        handle.ptr += heap_increment;
    });

    handles
}

fn get_hwnd(window_handle: &RawHandleWrapperHolder) -> HWND {
    match window_handle
        .0
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .window_handle
    {
        RawWindowHandle::Win32(window_handle) => {
            HWND(window_handle.hwnd.get() as *mut core::ffi::c_void)
        }
        _ => unreachable!(),
    }
}

fn create_viewport(window: &Window) -> D3D12_VIEWPORT {
    D3D12_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: window.physical_width() as f32,
        Height: window.physical_height() as f32,
        MinDepth: D3D12_MIN_DEPTH,
        MaxDepth: D3D12_MAX_DEPTH,
    }
}
