use std::{ffi::c_void, ops::Neg, ptr};

use bevy::{prelude::*, utils::hashbrown::HashMap};
use windows::{
    core::*,
    Win32::Graphics::{
        Direct3D::{
            Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION},
            ID3DBlob, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        },
        Direct3D12::*,
        Dxgi::Common::{
            DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
        },
    },
};

use crate::core::{Camera, Shader, VertexBuffer};

use super::Gpu;

type PipelineId = usize;

pub const PATH_TRACER_PIPELINE_ID: PipelineId = 0;

#[repr(C)]
#[derive(Copy, Clone)]
struct CameraData {
    inverse_view_matrix: [[f32; 4]; 4],
    aspect_ratio: f32,
    fov: f32,
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
        }
    }
}

pub trait Pipeline: Send + Sync {
    fn populate_command_list(&self, command_list: &mut ID3D12GraphicsCommandList);
    fn state(&self) -> &ID3D12PipelineState;
    fn write_camera_data(&mut self, transform: &GlobalTransform, camera: &Camera);
}

#[derive(Resource, Deref, DerefMut)]
pub struct PipelineStorage(HashMap<PipelineId, Box<dyn Pipeline>>);

impl PipelineStorage {
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

pub struct PathTracerPipeline {
    root_signature: ID3D12RootSignature,
    vertex_buffer: VertexBuffer,
    state: ID3D12PipelineState,
    constant_buffer: ID3D12Resource,
}

impl Pipeline for PathTracerPipeline {
    fn populate_command_list(&self, command_list: &mut ID3D12GraphicsCommandList) {
        unsafe {
            command_list.SetPipelineState(&self.state);
            command_list.SetGraphicsRootSignature(&self.root_signature);

            let buffer_address = self.constant_buffer.GetGPUVirtualAddress();
            command_list.SetGraphicsRootConstantBufferView(0, buffer_address);

            command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            command_list.IASetVertexBuffers(0, Some(&[*self.vertex_buffer.view()]));
            command_list.DrawInstanced(6, 1, 0, 0);
        }
    }

    fn write_camera_data(&mut self, transform: &GlobalTransform, camera: &Camera) {
        let data = CameraData::new(transform, camera);

        let mut data_begin: *mut std::ffi::c_void = ptr::null_mut();
        unsafe {
            self.constant_buffer
                .Map(0, None, Some(&mut data_begin))
                .expect("Failed to map constant buffer");

            ptr::copy_nonoverlapping(
                &data as *const _ as *const u8,
                data_begin as *mut u8,
                std::mem::size_of::<CameraData>(),
            );
            self.constant_buffer.Unmap(0, None);
        }
    }

    fn state(&self) -> &ID3D12PipelineState {
        &self.state
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct PathTracerShaderHandle(pub Handle<Shader>);

struct PathTracerShaders {
    vertex_shader: ID3DBlob,
    pixel_shader: ID3DBlob,
}

pub fn create_root_signature(gpu: &Gpu) -> ID3D12RootSignature {
    let root_descriptor = D3D12_ROOT_DESCRIPTOR {
        ShaderRegister: 0,
        RegisterSpace: 0,
    };

    let root_parameter_0 = D3D12_ROOT_PARAMETER_0 {
        Descriptor: root_descriptor,
    };
    let root_parameter = D3D12_ROOT_PARAMETER {
        ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
        Anonymous: root_parameter_0,
        ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
    };

    let root_signature_desc = D3D12_ROOT_SIGNATURE_DESC {
        Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        NumParameters: 1,
        NumStaticSamplers: 0,
        pStaticSamplers: std::ptr::null(),
        pParameters: &root_parameter,
    };
    let mut signature: Option<ID3DBlob> = None;
    let mut error: Option<ID3DBlob> = None;

    unsafe {
        let result = D3D12SerializeRootSignature(
            &root_signature_desc,
            D3D_ROOT_SIGNATURE_VERSION_1,
            &mut signature,
            Some(&mut error),
        );
        match result {
            Ok(_) => {}
            Err(e) => {
                panic!(
                    "Failed to serialize root signature: error: {:?}, more error {:?}",
                    error, e
                );
            }
        }
    };
    let signature =
        signature.expect("D3D12SerializeRootSignature was successful but signature is None");
    unsafe {
        gpu.device
            .CreateRootSignature(
                0,
                std::slice::from_raw_parts(
                    signature.GetBufferPointer() as *const u8,
                    signature.GetBufferSize(),
                ),
            )
            .expect("Failed to create root signature")
    }
}

fn compile_shaders(shader_source: &Shader) -> PathTracerShaders {
    let mut vertex_shader: Option<ID3DBlob> = None;
    let mut pixel_shader: Option<ID3DBlob> = None;
    let mut vertex_error_msg: Option<ID3DBlob> = None;
    let mut pixel_error_msg: Option<ID3DBlob> = None;

    let compile_flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };
    let shader_code = shader_source.pcstr();
    unsafe {
        let result_vs = D3DCompile(
            shader_code.as_ptr() as *const c_void,
            shader_code.as_bytes().len(),
            None,
            None,
            None,
            s!("VSMain"),
            s!("vs_5_0"),
            compile_flags,
            0,
            &mut vertex_shader,
            Some(&mut vertex_error_msg),
        );

        let result_ps = D3DCompile(
            shader_code.as_ptr() as *const c_void,
            shader_code.as_bytes().len(),
            None,
            None,
            None,
            s!("PSMain"),
            s!("ps_5_0"),
            compile_flags,
            0,
            &mut pixel_shader,
            Some(&mut pixel_error_msg),
        );

        match (result_vs, result_ps) {
            (Ok(_), Ok(_)) => {}
            (Err(e), _) => panic!(
                "Vertex shader compilation failed: {:?} error message: {:?}",
                e, vertex_error_msg
            ),
            (_, Err(e)) => panic!(
                "Pixel shader compilation failed: {:?} error message: {:?}",
                e, pixel_error_msg
            ),
        }
    }

    let vertex_shader = vertex_shader.expect("Compile was successful but vertex shader is None");
    let pixel_shader = pixel_shader.expect("Compile was successful but pixel shader is None");
    PathTracerShaders {
        vertex_shader,
        pixel_shader,
    }
}

fn create_pipeline_state(
    gpu: &Gpu,
    shaders: &PathTracerShaders,
    root_signature: &ID3D12RootSignature,
) -> ID3D12PipelineState {
    let position_element_desc = D3D12_INPUT_ELEMENT_DESC {
        SemanticName: s!("POSITION"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    };

    let uv_element_desc = D3D12_INPUT_ELEMENT_DESC {
        SemanticName: s!("TEXCOORD"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: D3D12_APPEND_ALIGNED_ELEMENT,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    };

    let input_element_descs = [position_element_desc, uv_element_desc];
    let input_layout_desc = D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: input_element_descs.as_ptr(),
        NumElements: input_element_descs.len() as u32,
    };

    let mut pipeline_state_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
        InputLayout: input_layout_desc,
        pRootSignature: unsafe { std::mem::transmute_copy(root_signature) },
        VS: D3D12_SHADER_BYTECODE {
            pShaderBytecode: unsafe { shaders.vertex_shader.GetBufferPointer() },
            BytecodeLength: unsafe { shaders.vertex_shader.GetBufferSize() },
        },
        PS: D3D12_SHADER_BYTECODE {
            pShaderBytecode: unsafe { shaders.pixel_shader.GetBufferPointer() },
            BytecodeLength: unsafe { shaders.pixel_shader.GetBufferSize() },
        },
        RasterizerState: D3D12_RASTERIZER_DESC {
            FillMode: D3D12_FILL_MODE_SOLID,
            CullMode: D3D12_CULL_MODE_NONE,
            ..Default::default()
        },
        BlendState: D3D12_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            IndependentBlendEnable: false.into(),
            RenderTarget: [
                D3D12_RENDER_TARGET_BLEND_DESC {
                    BlendEnable: false.into(),
                    LogicOpEnable: false.into(),
                    SrcBlend: D3D12_BLEND_ONE,
                    DestBlend: D3D12_BLEND_ZERO,
                    BlendOp: D3D12_BLEND_OP_ADD,
                    SrcBlendAlpha: D3D12_BLEND_ONE,
                    DestBlendAlpha: D3D12_BLEND_ZERO,
                    BlendOpAlpha: D3D12_BLEND_OP_ADD,
                    LogicOp: D3D12_LOGIC_OP_NOOP,
                    RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
                },
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
                D3D12_RENDER_TARGET_BLEND_DESC::default(),
            ],
        },
        DepthStencilState: D3D12_DEPTH_STENCIL_DESC::default(),
        SampleMask: u32::MAX,
        PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        NumRenderTargets: 1,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    pipeline_state_desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;

    // Create the graphics pipeline state
    unsafe {
        gpu.device
            .CreateGraphicsPipelineState(&pipeline_state_desc)
            .expect("Failed to create pipeline state")
    }
}

fn create_constant_buffer(gpu: &Gpu) -> ID3D12Resource {
    let constant_buffer_size = std::mem::size_of::<CameraData>() as u64;
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
    constant_buffer.expect("Failed to create constant buffer")
}

pub fn create_pathtracer_pipeline(
    gpu: Res<Gpu>,
    shader_handle: Res<PathTracerShaderHandle>,
    shaders: Res<Assets<Shader>>,
    mut pipelines: ResMut<PipelineStorage>,
) {
    if pipelines.contains_key(&PATH_TRACER_PIPELINE_ID) {
        return;
    }
    let shader_source = shaders.get(&shader_handle.0);
    if shader_source.is_none() {
        return;
    }

    let compiled_shaders = compile_shaders(shader_source.unwrap());
    let root_signature = create_root_signature(&gpu);
    let state = create_pipeline_state(&gpu, &compiled_shaders, &root_signature);
    let vertex_buffer = VertexBuffer::fullscreen_quad(&gpu);
    let constant_buffer = create_constant_buffer(&gpu);

    let pipeline = PathTracerPipeline {
        state,
        root_signature,
        vertex_buffer,
        constant_buffer,
    };

    pipelines.insert(PATH_TRACER_PIPELINE_ID, Box::new(pipeline));
}
