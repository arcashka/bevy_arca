struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

cbuffer CameraBuffer : register(b0) {
    matrix inv_view_matrix;
    float aspect_ratio;
    float fov;
};

cbuffer MeshData : register(b1)
{
    uint vertex_count;
};

StructuredBuffer<float3> vertex_buffer : register(t0);
StructuredBuffer<uint> index_buffer : register(t1);

static const float MINIMUM_RAY_HIT_TIME = 0.01f;
static const float SUPER_FAR = 10000.0f;
static const int BOUNCE_NUMBER = 10;
static const int RENDERS_PER_FRAME = 10;
static const float PI = 3.14159265359f;
static const float TWO_PI = 2.0f * PI;

struct Ray
{
    float3 origin;
    float3 direction;
};

bool RayIntersectsTriangle(Ray ray, float3 v0, float3 v1, float3 v2, out float t)
{
    float3 edge1 = v1 - v0;
    float3 edge2 = v2 - v0;
    float3 h = cross(ray.direction, edge2);
    float a = dot(edge1, h);

    if (abs(a) < 0.000001)
        return false; // Ray is parallel to triangle

    float f = 1.0 / a;
    float3 s = ray.origin - v0;
    float u = f * dot(s, h);
    if (u < 0.0 || u > 1.0)
        return false;

    float3 q = cross(s, edge1);
    float v = f * dot(ray.direction, q);
    if (v < 0.0 || u + v > 1.0)
        return false;

    t = f * dot(edge2, q);
    if (t > MINIMUM_RAY_HIT_TIME)
        return true;
    else
        return false;
}

float3 GetColorForRay(float3 origin, float3 direction, inout uint rngState)
{
    Ray ray;
    ray.origin = origin;
    ray.direction = direction;

    float minDistance = SUPER_FAR;
    float3 hitColor = float3(0.0f, 0.0f, 0.0f);
    bool hit = false;

    // Loop over all triangles
    for (uint i = 0; i < vertex_count; i += 3)
    {
        // Get vertex indices
        uint index0 = index_buffer[i];
        uint index1 = index_buffer[i + 1];
        uint index2 = index_buffer[i + 2];

        // Get vertex positions
        float3 v0 = vertex_buffer[index0];
        float3 v1 = vertex_buffer[index1];
        float3 v2 = vertex_buffer[index2];

        // Perform ray-triangle intersection
        float t;
        if (RayIntersectsTriangle(ray, v0, v1, v2, t))
        {
            if (t < minDistance)
            {
                minDistance = t;
                hit = true;
                // For simplicity, set hitColor based on normal or any desired value
                float3 normal = normalize(cross(v1 - v0, v2 - v0));
                hitColor = 0.5f * (normal + 1.0f); // Simple normal-based coloring
            }
        }
    }

    if (hit)
    {
        return hitColor;
    }
    else
    {
        // Return background color
        return float3(0.4f, 0.4f, 0.4f);
    }
}

PSInput VSMain(float4 position : POSITION, float2 uv : TEXCOORD) {
    PSInput result;
    result.position = position;
    result.uv = uv;
    return result;
}

float4 PSMain(PSInput input) : SV_TARGET
{
    uint rngStateBase = (uint(floor(input.uv.x * 32767.0f)) * 1974u + uint(floor(input.uv.y * 32767.0f)) * 9277u) | 1;

    float2 ndc = float2(2.0f * input.uv.x - 1.0f, 2.0f * input.uv.y - 1.0f);
    ndc.x *= aspect_ratio;
    float scale = tan(fov * 0.5f);

    float3 rayDirCameraSpace = normalize(float3(ndc.x * scale, ndc.y * scale, -1.0f));
    float3 ray_dir = normalize(mul((float3x3)inv_view_matrix, rayDirCameraSpace));
    float3 camera_world_space = inv_view_matrix._m03_m13_m23;

    float3 color = float3(0.0f, 0.0f, 0.0f);
    for (uint index = 0; index < RENDERS_PER_FRAME; ++index) {
        uint rngState = rngStateBase + index * 15731u;
        color += GetColorForRay(camera_world_space, ray_dir, rngState) / float(RENDERS_PER_FRAME);
    }

    return float4(color, 1.0f);
}
