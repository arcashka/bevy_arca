use bevy::{asset::Asset, prelude::*, reflect::TypePath};
use windows::Win32::Graphics::Direct3D12::D3D12_PRIMITIVE_TOPOLOGY_TYPE;

#[derive(Asset, TypePath)]
pub struct Mesh {
    primitive_topology: D3D12_PRIMITIVE_TOPOLOGY_TYPE,
    positions: Vec<[f32; 3]>,
    normals: Option<Vec<[f32; 3]>>,
    indices: Option<Vec<u32>>,
}

pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>();
    }
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

    pub fn insert_positions(&mut self, positions: Vec<[f32; 3]>) {
        self.positions = positions;
    }

    pub fn insert_normals(&mut self, normals: Vec<[f32; 3]>) {
        self.normals = Some(normals);
    }

    pub fn insert_indices(&mut self, indices: Vec<u32>) {
        self.indices = Some(indices);
    }
}
