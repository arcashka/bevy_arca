mod naive_pathtracer;

use bevy::{prelude::*, utils::HashMap};
use windows::Win32::Graphics::Direct3D12::{ID3D12GraphicsCommandList, ID3D12PipelineState};

use super::MeshData;
use crate::core::Camera;

pub use naive_pathtracer::{create_pathtracer_pipeline, PathTracerShaderHandle};

type PipelineId = usize;

pub const PATH_TRACER_PIPELINE_ID: PipelineId = 0;

pub trait Pipeline: Send + Sync {
    fn populate_command_list(&self, command_list: &mut ID3D12GraphicsCommandList);
    fn state(&self) -> &ID3D12PipelineState;
    fn write_camera_data(&mut self, transform: &GlobalTransform, camera: &Camera);
    fn set_mesh_data(&mut self, data: &MeshData);
}

#[derive(Resource, Deref, DerefMut)]
pub struct PipelineStorage(HashMap<PipelineId, Box<dyn Pipeline>>);

impl PipelineStorage {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CameraData {
    inverse_view_matrix: [[f32; 4]; 4],
    aspect_ratio: f32,
    fov: f32,
    __padding: [u32; 2],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MeshInfo {
    vertex_count: u32,
    __padding: [u32; 3],
}

impl MeshInfo {
    fn new(vertex_count: u32) -> Self {
        Self {
            vertex_count,
            __padding: [0; 3],
        }
    }
}

impl CameraData {
    fn new(transform: &GlobalTransform, camera: &Camera) -> Self {
        let forward = transform.forward() * 1.0;
        let up = transform.up() * 1.0;
        let eye_position = -transform.translation();
        let target_position = eye_position + forward;

        let view_matrix = Mat4::look_at_lh(eye_position, target_position, up);
        let inverse_view_matrix = view_matrix.inverse();

        Self {
            inverse_view_matrix: inverse_view_matrix.to_cols_array_2d(),
            aspect_ratio: camera.aspect_ratio,
            fov: camera.fov,
            __padding: [0; 2],
        }
    }
}
