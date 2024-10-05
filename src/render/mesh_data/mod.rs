mod mesh_buffer;

use bevy::prelude::*;

use crate::core::Mesh;

use super::RenderSchedule;

pub use mesh_buffer::MeshBuffer;

pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MeshData::new())
            .add_systems(RenderSchedule, build_mesh_data);
    }
}

#[derive(Resource, Default)]
pub struct MeshData {
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    updated: bool,
}

impl MeshData {
    pub fn new() -> MeshData {
        MeshData::default()
    }

    pub fn vertex_count(&self) -> usize {
        self.indices.len()
    }

    pub fn set_used(&mut self) {
        self.updated = false;
    }

    pub fn updated(&self) -> bool {
        self.updated
    }

    fn add_mesh(&mut self, mesh: &Mesh, transform: &GlobalTransform) {
        // TODO: move matrix multiplication to GPU
        let matrix = transform.compute_matrix();
        if mesh.indices.is_none() {
            let start_index = self.positions.len() as u32;
            let mut counter: u32 = 0;
            mesh.positions.iter().for_each(|p| {
                self.positions
                    .push((matrix * Vec4::new(p[0], p[1], p[2], 1.0)).xyz().to_array());
                self.indices.push(start_index + counter);
                counter += 1;
            });
        } else {
            self.positions.extend(
                mesh.positions
                    .iter()
                    .map(|p| (matrix * Vec4::new(p[0], p[1], p[2], 1.0)).xyz().to_array()),
            );
            self.indices.extend(mesh.indices.as_ref().unwrap().iter());
        }
    }
    fn clear(&mut self) {
        self.indices.clear();
        self.positions.clear();
    }
}

pub fn build_mesh_data(
    changed_meshes: Query<Entity, (With<Handle<Mesh>>, Changed<GlobalTransform>)>,
    all_mesh_handles: Query<(&Handle<Mesh>, &GlobalTransform)>,
    mesh_assets: Res<Assets<Mesh>>,
    mut mesh_data: ResMut<MeshData>,
) {
    if changed_meshes.is_empty() {
        return;
    }

    mesh_data.clear();
    for (mesh_handle, mesh_global_transform) in all_mesh_handles.iter() {
        let mesh = mesh_assets.get(mesh_handle).unwrap();
        mesh_data.add_mesh(mesh, mesh_global_transform);
    }
    mesh_data.updated = true;
}
