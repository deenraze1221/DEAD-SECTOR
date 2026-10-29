// Sector Engine scene shader: instanced geometry with a Godot-compatible
// lighting model (Burley diffuse, GGX specular, ambient colour, sky
// reflections, omni lights, exponential fog). Writes linear HDR colour.
//
// Games customise surfaces with a hook spliced in at the SURFACE_HOOK marker:
//   fn surface_hook(shading: u32, input: SurfaceInput) -> SurfaceOutput
// It runs for every fragment (keep derivatives in uniform control flow) and
// may read `frame.game` for game-defined per-frame values.

const PI: f32 = 3.14159265358979;
const MAX_LIGHTS: u32 = 16u;

struct PointLight {
    // xyz = position, w = range.
    position_range: vec4<f32>,
    // rgb = linear colour × energy.
    color: vec4<f32>,
};

struct Frame {
    view_proj: mat4x4<f32>,
    // xyz = camera position, w = time in seconds.
    camera_pos: vec4<f32>,
    // Unit vector pointing towards the sun.
    sun_dir: vec4<f32>,
    // Linear colour × energy.
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    // rgb = linear fog colour, w = density.
    fog: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    ground_horizon: vec4<f32>,
    ground_bottom: vec4<f32>,
    // z = point light count.
    params: vec4<f32>,
    // Game-defined values for the surface hook.
    game: vec4<f32>,
    lights: array<PointLight, MAX_LIGHTS>,
};

struct Material {
    albedo: vec4<f32>,
    emission: vec4<f32>,
    // x = roughness, y = metallic.
    pbr: vec4<f32>,
    // x = flags (FLAG_*), y = game shading id for the surface hook.
    flags: vec4<u32>,
};

const FLAG_UNLIT: u32 = 1u;
const FLAG_ALPHA_TEST: u32 = 2u;

struct SurfaceInput {
    // Material colour × vertex colour × texture.
    albedo: vec3<f32>,
    // Material colour × vertex colour (no texture).
    tint: vec3<f32>,
    roughness: f32,
    world: vec3<f32>,
    normal: vec3<f32>,
};

struct SurfaceOutput {
    albedo: vec3<f32>,
    roughness: f32,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var label_texture: texture_2d<f32>;
@group(1) @binding(2) var label_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(4) model0: vec4<f32>,
    @location(5) model1: vec4<f32>,
    @location(6) model2: vec4<f32>,
    @location(7) model3: vec4<f32>,
    @location(8) instance_color: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
};

@vertex
fn vs_main(v: VertexIn) -> VertexOut {
    let model = mat4x4<f32>(v.model0, v.model1, v.model2, v.model3);
    let world = model * vec4<f32>(v.position, 1.0);
    // Inverse-transpose of the 3×3 part via cofactors, so non-uniformly scaled
    // boxes keep correct normals; sign(det) keeps mirrored instances facing out.
    let a = v.model0.xyz;
    let b = v.model1.xyz;
    let c = v.model2.xyz;
    let det = dot(a, cross(b, c));
    let n = (cross(b, c) * v.normal.x + cross(c, a) * v.normal.y + cross(a, b) * v.normal.z) * sign(det);

    var out: VertexOut;
    out.clip = frame.view_proj * world;
    out.world = world.xyz;
    out.normal = n;
    out.uv = v.uv;
    out.color = v.color * v.instance_color.rgb;
    return out;
}

// SURFACE_HOOK

// ---- Godot lighting ---------------------------------------------------------

fn schlick(u: f32) -> f32 {
    let m = clamp(1.0 - u, 0.0, 1.0);
    let m2 = m * m;
    return m2 * m2 * m;
}

// Diffuse + specular radiance from one light. `light_color` is colour × energy;
// Godot multiplies light energy by π and divides it out again in the BRDF.
fn direct_light(
    n: vec3<f32>,
    v: vec3<f32>,
    l: vec3<f32>,
    light_color: vec3<f32>,
    roughness: f32,
    f0: vec3<f32>,
    metallic: f32,
) -> array<vec3<f32>, 2> {
    let n_dot_l = dot(n, l);
    let c_n_dot_l = max(n_dot_l, 0.0);
    let h = normalize(v + l);
    let c_n_dot_v = max(dot(n, v), 1e-4);
    let c_n_dot_h = clamp(dot(n, h), 0.0, 1.0);
    let c_l_dot_h = clamp(dot(l, h), 0.0, 1.0);

    // Burley diffuse.
    let fd90_minus_1 = 2.0 * c_l_dot_h * c_l_dot_h * roughness - 0.5;
    let fd_v = 1.0 + fd90_minus_1 * schlick(c_n_dot_v);
    let fd_l = 1.0 + fd90_minus_1 * schlick(c_n_dot_l);
    let diffuse = light_color * (fd_v * fd_l * c_n_dot_l);

    // GGX specular with Smith visibility and Schlick Fresnel.
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denom = c_n_dot_h * c_n_dot_h * (alpha2 - 1.0) + 1.0;
    let d = alpha2 / (PI * denom * denom);
    let vis = 0.5 / max(mix(2.0 * c_n_dot_l * c_n_dot_v, c_n_dot_l + c_n_dot_v, alpha), 1e-5);
    let f90 = clamp(dot(f0, vec3<f32>(50.0 * 0.33)), metallic, 1.0);
    let f = f0 + (f90 - f0) * schlick(c_l_dot_h);
    let specular = light_color * PI * (c_n_dot_l * d * vis) * f;
    return array<vec3<f32>, 2>(diffuse, specular);
}

// ProceduralSkyMaterial gradient (sky_curve 0.12, ground_curve 0.02).
fn sky_color(dir: vec3<f32>) -> vec3<f32> {
    let v_angle = acos(clamp(dir.y, -1.0, 1.0));
    let c_sky = 1.0 - v_angle / (PI * 0.5);
    let sky = mix(frame.sky_horizon.rgb, frame.sky_top.rgb, clamp(1.0 - pow(1.0 - c_sky, 1.0 / 0.12), 0.0, 1.0));
    let c_ground = (v_angle - PI * 0.5) / (PI * 0.5);
    let ground = mix(frame.ground_horizon.rgb, frame.ground_bottom.rgb, clamp(1.0 - pow(1.0 - c_ground, 1.0 / 0.02), 0.0, 1.0));
    return select(ground, sky, dir.y >= 0.0);
}

// Karis' analytic environment BRDF, as used by Godot.
fn env_brdf(f0: vec3<f32>, roughness: f32, n_dot_v: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let env = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * env.x + env.y;
}

fn apply_fog(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let distance = length(world - frame.camera_pos.xyz);
    let fog = 1.0 - exp(min(0.0, -distance * frame.fog.w));
    return mix(color, frame.fog.rgb, fog);
}

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let flags = material.flags.x;
    // Untextured materials bind a 1×1 white texture.
    let texel = textureSample(label_texture, label_sampler, in.uv);
    if (flags & FLAG_ALPHA_TEST) != 0u && texel.a < 0.5 {
        discard;
    }
    let len = length(in.normal);
    var n = select(vec3<f32>(0.0, 1.0, 0.0), in.normal / len, len > 1e-6);
    if !front {
        n = -n;
    }

    let surface = surface_hook(
        material.flags.y,
        SurfaceInput(material.albedo.rgb * in.color * texel.rgb, material.albedo.rgb * in.color, material.pbr.x, in.world, n),
    );
    let albedo = surface.albedo;
    let roughness = surface.roughness;

    var color: vec3<f32>;
    if (flags & FLAG_UNLIT) != 0u {
        color = albedo;
    } else {
        let metallic = material.pbr.y;
        let v = normalize(frame.camera_pos.xyz - in.world);
        let f0 = mix(vec3<f32>(0.04), albedo, metallic);
        let sun = direct_light(n, v, frame.sun_dir.xyz, frame.sun_color.rgb, roughness, f0, metallic);
        var diffuse = sun[0];
        var specular = sun[1];
        let count = u32(frame.params.z);
        for (var i = 0u; i < min(count, MAX_LIGHTS); i++) {
            let light = frame.lights[i];
            let to_light = light.position_range.xyz - in.world;
            let distance = length(to_light);
            var attenuation = distance / light.position_range.w;
            attenuation *= attenuation;
            attenuation *= attenuation;
            attenuation = max(1.0 - attenuation, 0.0);
            attenuation = attenuation * attenuation / max(distance, 0.0001);
            if attenuation > 0.0 {
                let lit = direct_light(n, v, to_light / distance, light.color.rgb * attenuation, roughness, f0, metallic);
                diffuse += lit[0];
                specular += lit[1];
            }
        }
        let n_dot_v = max(dot(n, v), 1e-4);
        // Rough surfaces see a blurred sky: fade the reflection towards the horizon colour.
        let reflected = mix(sky_color(reflect(-v, n)), frame.sky_horizon.rgb, roughness);
        specular += reflected * env_brdf(f0, roughness, n_dot_v);
        color = (diffuse + frame.ambient.rgb) * albedo * (1.0 - metallic) + specular + material.emission.rgb;
    }
    return vec4<f32>(apply_fog(color, in.world), material.albedo.a);
}
