use bevy::{asset::Asset, reflect::TypePath};
use windows::Win32::Graphics::Direct3D12::D3D12_PRIMITIVE_TOPOLOGY_TYPE;

#[derive(Asset, TypePath)]
pub struct Mesh {
    pub primitive_topology: D3D12_PRIMITIVE_TOPOLOGY_TYPE,
    pub positions: Vec<[f32; 3]>,
    pub normals: Option<Vec<[f32; 3]>>,
    pub indices: Option<Vec<u32>>,
}

impl Mesh {
    pub fn new(primitive_topology: D3D12_PRIMITIVE_TOPOLOGY_TYPE) -> Self {
        Self {
            primitive_topology,
            positions: Vec::new(),
            normals: None,
            indices: None,
        }
    }
}
