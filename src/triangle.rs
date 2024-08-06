use bevy::{ecs::entity::EntityHashMap, prelude::*};

use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::DXGI_SAMPLE_DESC};

use crate::gpu::Gpu;

#[derive(Component)]
pub struct Triangle;

pub struct TriangleVertexBuffer {
    pub _buffer: ID3D12Resource,
    pub view: D3D12_VERTEX_BUFFER_VIEW,
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct TriangleVertexBuffers(pub EntityHashMap<TriangleVertexBuffer>);

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

pub fn create_vertex_buffers(
    gpu: Res<Gpu>,
    triangles: Query<(Entity, &Triangle)>,
    mut vertex_buffers: ResMut<TriangleVertexBuffers>,
) {
    for (entity, _) in triangles.iter() {
        if vertex_buffers.contains_key(&entity) {
            return;
        }

        let vertices = [
            Vertex {
                position: [0.0, 0.25, 0.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.25, -0.25, 0.0],
                color: [0.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [-0.25, -0.25, 0.0],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ];

        let mut vertex_buffer: Option<ID3D12Resource> = None;
        unsafe {
            gpu.device
                .CreateCommittedResource(
                    &D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_UPLOAD,
                        ..Default::default()
                    },
                    D3D12_HEAP_FLAG_NONE,
                    &D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
                        Width: std::mem::size_of_val(&vertices) as u64,
                        Height: 1,
                        DepthOrArraySize: 1,
                        MipLevels: 1,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                        ..Default::default()
                    },
                    D3D12_RESOURCE_STATE_GENERIC_READ,
                    None,
                    &mut vertex_buffer,
                )
                .expect("Could not create vertex buffer");
        };
        let vertex_buffer = vertex_buffer.unwrap();

        unsafe {
            let mut data = std::ptr::null_mut();
            vertex_buffer
                .Map(0, None, Some(&mut data))
                .expect("failed to map vertex buffer");
            std::ptr::copy_nonoverlapping(vertices.as_ptr(), data as *mut Vertex, vertices.len());
            vertex_buffer.Unmap(0, None);
        }

        let vbv = D3D12_VERTEX_BUFFER_VIEW {
            BufferLocation: unsafe { vertex_buffer.GetGPUVirtualAddress() },
            StrideInBytes: std::mem::size_of::<Vertex>() as u32,
            SizeInBytes: std::mem::size_of_val(&vertices) as u32,
        };

        vertex_buffers.insert(
            entity,
            TriangleVertexBuffer {
                _buffer: vertex_buffer,
                view: vbv,
            },
        );
    }
}
