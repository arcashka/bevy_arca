use windows::Win32::Graphics::Direct3D12::{
    ID3D12DescriptorHeap, D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_DESCRIPTOR_HEAP_DESC,
    D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_TYPE, D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
    D3D12_GPU_DESCRIPTOR_HANDLE,
};

use super::Gpu;

pub struct DescriptorHeap {
    heap: ID3D12DescriptorHeap,
    current_ptr: D3D12_CPU_DESCRIPTOR_HANDLE,
    heap_increment: usize,
}

impl DescriptorHeap {
    pub fn new(
        gpu: &Gpu,
        heap_type: D3D12_DESCRIPTOR_HEAP_TYPE,
        descriptor_count: usize,
        flags: D3D12_DESCRIPTOR_HEAP_FLAGS,
    ) -> Self {
        let heap: ID3D12DescriptorHeap = unsafe {
            gpu.device
                .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                    Type: heap_type,
                    NumDescriptors: descriptor_count as u32,
                    Flags: flags,
                    ..Default::default()
                })
                .expect("Failed to create Render Target View Descriptor heap")
        };
        let heap_increment = unsafe {
            gpu.device
                .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV)
        } as usize;
        let heap_start = unsafe { heap.GetCPUDescriptorHandleForHeapStart() };
        Self {
            heap,
            current_ptr: heap_start,
            heap_increment,
        }
    }

    pub fn cpu_handle(&mut self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        let result = self.current_ptr;
        self.current_ptr.ptr += self.heap_increment;
        result
    }

    pub fn heap(&self) -> ID3D12DescriptorHeap {
        self.heap.clone()
    }

    pub fn gpu_handle(&self) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        unsafe { self.heap.GetGPUDescriptorHandleForHeapStart() }
    }
}
