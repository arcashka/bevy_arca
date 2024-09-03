struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

PSInput VSMain(float4 position : POSITION, float2 uv : TEXCOORD) {
    PSInput result;
    result.position = position;
    result.uv = uv;
    return result;
}

float4 PSMain(PSInput input) : SV_TARGET
{
    return float4(input.uv, 0.0, 1.0);
}
