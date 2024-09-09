struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

cbuffer CameraBuffer : register(b0) {
    matrix viewMatrix;
    matrix invViewMatrix;
    matrix projMatrix;
    matrix invProjMatrix;
};

static const float c_minimumRayHitTime = 0.01f;

static const float c_rayPosNormalNudge = 0.01f;

static const float c_superFar = 10000.0f;

static const int c_numBounces = 10;

static const int c_numRendersPerFrame = 10;

static const float c_pi = 3.14159265359f;
static const float c_twopi = 2.0f * c_pi;

uint wang_hash(inout uint seed)
{
    seed = uint(seed ^ uint(61)) ^ uint(seed >> uint(16));
    seed *= uint(9);
    seed = seed ^ (seed >> 4);
    seed *= uint(0x27d4eb2d);
    seed = seed ^ (seed >> 15);
    return seed;
}

float RandomFloat01(inout uint state)
{
    return float(wang_hash(state)) / 4294967296.0f;
}

float3 RandomUnitVector(inout uint state)
{
    float z = RandomFloat01(state) * 2.0f - 1.0f;
    float a = RandomFloat01(state) * c_twopi;
    float r = sqrt(1.0f - z * z);
    float x = r * cos(a);
    float y = r * sin(a);
    return float3(x, y, z);
}

struct SRayHitInfo
{
    float dist;
    float3 normal;
    float3 albedo;
    float3 emissive;
};

float ScalarTriple(float3 u, float3 v, float3 w)
{
    return dot(cross(u, v), w);
}

bool TestQuadTrace(float3 rayPos, float3 rayDir, inout SRayHitInfo info, float3 a, float3 b, float3 c, float3 d)
{
    float3 normal = normalize(cross(c - a, c - b));
    if (dot(normal, rayDir) > 0.0f)
    {
        normal *= -1.0f;
        float3 temp = d;
        d = a;
        a = temp;

        temp = b;
        b = c;
        c = temp;
    }

    float3 p = rayPos;
    float3 q = rayPos + rayDir;
    float3 pq = q - p;
    float3 pa = a - p;
    float3 pb = b - p;
    float3 pc = c - p;

    float3 m = cross(pc, pq);
    float v = dot(pa, m);
    float3 intersectPos;
    if (v >= 0.0f)
    {
        // Test against triangle a, b, c
        float u = -dot(pb, m);
        if (u < 0.0f) return false;
        float w = ScalarTriple(pq, pb, pa);
        if (w < 0.0f) return false;
        float denom = 1.0f / (u + v + w);
        u *= denom;
        v *= denom;
        w *= denom;
        intersectPos = u * a + v * b + w * c;
    }
    else
    {
        float3 pd = d - p;
        float u = dot(pd, m);
        if (u < 0.0f) return false;
        float w = ScalarTriple(pq, pa, pd);
        if (w < 0.0f) return false;
        v = -v;
        float denom = 1.0f / (u + v + w);
        u *= denom;
        v *= denom;
        w *= denom;
        intersectPos = u * a + v * d + w * c;
    }

    float dist;
    if (abs(rayDir.x) > 0.1f)
    {
        dist = (intersectPos.x - rayPos.x) / rayDir.x;
    }
    else if (abs(rayDir.y) > 0.1f)
    {
        dist = (intersectPos.y - rayPos.y) / rayDir.y;
    }
    else
    {
        dist = (intersectPos.z - rayPos.z) / rayDir.z;
    }

    if (dist > c_minimumRayHitTime && dist < info.dist)
    {
        info.dist = dist;
        info.normal = normal;
        return true;
    }

    return false;
}

bool TestSphereTrace(float3 rayPos, float3 rayDir, inout SRayHitInfo info, float4 sphere)
{
    float3 m = rayPos - sphere.xyz;
    float b = dot(m, rayDir);
    float c = dot(m, m) - sphere.w * sphere.w;
    if (c > 0.0f && b > 0.0f)
        return false;
    float discr = b * b - c;
    if (discr < 0.0f)
        return false;
    bool fromInside = false;
    float dist = -b - sqrt(discr);
    if (dist < 0.0f)
    {
        fromInside = true;
        dist = -b + sqrt(discr);
    }

    if (dist > c_minimumRayHitTime && dist < info.dist)
    {
        info.dist = dist;
        info.normal = normalize((rayPos + rayDir * dist) - sphere.xyz) * (fromInside ? -1.0f : 1.0f);
        return true;
    }

    return false;
}

void TestSceneTrace(float3 rayPos, float3 rayDir, inout SRayHitInfo hitInfo)
{
    float3 sceneTranslation = float3(0.0f, 0.0f, 10.0f);
    float4 sceneTranslation4 = float4(sceneTranslation, 0.0f);

    // Back wall
    {
        float3 A = float3(-12.6f, 12.6f, 25.0f) + sceneTranslation;
        float3 B = float3(12.6f, 12.6f, 25.0f) + sceneTranslation;
        float3 C = float3(12.6f, -12.6f, 25.0f) + sceneTranslation;
        float3 D = float3(-12.6f, -12.6f, 25.0f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.7f, 0.7f, 0.7f);
            hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        }
    }

    // Floor
    {
        float3 A = float3(-12.6f, 12.45f, 25.0f) + sceneTranslation;
        float3 B = float3(12.6f, 12.45f, 25.0f) + sceneTranslation;
        float3 C = float3(12.6f, 12.45f, 15.0f) + sceneTranslation;
        float3 D = float3(-12.6f, 12.45f, 15.0f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.7f, 0.7f, 0.7f);
            hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        }
    }

    // Ceiling
    {
        float3 A = float3(-12.6f, -12.5f, 25.0f) + sceneTranslation;
        float3 B = float3(12.6f, -12.5f, 25.0f) + sceneTranslation;
        float3 C = float3(12.6f, -12.5f, 15.0f) + sceneTranslation;
        float3 D = float3(-12.6f, -12.5f, 15.0f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.7f, 0.7f, 0.7f);
            hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        }
    }

    // Left wall
    {
        float3 A = float3(-12.5f, 12.6f, 25.0f) + sceneTranslation;
        float3 B = float3(-12.5f, 12.6f, 15.0f) + sceneTranslation;
        float3 C = float3(-12.5f, -12.6f, 15.0f) + sceneTranslation;
        float3 D = float3(-12.5f, -12.6f, 25.0f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.7f, 0.1f, 0.1f);
            hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        }
    }

    // Right wall
    {
        float3 A = float3(12.5f, 12.6f, 25.0f) + sceneTranslation;
        float3 B = float3(12.5f, 12.6f, 15.0f) + sceneTranslation;
        float3 C = float3(12.5f, -12.6f, 15.0f) + sceneTranslation;
        float3 D = float3(12.5f, -12.6f, 25.0f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.1f, 0.7f, 0.1f);
            hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        }
    }

    // Light
    {
        float3 A = float3(-5.0f, -12.4f, 22.5f) + sceneTranslation;
        float3 B = float3(5.0f, -12.4f, 22.5f) + sceneTranslation;
        float3 C = float3(5.0f, -12.4f, 17.5f) + sceneTranslation;
        float3 D = float3(-5.0f, -12.4f, 17.5f) + sceneTranslation;
        if (TestQuadTrace(rayPos, rayDir, hitInfo, A, B, C, D))
        {
            hitInfo.albedo = float3(0.0f, 0.0f, 0.0f);
            hitInfo.emissive = float3(1.0f, 0.9f, 0.7f) * 20.0f;
        }
}

if (TestSphereTrace(rayPos, rayDir, hitInfo, float4(-9.0f, 9.5f, 20.0f, 3.0f) + sceneTranslation4))
{
    hitInfo.albedo = float3(0.9f, 0.9f, 0.75f);
    hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
}

if (TestSphereTrace(rayPos, rayDir, hitInfo, float4(0.0f, 9.5f, 20.0f, 3.0f) + sceneTranslation4))
{
    hitInfo.albedo = float3(0.9f, 0.75f, 0.9f);
    hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
}

if (TestSphereTrace(rayPos, rayDir, hitInfo, float4(9.0f, 9.5f, 20.0f, 3.0f) + sceneTranslation4))
{
    hitInfo.albedo = float3(0.75f, 0.9f, 0.9f);
    hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
}
}

float3 GetColorForRay(float3 startRayPos, float3 startRayDir, inout uint rngState)
{
    // Initialize
    float3 ret = float3(0.0f, 0.0f, 0.0f);
    float3 throughput = float3(1.0f, 1.0f, 1.0f);
    float3 rayPos = startRayPos;
    float3 rayDir = startRayDir;

    for (int bounceIndex = 0; bounceIndex <= c_numBounces; ++bounceIndex)
    {
        SRayHitInfo hitInfo;
        hitInfo.dist = c_superFar;
        hitInfo.normal = float3(0.0f, 0.0f, 0.0f);
        hitInfo.albedo = float3(0.0f, 0.0f, 0.0f);
        hitInfo.emissive = float3(0.0f, 0.0f, 0.0f);
        TestSceneTrace(rayPos, rayDir, hitInfo);

        if (hitInfo.dist == c_superFar)
        {
            ret += float3(0.7f, 0.7f, 0.7f) * throughput;
            break;
        }

        // Update the ray position
        rayPos = (rayPos + rayDir * hitInfo.dist) + hitInfo.normal * c_rayPosNormalNudge;

        // Calculate new ray direction, in a cosine weighted hemisphere oriented at normal
        rayDir = normalize(hitInfo.normal + RandomUnitVector(rngState));

        // Add in emissive lighting
        ret += hitInfo.emissive * throughput;

        // Update the colorMultiplier
        throughput *= hitInfo.albedo;
    }

    return ret;
}

PSInput VSMain(float4 position : POSITION, float2 uv : TEXCOORD) {
    PSInput result;
    result.position = position;
    result.uv = uv;
    return result;
}

float4 PSMain(PSInput input) : SV_TARGET
{
    uint rngState = (uint(floor(input.uv.x * 32767.0f)) * 1974u + uint(floor(input.uv.y * 32767)) * 9277u) | 1;
    float2 ndc = input.uv * 2.0f - 1.0f;
    float4 clipSpacePos = float4(ndc, 1.0f, 1.0f);
    float4 viewSpacePos = mul(invProjMatrix, clipSpacePos);
    viewSpacePos /= viewSpacePos.w;
    float4 worldSpacePos = mul(invViewMatrix, viewSpacePos);
    float3 cameraPosition = invViewMatrix[3].xyz;
    float3 rayDir = normalize(worldSpacePos.xyz - cameraPosition);
    float3 rayPosition = cameraPosition;

    float3 color = float3(0.0f, 0.0f, 0.0f);
    for (int index = 0; index < c_numRendersPerFrame; ++index)
        color += GetColorForRay(rayPosition, rayDir, rngState) / float(c_numRendersPerFrame);

    return float4(color, 1.0f);
}
