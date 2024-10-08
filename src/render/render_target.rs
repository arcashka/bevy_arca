use bevy::{prelude::*, window::RawHandleWrapperHolder};

use raw_window_handle::RawWindowHandle;
use smallvec::SmallVec;
use windows::{
    core::Interface,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct3D12::*,
            Dxgi::{
                Common::{DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
                *,
            },
        },
        System::Threading::{CreateEventA, WaitForSingleObject, INFINITE},
    },
};

use super::{gpu::Gpu, DescriptorHeap, ResizeEvent};
use crate::win_types::WinHandle;

pub const FRAME_COUNT: usize = 2;

struct Fence {
    fence: ID3D12Fence,
    fence_value: u64,
    fence_event: WinHandle,
}

#[derive(Component)]
pub struct WindowRenderTarget {
    pub swapchain: IDXGISwapChain4,
    rtvs: SmallVec<[ID3D12Resource; FRAME_COUNT]>,
    rtv_handles: SmallVec<[D3D12_CPU_DESCRIPTOR_HANDLE; FRAME_COUNT]>,
    swapchain_buffer_index: u32,
    fence: Fence,
    pub viewport: D3D12_VIEWPORT,
    pub rect: RECT,
}

#[derive(Resource, Deref, DerefMut)]
pub struct RtvHeap(pub DescriptorHeap);

pub fn create_render_targets(
    mut windows: Query<(Entity, &Window, &RawHandleWrapperHolder), Without<WindowRenderTarget>>,
    mut commands: Commands,
    mut rtv_heap: ResMut<RtvHeap>,
    gpu: Res<Gpu>,
    mut resize_events: EventWriter<ResizeEvent>,
) {
    for (entity, window, window_handle) in &mut windows {
        commands.entity(entity).insert(WindowRenderTarget::new(
            window,
            window_handle,
            &gpu,
            &mut rtv_heap,
        ));
        resize_events.send(ResizeEvent {
            entity,
            width: window.width(),
            height: window.height(),
        });
    }
}

pub fn switch_frame(
    mut windows: Query<(&Window, &mut WindowRenderTarget, Entity)>,
    gpu: Res<Gpu>,
    mut resize_events: EventWriter<ResizeEvent>,
) {
    for (window, mut render_target, entity) in &mut windows {
        render_target.wait_frame_finished();
        let new_swapchain_desc = create_swapchain_desc(window);
        let old_swapchain_desc = unsafe { render_target.swapchain.GetDesc1() }.unwrap();
        if new_swapchain_desc != old_swapchain_desc {
            render_target.handle_resize(
                &gpu.device,
                new_swapchain_desc,
                window.width(),
                window.height(),
            );
            resize_events.send(ResizeEvent {
                entity,
                width: window.width(),
                height: window.height(),
            });
        }
        render_target.update_frame_index();
    }
}

impl WindowRenderTarget {
    fn new(
        window: &Window,
        window_handle: &RawHandleWrapperHolder,
        gpu: &Gpu,
        rtv_heap: &mut DescriptorHeap,
    ) -> Self {
        let desc = create_swapchain_desc(window);
        let swapchain = unsafe {
            gpu.factory.CreateSwapChainForHwnd(
                &gpu.queue,
                get_hwnd(window_handle),
                &desc,
                None, // ??
                None,
            )
        }
        .expect("failed to create swapchain")
        .cast::<IDXGISwapChain4>()
        .expect("failed to cast swapchain to IDXGISwapChain4");

        let frame_index = unsafe { swapchain.GetCurrentBackBufferIndex() };
        let viewport = create_viewport(window.width(), window.height());
        let rect = create_rect(window.width() as i32, window.height() as i32);
        let fence = create_fence(gpu);

        let mut window_render_target = WindowRenderTarget {
            swapchain,
            rtvs: SmallVec::new(),
            rtv_handles: SmallVec::new(),
            swapchain_buffer_index: frame_index,
            fence,
            viewport,
            rect,
        };

        window_render_target.create_descriptors(rtv_heap);
        window_render_target.create_rtvs(&gpu.device);
        window_render_target
    }

    pub fn back_buffer(&self) -> &ID3D12Resource {
        &self.rtvs[self.swapchain_buffer_index as usize]
    }

    pub fn back_buffer_handle(&self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        self.rtv_handles[self.swapchain_buffer_index as usize]
    }

    // TODO: can i not have queue here?
    pub fn signal_end_present(&mut self, queue: &ID3D12CommandQueue) {
        unsafe {
            queue
                .Signal(&self.fence.fence, self.fence.fence_value)
                .expect("Signal Fence failed");
        }
        self.fence.fence_value += 1;
    }

    fn update_frame_index(&mut self) {
        self.swapchain_buffer_index = unsafe { self.swapchain.GetCurrentBackBufferIndex() };
    }

    fn wait_frame_finished(&mut self) {
        let previous_fence_value = self.fence.fence_value - 1;
        if unsafe { self.fence.fence.GetCompletedValue() } < previous_fence_value {
            unsafe {
                self.fence
                    .fence
                    .SetEventOnCompletion(previous_fence_value, self.fence.fence_event.0)
            }
            .ok()
            .unwrap();

            unsafe { WaitForSingleObject(self.fence.fence_event.0, INFINITE) };
        }
    }

    fn create_descriptors(&mut self, rtv_heap: &mut DescriptorHeap) {
        for _ in 0..FRAME_COUNT {
            self.rtv_handles.push(rtv_heap.cpu_handle());
        }
    }

    fn create_rtvs(&mut self, device: &ID3D12Device9) {
        (0..FRAME_COUNT).for_each(|i| {
            let rtv = unsafe { self.swapchain.GetBuffer::<ID3D12Resource>(i as u32) }.unwrap();
            unsafe { device.CreateRenderTargetView(&rtv, None, self.rtv_handles[i]) };

            if self.rtvs.len() == i {
                self.rtvs.push(rtv);
            } else {
                self.rtvs[i] = rtv;
            }
        });
    }

    fn handle_resize(
        &mut self,
        device: &ID3D12Device9,
        desc: DXGI_SWAP_CHAIN_DESC1,
        width: f32,
        height: f32,
    ) {
        self.destroy_resources();

        unsafe {
            self.swapchain.ResizeBuffers(
                desc.BufferCount,
                desc.Width,
                desc.Height,
                desc.Format,
                DXGI_SWAP_CHAIN_FLAG(desc.Flags as i32),
            )
        }
        .expect("ResizeBuffers failed");

        self.viewport = create_viewport(width, height);
        self.rect = create_rect(width as i32, height as i32);

        self.create_rtvs(device);
    }

    fn destroy_resources(&mut self) {
        self.rtvs.clear();
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

fn create_viewport(width: f32, height: f32) -> D3D12_VIEWPORT {
    D3D12_VIEWPORT {
        TopLeftX: 0.0,
        TopLeftY: 0.0,
        Width: width,
        Height: height,
        MinDepth: D3D12_MIN_DEPTH,
        MaxDepth: D3D12_MAX_DEPTH,
    }
}

fn create_rect(width: i32, height: i32) -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    }
}

fn create_fence(gpu: &Gpu) -> Fence {
    let fence = unsafe { gpu.device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }
        .expect("failed to create fence");
    let fence_value = 0;
    let fence_event =
        unsafe { CreateEventA(None, false, false, None).expect("Failed to create event") };

    Fence {
        fence,
        fence_value,
        fence_event: WinHandle(fence_event),
    }
}
