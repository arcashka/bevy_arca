use windows::Win32::Graphics::{
    Direct3D12::*,
    Dxgi::Common::{DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC},
};

use crate::render::{DescriptorHeap, Gpu};

use super::MeshData;

pub struct MeshBuffer {
    gpu_vertex_buffer: ID3D12Resource,
    upload_vertex_buffer: ID3D12Resource,
    gpu_index_buffer: ID3D12Resource,
    upload_index_buffer: ID3D12Resource,
}

impl MeshBuffer {
    pub fn new(gpu: &Gpu) -> Self {
        let mut gpu_vertex_buffer: Option<ID3D12Resource> = None;
        let mut upload_vertex_buffer: Option<ID3D12Resource> = None;
        let mut gpu_index_buffer: Option<ID3D12Resource> = None;
        let mut upload_index_buffer: Option<ID3D12Resource> = None;

        unsafe {
            let default_heap_properties = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_DEFAULT,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 0,
                VisibleNodeMask: 0,
            };

            let upload_heap_properties = D3D12_HEAP_PROPERTIES {
                Type: D3D12_HEAP_TYPE_UPLOAD,
                CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                CreationNodeMask: 0,
                VisibleNodeMask: 0,
            };

            let vertex_buffer_size = 1024 * 1024;
            let index_buffer_size = 1024 * 1024;

            let index_buffer_desc = D3D12_RESOURCE_DESC {
                Alignment: 0,
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: index_buffer_size,
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
            let vertex_buffer_desc = D3D12_RESOURCE_DESC {
                Alignment: 0,
                Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                Width: vertex_buffer_size,
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

            gpu.device
                .CreateCommittedResource(
                    &default_heap_properties,
                    D3D12_HEAP_FLAG_NONE,
                    &vertex_buffer_desc,
                    D3D12_RESOURCE_STATE_COMMON,
                    None,
                    &mut gpu_vertex_buffer,
                )
                .expect("Could not create GPU vertex buffer");

            gpu.device
                .CreateCommittedResource(
                    &upload_heap_properties,
                    D3D12_HEAP_FLAG_NONE,
                    &vertex_buffer_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut upload_vertex_buffer,
                )
                .expect("Could not create upload vertex buffer");

            gpu.device
                .CreateCommittedResource(
                    &default_heap_properties,
                    D3D12_HEAP_FLAG_NONE,
                    &index_buffer_desc,
                    D3D12_RESOURCE_STATE_COMMON,
                    None,
                    &mut gpu_index_buffer,
                )
                .expect("Could not create GPU index buffer");

            gpu.device
                .CreateCommittedResource(
                    &upload_heap_properties,
                    D3D12_HEAP_FLAG_NONE,
                    &index_buffer_desc,
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut upload_index_buffer,
                )
                .expect("Could not create upload index buffer");
        }
        Self {
            gpu_vertex_buffer: gpu_vertex_buffer.unwrap(),
            upload_vertex_buffer: upload_vertex_buffer.unwrap(),
            gpu_index_buffer: gpu_index_buffer.unwrap(),
            upload_index_buffer: upload_index_buffer.unwrap(),
        }
    }

    pub fn set_new_data(&self, data: &MeshData) {
        unsafe {
            let mut dst_data_vertex = std::ptr::null_mut();
            self.upload_vertex_buffer
                .Map(0, None, Some(&mut dst_data_vertex))
                .expect("failed to map vertex buffer");
            std::ptr::copy_nonoverlapping(
                data.positions.as_ptr(),
                dst_data_vertex as *mut [f32; 3],
                data.positions.len(),
            );
            self.upload_vertex_buffer.Unmap(0, None);

            let mut dst_data_index = std::ptr::null_mut();
            self.upload_index_buffer
                .Map(0, None, Some(&mut dst_data_index))
                .expect("failed to map index buffer");
            std::ptr::copy_nonoverlapping(
                data.indices.as_ptr(),
                dst_data_index as *mut u32,
                data.indices.len(),
            );
            self.upload_index_buffer.Unmap(0, None);
        }
    }

    pub fn upload(&self, command_list: &mut ID3D12GraphicsCommandList) {
        unsafe {
            let barriers_before = [
                D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::transmute_copy(&self.gpu_vertex_buffer),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_GENERIC_READ,
                                StateAfter: D3D12_RESOURCE_STATE_COPY_DEST,
                            },
                        ),
                    },
                },
                D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::transmute_copy(&self.gpu_index_buffer),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_GENERIC_READ,
                                StateAfter: D3D12_RESOURCE_STATE_COPY_DEST,
                            },
                        ),
                    },
                },
            ];
            command_list.ResourceBarrier(&barriers_before);
            command_list.CopyResource(&self.gpu_vertex_buffer, &self.upload_vertex_buffer);
            command_list.CopyResource(&self.gpu_index_buffer, &self.upload_index_buffer);

            let barriers_after = [
                D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::transmute_copy(&self.gpu_vertex_buffer),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                                StateAfter: D3D12_RESOURCE_STATE_GENERIC_READ,
                            },
                        ),
                    },
                },
                D3D12_RESOURCE_BARRIER {
                    Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                    Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
                    Anonymous: D3D12_RESOURCE_BARRIER_0 {
                        Transition: std::mem::ManuallyDrop::new(
                            D3D12_RESOURCE_TRANSITION_BARRIER {
                                pResource: std::mem::transmute_copy(&self.gpu_index_buffer),
                                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                                StateBefore: D3D12_RESOURCE_STATE_COPY_DEST,
                                StateAfter: D3D12_RESOURCE_STATE_GENERIC_READ,
                            },
                        ),
                    },
                },
            ];
            command_list.ResourceBarrier(&barriers_after);
        }
    }

    pub fn write_to_descriptor_heap(&self, gpu: &Gpu, descriptor_heap: &mut DescriptorHeap) {
        let vertex_srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_UNKNOWN,
            ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Buffer: D3D12_BUFFER_SRV {
                    FirstElement: 0,
                    NumElements: 1024,
                    StructureByteStride: std::mem::size_of::<[f32; 3]>() as u32,
                    Flags: D3D12_BUFFER_SRV_FLAG_NONE,
                },
            },
        };
        unsafe {
            let handle = descriptor_heap.cpu_handle();
            gpu.device.CreateShaderResourceView(
                &self.gpu_vertex_buffer,
                Some(&vertex_srv_desc),
                handle,
            );
        }

        let index_srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_UNKNOWN,
            ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Buffer: D3D12_BUFFER_SRV {
                    FirstElement: 0,
                    NumElements: 1024,
                    StructureByteStride: std::mem::size_of::<u32>() as u32,
                    Flags: D3D12_BUFFER_SRV_FLAG_NONE,
                },
            },
        };
        unsafe {
            let handle = descriptor_heap.cpu_handle();
            gpu.device.CreateShaderResourceView(
                &self.gpu_index_buffer,
                Some(&index_srv_desc),
                handle,
            );
        }
    }
}
