use windows::Win32::Graphics::Direct3D12::{
    D3D12_COMPARISON_FUNC_ALWAYS, D3D12_FILTER_MIN_MAG_MIP_POINT, D3D12_SAMPLER_DESC,
    D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
};

#[derive(Debug, Clone)]
pub struct Sampler {
    pub desc: D3D12_SAMPLER_DESC,
}

impl Default for Sampler {
    fn default() -> Self {
        Self {
            desc: D3D12_SAMPLER_DESC {
                Filter: D3D12_FILTER_MIN_MAG_MIP_POINT,
                AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
                AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
                AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
                MipLODBias: 0.0,
                MaxAnisotropy: 1,
                ComparisonFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                BorderColor: [0.0, 0.0, 0.0, 0.0],
                MinLOD: 0.0,
                MaxLOD: f32::MAX,
            },
        }
    }
}
