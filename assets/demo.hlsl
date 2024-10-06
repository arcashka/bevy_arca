// Based on https://github.com/SebLague/Ray-Tracing/blob/Episode01/Assets/Scripts/Shaders/RayTracing.shader

struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

cbuffer CameraBuffer : register(b0) {
    matrix inverse_view_matrix;
    float aspect_ratio;
    float fov;
};

cbuffer MeshData : register(b1)
{
    uint vertex_count;
};

StructuredBuffer<float3> vertex_buffer : register(t0);
StructuredBuffer<uint> index_buffer : register(t1);

static const float SUPER_FAR = 10000.0f;
static const uint MAX_BOUNCE_COUNT = 10;
static const uint RENDERS_PER_FRAME = 2;
static const float PI = 3.14159265359f;

struct Ray
{
    float3 origin;
    float3 direction;
};

struct RayTracingMaterial
{
    float4 color;
    float4 specular_color;
    float4 emission_color;
    float specular_probability;
    float emission_strength;
    float smoothness;
};

struct Triangle
{
    float3 a;
    float3 b;
    float3 c;
};

struct HitInfo
{
    bool hit;
    float distance;
    float3 hit_point;
    float3 normal;
    RayTracingMaterial material;
};

uint NextRandom(inout uint state)
{
    state = state * 747796405 + 2891336453;
    uint result = ((state >> ((state >> 28) + 4)) ^ state) * 277803737;
    result = (result >> 22) ^ result;
    return result;
}

float RandomValue(inout uint state)
{
    return NextRandom(state) / 4294967295.0; // 2^32 - 1
}

float RandomValueNormalDistribution(inout uint state)
{
    float theta = 2 * 3.1415926 * RandomValue(state);
    float rho = sqrt(-2 * log(RandomValue(state)));
    return rho * cos(theta);
}

float3 RandomDirection(inout uint state)
{
    float x = RandomValueNormalDistribution(state);
    float y = RandomValueNormalDistribution(state);
    float z = RandomValueNormalDistribution(state);
    return normalize(float3(x, y, z));
}

float2 RandomPointInCircle(inout uint rng_state)
{
    float angle = RandomValue(rng_state) * 2 * PI;
    float2 point_on_circle = float2(cos(angle), sin(angle));
    return point_on_circle * sqrt(RandomValue(rng_state));
}

float2 mod2(float2 x, float2 y)
{
    return x - y * floor(x/y);
}

float3 GetEnvironmentLight(Ray ray)
{
    return float3(0.2f, 0.3f, 0.3f);
}

HitInfo IntersectTriangle(Ray ray, Triangle tri)
{
    float3 edge_ab = tri.b - tri.a;
    float3 edge_ac = tri.c - tri.a;

    float3 normal_vector = cross(edge_ab, edge_ac);
    float3 ao = ray.origin - tri.a;
    float3 dao = cross(ao, ray.direction);

    float determinant = -dot(ray.direction, normal_vector);
    float inv_determinant = 1 / determinant;

    float distance = dot(ao, normal_vector) * inv_determinant;
    float u = dot(edge_ac, dao) * inv_determinant;
    float v = -dot(edge_ab, dao) * inv_determinant;
    float w = 1 - u - v;

    HitInfo hit_info;
    hit_info.hit = determinant >= 1E-6 && distance >= 0 && u >= 0 && v >= 0 && w >= 0;
    hit_info.hit_point = ray.origin + ray.direction * distance;
    hit_info.normal = normalize(normal_vector);
    hit_info.distance = distance;

    return hit_info;
}

HitInfo GetCollision(Ray ray)
{
    HitInfo closest_hit;
    closest_hit.distance = SUPER_FAR;

    for (uint i = 0; i < vertex_count; i += 3)
    {
        Triangle tri;
        tri.a = vertex_buffer[index_buffer[i]];
        tri.b = vertex_buffer[index_buffer[i + 1]];
        tri.c = vertex_buffer[index_buffer[i + 2]];

        HitInfo hit = IntersectTriangle(ray, tri);
        if (hit.hit && hit.distance < closest_hit.distance)
        {
            closest_hit = hit;
            closest_hit.material.color = float4(0.6f, 0.0f, 0.0f, 1.0f);
            closest_hit.material.smoothness = 0.5f;
            closest_hit.material.specular_color = float4(0.5f, 0.5f, 0.5f, 1.0f);
            closest_hit.material.specular_probability = 0.5f;
            closest_hit.material.emission_color = float4(0.0f, 0.0f, 0.0f, 1.0f);
            closest_hit.material.emission_strength = 0.0f;
        }
    }
    return closest_hit;
}

float3 Trace(Ray ray, inout uint rng_state)
{
    float3 incoming_light = 0;
    float3 ray_color = 1;

    for (uint bounce_index = 0; bounce_index <= MAX_BOUNCE_COUNT; bounce_index++)
    {
        HitInfo hit_info = GetCollision(ray);

        if (hit_info.hit)
        {
            RayTracingMaterial material = hit_info.material;

            ray.origin = hit_info.hit_point;
            bool is_specular_bounce = material.specular_probability >= RandomValue(rng_state);
            float3 diffuse_direction = normalize(hit_info.normal + RandomDirection(rng_state));
            float3 specular_direction = reflect(ray.direction, hit_info.normal);
            ray.direction = normalize(lerp(diffuse_direction, specular_direction, material.smoothness * is_specular_bounce));

            // Update light calculations
            float3 emitted_light = material.emission_color.rgb * material.emission_strength;
            incoming_light += emitted_light * ray_color;
            ray_color *= lerp(material.color.rgb, material.specular_color.rgb, is_specular_bounce);

            // Random early exit if ray color is nearly 0 (can't contribute much to final result)
            float p = max(ray_color.r, max(ray_color.g, ray_color.b));
            if (RandomValue(rng_state) >= p) {
                break;
            }
            ray_color *= 1.0f / p;
        }
        else
        {
            incoming_light += GetEnvironmentLight(ray) * ray_color;
            break;
        }
    }

    return incoming_light;
}

PSInput VSMain(float4 position : POSITION, float2 uv : TEXCOORD) {
    PSInput result;
    result.position = position;
    result.uv = uv;
    return result;
}

float4 PSMain(PSInput input) : SV_TARGET
{
    uint rng_state = (uint(floor(input.uv.x * 32767.0f)) * 1974u + uint(floor(input.uv.y * 32767.0f)) * 9277u) | 1u;
    float2 ndc = float2(2.0f * input.uv.x - 1.0f, 2.0f * input.uv.y - 1.0f);
    ndc.x *= aspect_ratio;
    float scale = tan(fov * 0.5f);

    float3 ray_direction_camera_space = normalize(float3(ndc.x * scale, ndc.y * scale, -1.0f));

    Ray ray;
    ray.direction = normalize(mul((float3x3)inverse_view_matrix, ray_direction_camera_space));
    ray.origin = inverse_view_matrix._m03_m13_m23;

    float3 color = float3(0.0f, 0.0f, 0.0f);
    for (uint index = 0; index < RENDERS_PER_FRAME; ++index) {
        color += Trace(ray, rng_state);
    }

    return float4(color / float(RENDERS_PER_FRAME), 1.0f);
}
