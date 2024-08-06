use crate::gpu::Gpu;
use bevy::{
    prelude::*,
    window::{RawHandleWrapperHolder, WindowMode},
};
use raw_window_handle::RawWindowHandle;
use smallvec::SmallVec;
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

#[derive(Component)]
pub struct WindowRenderTarget {
    pub swapchain: IDXGISwapChain4,
    pub rtv_heap: ID3D12DescriptorHeap,
    // remove Option here??
    pub rtvs: Option<[ID3D12Resource; FRAME_COUNT]>,
    pub rtv_handles: Option<[D3D12_CPU_DESCRIPTOR_HANDLE; FRAME_COUNT]>,
    pub frame_index: u32,
}

pub fn create_render_targets(
    mut windows: Query<(Entity, &Window, &RawHandleWrapperHolder), Without<WindowRenderTarget>>,
    mut commands: Commands,
    gpu: Res<Gpu>,
) {
    for (entity, window, window_handle) in &mut windows {
        if !matches!(
            window.mode,
            WindowMode::Windowed | WindowMode::BorderlessFullscreen(_)
        ) {
            panic!(
                "WindowMode must be Windowed or BorderlessFullscreen, was {:?}",
                window.mode
            );
        }

        let swapchain_desc = swapchain_desc(window);
        commands.entity(entity).insert(create_window_render_target(
            &gpu,
            &swapchain_desc,
            window_handle,
        ));
    }
}

pub fn resize_swapchains_if_needed(
    mut windows: Query<(&Window, &mut WindowRenderTarget)>,
    gpu: Res<Gpu>,
) {
    for (window, mut render_target) in &mut windows {
        let new_swapchain_desc = swapchain_desc(window);
        let old_swapchain_desc = unsafe { render_target.swapchain.GetDesc1() }.unwrap();
        if new_swapchain_desc == old_swapchain_desc {
            continue;
        }

        render_target.rtvs = None;
        render_target.rtv_handles = None;

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

        let (rtvs, rtv_handles) = create_rtvs(
            &gpu.device,
            &render_target.swapchain,
            &render_target.rtv_heap,
        );
        render_target.rtvs = Some(rtvs);
        render_target.rtv_handles = Some(rtv_handles);
        render_target.frame_index = unsafe { render_target.swapchain.GetCurrentBackBufferIndex() };
    }
}

fn swapchain_desc(window: &Window) -> DXGI_SWAP_CHAIN_DESC1 {
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

fn create_window_render_target(
    gpu: &Gpu,
    swapchain_desc: &DXGI_SWAP_CHAIN_DESC1,
    window_handle: &RawHandleWrapperHolder,
) -> WindowRenderTarget {
    let swapchain = unsafe {
        gpu.factory.CreateSwapChainForHwnd(
            &gpu.queue,
            get_hwnd(window_handle),
            swapchain_desc,
            None,
            None,
        )
    }
    .unwrap()
    .cast::<IDXGISwapChain4>()
    .unwrap();

    unsafe { swapchain.SetMaximumFrameLatency(1).unwrap() };
    let rtv_heap = unsafe {
        gpu.device
            .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                NumDescriptors: FRAME_COUNT as u32,
                ..Default::default()
            })
    }
    .unwrap();
    let (rtvs, rtv_handles) = create_rtvs(&gpu.device, &swapchain, &rtv_heap);

    let frame_index = unsafe { swapchain.GetCurrentBackBufferIndex() };

    WindowRenderTarget {
        swapchain,
        rtv_heap,
        rtvs: Some(rtvs),
        rtv_handles: Some(rtv_handles),
        frame_index,
    }
}

fn create_rtvs(
    device: &ID3D12Device9,
    swapchain: &IDXGISwapChain4,
    rtv_heap: &ID3D12DescriptorHeap,
) -> (
    [ID3D12Resource; FRAME_COUNT],
    [D3D12_CPU_DESCRIPTOR_HANDLE; FRAME_COUNT],
) {
    let mut rtvs = SmallVec::with_capacity(FRAME_COUNT);
    let mut handles = [D3D12_CPU_DESCRIPTOR_HANDLE::default(); FRAME_COUNT];

    let heap_increment =
        unsafe { device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) } as usize;
    let mut handle = unsafe { rtv_heap.GetCPUDescriptorHandleForHeapStart() };

    (0..FRAME_COUNT).for_each(|i| {
        let rtv = unsafe { swapchain.GetBuffer::<ID3D12Resource>(i as u32) }.unwrap();
        unsafe { device.CreateRenderTargetView(&rtv, None, handle) };

        rtvs.push(rtv);
        handles[i] = handle;

        handle.ptr += heap_increment;
    });

    (rtvs.into_inner().unwrap(), handles)
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
