use std::{collections::HashMap, ffi::c_void};

use bevy::prelude::*;
use windows::{
    core::*,
    Win32::{
        Foundation::{HANDLE, RECT},
        Graphics::{
            Direct3D::{
                Fxc::{D3DCompile, D3DCOMPILE_DEBUG, D3DCOMPILE_SKIP_OPTIMIZATION},
                ID3DBlob, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            },
            Direct3D12::*,
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
                },
                DXGI_PRESENT,
            },
        },
        System::Threading::{CreateEventA, WaitForSingleObject, INFINITE},
    },
};

use crate::{
    gpu::Gpu,
    swapchain::WindowRenderTarget,
    triangle::{Triangle, TriangleVertexBuffer, TriangleVertexBuffers},
};

type PipelineId = usize;
const THE_ONLY_PIPELINE: PipelineId = 0;

#[derive(Default)]
pub struct Pipeline {
    root_signature: Option<ID3D12RootSignature>,
    state: Option<ID3D12PipelineState>,
    command_list: Option<ID3D12GraphicsCommandList>,
}

#[derive(Resource)]
pub struct Pipelines {
    pub storage: HashMap<PipelineId, Pipeline>,
}

impl Pipelines {
    pub fn new() -> Self {
        Pipelines {
            storage: HashMap::new(),
        }
    }
}

pub fn create_root_signature(gpu: Res<Gpu>, mut pipelines: ResMut<Pipelines>) {
    let pipeline_entry = pipelines.storage.entry(0).or_default();
    if pipeline_entry.root_signature.is_some() {
        return;
    }

    let root_signature_desc = D3D12_ROOT_SIGNATURE_DESC {
        Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
        ..Default::default()
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
    pipeline_entry.root_signature = Some(unsafe {
        gpu.device
            .CreateRootSignature(
                0,
                std::slice::from_raw_parts(
                    signature.GetBufferPointer() as *const u8,
                    signature.GetBufferSize(),
                ),
            )
            .expect("Failed to create root signature")
    });
}

pub fn create_pipeline_state(gpu: Res<Gpu>, mut pipelines: ResMut<Pipelines>) {
    let pipeline_entry = pipelines.storage.entry(THE_ONLY_PIPELINE).or_default();
    if pipeline_entry.state.is_some() {
        return;
    }

    let shader_code = s!(r#"
    struct VS_OUTPUT {
        float4 Pos : SV_POSITION;
        float4 Color : COLOR0;
    };

    VS_OUTPUT VSMain(float4 inPos : POSITION, float4 inColor : COLOR) {
        VS_OUTPUT output;
        output.Pos = inPos;
        output.Color = inColor;
        return output;
    }

    float4 PSMain(VS_OUTPUT input) : SV_TARGET {
        return input.Color;
    }
    "#);

    let mut vertex_shader: Option<ID3DBlob> = None;
    let mut pixel_shader: Option<ID3DBlob> = None;
    let mut vertex_error_msg: Option<ID3DBlob> = None;
    let mut pixel_error_msg: Option<ID3DBlob> = None;

    let compile_flags = if cfg!(debug_assertions) {
        D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
    } else {
        0
    };
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

    let vertex_shader = vertex_shader
        .as_ref()
        .expect("Compile was successful but vertex shader is None");
    let pixel_shader = pixel_shader
        .as_ref()
        .expect("Compile was successful but pixel shader is None");

    let position_element_desc = D3D12_INPUT_ELEMENT_DESC {
        SemanticName: s!("POSITION"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    };
    let color_element_desc = D3D12_INPUT_ELEMENT_DESC {
        SemanticName: s!("COLOR"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 12,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    };

    let mut pipeline_state_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
        InputLayout: D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: [position_element_desc, color_element_desc].as_ptr(),
            NumElements: 2,
        },
        pRootSignature: unsafe { std::mem::transmute_copy(&pipeline_entry.root_signature) },
        VS: D3D12_SHADER_BYTECODE {
            pShaderBytecode: unsafe { vertex_shader.GetBufferPointer() },
            BytecodeLength: unsafe { vertex_shader.GetBufferSize() },
        },
        PS: D3D12_SHADER_BYTECODE {
            pShaderBytecode: unsafe { pixel_shader.GetBufferPointer() },
            BytecodeLength: unsafe { pixel_shader.GetBufferSize() },
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

    pipeline_entry.state = Some(unsafe {
        gpu.device
            .CreateGraphicsPipelineState(&pipeline_state_desc)
            .unwrap()
    });
}

pub fn create_command_list(gpu: Res<Gpu>, mut pipelines: ResMut<Pipelines>) {
    let pipeline_entry = pipelines.storage.entry(THE_ONLY_PIPELINE).or_default();
    if pipeline_entry.command_list.is_some() {
        return;
    }

    let command_list: ID3D12GraphicsCommandList = unsafe {
        gpu.device.CreateCommandList(
            0,
            D3D12_COMMAND_LIST_TYPE_DIRECT,
            &gpu.command_allocator,
            pipeline_entry.state.as_ref().expect("State Object is None"),
        )
    }
    .expect("CreateCommandList failed");
    unsafe {
        command_list.Close().expect("Failed to close command list");
    };

    pipeline_entry.command_list = Some(command_list);
}

// redo this shit
pub fn render(
    mut windows: Query<(&mut WindowRenderTarget, &Window)>,
    triangles: Query<Entity, With<Triangle>>,
    vertex_buffers: Res<TriangleVertexBuffers>,
    gpu: Res<Gpu>,
    mut pipelines: ResMut<Pipelines>,
) {
    let pipeline = pipelines.storage.get_mut(&THE_ONLY_PIPELINE).unwrap();
    for (mut target, window) in windows.iter_mut() {
        for triangle in triangles.iter() {
            let vertex_buffer = vertex_buffers.get(&triangle).unwrap();
            populate_command_list(&gpu, pipeline, window, &target, vertex_buffer)
                .expect("Failed to populate command list");
            let command_list = Some(pipeline.command_list.as_ref().unwrap().cast().unwrap());
            unsafe { gpu.queue.ExecuteCommandLists(&[command_list]) };
        }
        unsafe { target.swapchain.Present(1, DXGI_PRESENT(0)) }
            .ok()
            .unwrap();
        target.signal_end_present(&gpu.queue);
    }
}

fn populate_command_list(
    gpu: &Gpu,
    pipeline: &Pipeline,
    window: &Window,
    render_target: &WindowRenderTarget,
    vertex_buffer: &TriangleVertexBuffer,
) -> Result<()> {
    // Command list allocators can only be reset when the associated
    // command lists have finished execution on the GPU; apps should use
    // fences to determine GPU execution progress.
    unsafe {
        gpu.command_allocator.Reset()?;
    }

    let command_list = pipeline.command_list.as_ref().unwrap();
    let pipeline_state_object = pipeline.state.as_ref().unwrap();
    let root_signature = pipeline.root_signature.as_ref().unwrap();
    let viewport = render_target.viewport;

    // However, when ExecuteCommandList() is called on a particular
    // command list, that command list can then be reset at any time and
    // must be before re-recording.
    unsafe {
        command_list.Reset(&gpu.command_allocator, pipeline_state_object)?;
    }

    let scissor_rect = RECT {
        left: 0,
        top: 0,
        right: window.width() as i32,
        bottom: window.height() as i32,
    };
    // Set necessary state.
    unsafe {
        command_list.SetGraphicsRootSignature(root_signature);
        command_list.RSSetViewports(&[viewport]);
        command_list.RSSetScissorRects(&[scissor_rect]);
    };

    let back_buffer = render_target.back_buffer();
    // Indicate that the back buffer will be used as a render target.
    let barrier = transition_barrier(
        back_buffer,
        D3D12_RESOURCE_STATE_PRESENT,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
    );
    unsafe { command_list.ResourceBarrier(&[barrier]) };

    let rtv_handle = render_target.back_buffer_handle();

    unsafe { command_list.OMSetRenderTargets(1, Some(&rtv_handle), false, None) };

    // Record commands.
    unsafe {
        command_list.ClearRenderTargetView(rtv_handle, &[0.0_f32, 0.2_f32, 0.4_f32, 1.0_f32], None);
        command_list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
        command_list.IASetVertexBuffers(0, Some(&[vertex_buffer.view]));
        command_list.DrawInstanced(3, 1, 0, 0);

        // Indicate that the back buffer will now be used to present.
        command_list.ResourceBarrier(&[transition_barrier(
            back_buffer,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        )]);
    }

    unsafe { command_list.Close() }
}

fn transition_barrier(
    resource: &ID3D12Resource,
    state_before: D3D12_RESOURCE_STATES,
    state_after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: std::mem::ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: unsafe { std::mem::transmute_copy(resource) },
                StateBefore: state_before,
                StateAfter: state_after,
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
            }),
        },
    }
}
