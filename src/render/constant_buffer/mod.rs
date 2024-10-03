use std::ptr;

use windows::Win32::Graphics::{
    Direct3D12::*,
    Dxgi::Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
};

use super::Gpu;

pub struct ConstantBuffer<T> {
    pub buffer: ID3D12Resource,
    _type: std::marker::PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(gpu: &Gpu) -> Self {
        let size_of = std::mem::size_of::<T>();
        let constant_buffer_size = size_of as u64;
        let constant_buffer_desc = D3D12_RESOURCE_DESC {
            Alignment: 0,
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: constant_buffer_size,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                ..Default::default()
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };

        let mut constant_buffer: Option<ID3D12Resource> = None;

        let heap_properties = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_UPLOAD,
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 1,
            VisibleNodeMask: 1,
        };

        unsafe {
            gpu.device
                .CreateCommittedResource(
                    &heap_properties,
                    D3D12_HEAP_FLAG_NONE,
                    &constant_buffer_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut constant_buffer,
                )
                .expect("Failed to create constant buffer");
        }
        Self {
            buffer: constant_buffer.expect("Failed to create constant buffer"),
            _type: std::marker::PhantomData,
        }
    }

    pub fn write(&mut self, data: &T) {
        let mut data_begin: *mut std::ffi::c_void = ptr::null_mut();
        unsafe {
            self.buffer
                .Map(0, None, Some(&mut data_begin))
                .expect("Failed to map constant buffer");

            ptr::copy_nonoverlapping(
                data as *const _ as *const u8,
                data_begin as *mut u8,
                std::mem::size_of::<T>(),
            );
            self.buffer.Unmap(0, None);
        }
    }

    pub fn gpu_adress(&self) -> u64 {
        unsafe { self.buffer.GetGPUVirtualAddress() }
    }
}
