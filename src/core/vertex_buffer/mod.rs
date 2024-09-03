use windows::Win32::Graphics::{
    Direct3D12::{
        ID3D12Resource, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_UPLOAD,
        D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_STATE_GENERIC_READ,
        D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_VERTEX_BUFFER_VIEW,
    },
    Dxgi::Common::DXGI_SAMPLE_DESC,
};

use crate::render::Gpu;

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
}

const FULLSCREEN_QUAD_VERTICES: [Vertex; 6] = [
    Vertex {
        position: [-1.0, -1.0, 0.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, -1.0, 0.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [-1.0, 1.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        uv: [1.0, 0.0],
    },
];

pub struct VertexBuffer {
    _buffer: ID3D12Resource,
    view: D3D12_VERTEX_BUFFER_VIEW,
}

impl VertexBuffer {
    pub fn view(&self) -> &D3D12_VERTEX_BUFFER_VIEW {
        &self.view
    }

    pub fn fullscreen_quad(gpu: &Gpu) -> Self {
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
                        Width: std::mem::size_of_val(&FULLSCREEN_QUAD_VERTICES) as u64,
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
            std::ptr::copy_nonoverlapping(
                FULLSCREEN_QUAD_VERTICES.as_ptr(),
                data as *mut Vertex,
                FULLSCREEN_QUAD_VERTICES.len(),
            );
            vertex_buffer.Unmap(0, None);
        }

        let vbv = D3D12_VERTEX_BUFFER_VIEW {
            BufferLocation: unsafe { vertex_buffer.GetGPUVirtualAddress() },
            StrideInBytes: std::mem::size_of::<Vertex>() as u32,
            SizeInBytes: std::mem::size_of_val(&FULLSCREEN_QUAD_VERTICES) as u32,
        };

        VertexBuffer {
            _buffer: vertex_buffer,
            view: vbv,
        }
    }
}
