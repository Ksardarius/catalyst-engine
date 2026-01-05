// ========================================================================
//  STRUCTS (Must match Rust #[repr(C)] layout)
// ========================================================================

// --- LIGHTING ---
struct PointLight {
    position: vec4<f32>, // .xyz = position, .w = intensity
    color: vec4<f32>,    // .xyz = color,    .w = radius (or unused)
};

struct LightUniforms {
    sun_direction: vec4<f32>, // .xyz = direction, .w = intensity
    sun_color: vec4<f32>,     // .xyz = color,     .w = padding
    lights: array<PointLight, 4>,
    camera_pos: vec3<f32>,
    active_lights: u32,       // How many point lights to loop over
};

// --- MATERIAL ---
struct MaterialUniforms {
    base_color: vec4<f32>,
    roughness: f32,
    metallic: f32,
    padding: vec2<f32>,
};

// --- MESH (Per-Object) ---
struct MeshUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>, // We only use top-left 3x3
};

// --- CAMERA (Global) ---
struct Camera {
    view_proj: mat4x4<f32>,
};

// ========================================================================
//  BINDINGS
// ========================================================================

// --- GROUP 0: SCENE & CAMERA (Global) ---
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> scene_data: LightUniforms;

// --- GROUP 1: MATERIAL (Per-Material) ---
@group(1) @binding(0) var<uniform> material: MaterialUniforms;
@group(1) @binding(1) var t_diffuse: texture_2d<f32>;
@group(1) @binding(2) var s_diffuse: sampler;
@group(1) @binding(3) var t_metallic_roughness: texture_2d<f32>;
@group(1) @binding(4) var s_metallic_roughness: sampler;
@group(1) @binding(5) var t_normal: texture_2d<f32>;
@group(1) @binding(6) var s_normal: sampler;

// --- GROUP 2: MESH (Per-Object) ---
@group(2) @binding(0) var<uniform> mesh: MeshUniform;


// ========================================================================
//  INPUT / OUTPUT
// ========================================================================

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

// ========================================================================
//  VERTEX SHADER
// ========================================================================

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // 1. Pass UVs
    out.uv = in.uv;

    // 2. World Position
    let world_pos_4 = mesh.model * vec4<f32>(in.position, 1.0);
    out.world_pos = world_pos_4.xyz;

    // 3. Normal (Using Normal Matrix to handle non-uniform scaling)
    let normal_matrix = mat3x3<f32>(
        mesh.normal_matrix[0].xyz,
        mesh.normal_matrix[1].xyz,
        mesh.normal_matrix[2].xyz
    );
    out.normal = normalize(normal_matrix * in.normal);

    // 4. Clip Position (Screen Space)
    out.clip_position = camera.view_proj * world_pos_4;

    return out;
}

// ========================================================================
//  PBR FUNCTIONS
// ========================================================================

const PI = 3.14159265359;

// Trick to calculate TBN matrix on the fly without pre-computing tangents
fn getNormalFromMap(uv: vec2<f32>, world_pos: vec3<f32>, normal_geom: vec3<f32>) -> vec3<f32> {
    let tangent_normal = textureSample(t_normal, s_normal, uv).xyz * 2.0 - 1.0;

    let Q1 = dpdx(world_pos);
    let Q2 = dpdy(world_pos);
    let st1 = dpdx(uv);
    let st2 = dpdy(uv);

    let N = normalize(normal_geom);
    let T = normalize(Q1 * st2.y - Q2 * st1.y);
    let B = -normalize(cross(N, T));
    let TBN = mat3x3<f32>(T, B, N);

    return normalize(TBN * tangent_normal);
}

fn DistributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let NdotH = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;
    let denom = (NdotH2 * (a2 - 1.0) + 1.0);
    return a2 / (PI * denom * denom);
}

fn GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
    let r = (roughness + 1.0);
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn GeometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx2 = GeometrySchlickGGX(NdotV, roughness);
    let ggx1 = GeometrySchlickGGX(NdotL, roughness);
    return ggx1 * ggx2;
}

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (vec3<f32>(1.0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

// ========================================================================
//  FRAGMENT SHADER
// ========================================================================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // --- 1. SAMPLE MATERIAL ---
    // Albedo
    let albedo = textureSample(t_diffuse, s_diffuse, in.uv).rgb * material.base_color.rgb;
    
    // Metallic/Roughness (Packed: G=Roughness, B=Metallic)
    let mr_sample = textureSample(t_metallic_roughness, s_metallic_roughness, in.uv);
    let roughness = mr_sample.g * material.roughness; 
    let metallic = mr_sample.b * material.metallic;

    // Normals
    let N = getNormalFromMap(in.uv, in.world_pos, in.normal);
    let V = normalize(scene_data.camera_pos - in.world_pos);

    // F0 setup
    var F0 = vec3<f32>(0.04); 
    F0 = mix(F0, albedo, metallic);

    var Lo = vec3<f32>(0.0);

    // --- 2. DIRECTIONAL LIGHT (SUN) ---
    {
        let L = normalize(-scene_data.sun_direction.xyz);
        let H = normalize(V + L);
        let radiance = scene_data.sun_color.rgb * scene_data.sun_direction.w; 

        // Cook-Torrance
        let NDF = DistributionGGX(N, H, roughness);
        let G = GeometrySmith(N, V, L, roughness);
        let F = fresnelSchlick(max(dot(H, V), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
        let specular = numerator / denominator;

        let kS = F;
        let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
        let NdotL = max(dot(N, L), 0.0);
        
        Lo += (kD * albedo / PI + specular) * radiance * NdotL;
    }

    // --- 3. POINT LIGHTS ---
    for (var i = 0u; i < scene_data.active_lights; i++) {
        let light = scene_data.lights[i];
        let light_pos = light.position.xyz;
        let light_intensity = light.position.w;
        let light_color = light.color.rgb;

        let dist = length(light_pos - in.world_pos);
        let L = normalize(light_pos - in.world_pos);
        let H = normalize(V + L);
        
        let attenuation = 1.0 / (dist * dist);
        let radiance = light_color * light_intensity * attenuation;

        // Cook-Torrance
        let NDF = DistributionGGX(N, H, roughness);
        let G = GeometrySmith(N, V, L, roughness);
        let F = fresnelSchlick(max(dot(H, V), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
        let specular = numerator / denominator;

        let kS = F;
        let kD = (vec3<f32>(1.0) - kS) * (1.0 - metallic);
        let NdotL = max(dot(N, L), 0.0);

        Lo += (kD * albedo / PI + specular) * radiance * NdotL;
    }

    // --- 4. AMBIENT & OUTPUT ---
    let ambient = vec3<f32>(0.03) * albedo;
    let color = ambient + Lo;

    return vec4<f32>(color, 1.0);
}