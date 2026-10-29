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
    // x = shadow slot (−1 = none).
    shadow: vec4<f32>,
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

struct Shadows {
    cascades: array<mat4x4<f32>, 4>,
    // Far edge of each cascade along the view direction.
    splits: vec4<f32>,
    // World size of one texel per cascade.
    texels: vec4<f32>,
    // x = cascade count (0 = off), y = filter (0 hard, 1 PCF, 2 soft), z = fade start, w = distance.
    sun: vec4<f32>,
    camera_forward: vec4<f32>,
    // x = filter, y = 1 / face resolution, z = 1 when the occlusion texture is valid.
    local: vec4<f32>,
    faces: array<mat4x4<f32>, 24>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var label_texture: texture_2d<f32>;
@group(1) @binding(2) var label_sampler: sampler;
@group(2) @binding(0) var<uniform> shadows: Shadows;
@group(2) @binding(1) var sun_shadow_map: texture_depth_2d_array;
@group(2) @binding(2) var local_shadow_map: texture_depth_2d_array;
@group(2) @binding(3) var shadow_sampler: sampler_comparison;
@group(2) @binding(4) var occlusion_map: texture_2d<f32>;

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
    @builtin(position) @invariant clip: vec4<f32>,
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

// ---- shadows -----------------------------------------------------------------

// 16-tap Poisson disc.
const POISSON = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216), vec2<f32>(0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870), vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432), vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543, 0.27676845), vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554), vec2<f32>(0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023), vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507), vec2<f32>(-0.81409955, 0.91437590),
    vec2<f32>(0.19984126, 0.78641367), vec2<f32>(0.14383161, -0.14100790),
);

fn compare(map: texture_depth_2d_array, uv: vec2<f32>, layer: u32, depth: f32) -> f32 {
    return textureSampleCompareLevel(map, shadow_sampler, uv, i32(layer), depth);
}

// Filtered lookup: hard (one 2×2 tap), PCF (3×3 taps) or soft (rotated Poisson disc).
fn filtered(map: texture_depth_2d_array, uv: vec2<f32>, layer: u32, depth: f32, texel: f32, mode: u32, pixel: vec2<f32>) -> f32 {
    if mode == 0u {
        return compare(map, uv, layer, depth);
    }
    if mode == 1u {
        var sum = 0.0;
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                sum += compare(map, uv + vec2<f32>(f32(x), f32(y)) * texel, layer, depth);
            }
        }
        return sum / 9.0;
    }
    let angle = fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715)))) * 6.28318531;
    let rotation = mat2x2<f32>(cos(angle), sin(angle), -sin(angle), cos(angle));
    var sum = 0.0;
    for (var i = 0; i < 16; i++) {
        sum += compare(map, uv + rotation * POISSON[i] * texel * 2.5, layer, depth);
    }
    return sum / 16.0;
}

fn project(m: mat4x4<f32>, world: vec3<f32>) -> vec3<f32> {
    let clip = m * vec4<f32>(world, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5, ndc.z);
}

// Sun visibility from the cascades: 1 = lit.
fn sun_shadow(world: vec3<f32>, n: vec3<f32>, pixel: vec2<f32>) -> f32 {
    let count = u32(shadows.sun.x);
    if count == 0u {
        return 1.0;
    }
    let view_depth = dot(world - frame.camera_pos.xyz, shadows.camera_forward.xyz);
    if view_depth >= shadows.sun.w {
        return 1.0;
    }
    var cascade = count - 1u;
    for (var i = 0u; i < count; i++) {
        if view_depth < shadows.splits[i] {
            cascade = i;
            break;
        }
    }
    // Normal offset of ~1.5 texels, more on surfaces facing away from the sun.
    let n_dot_l = clamp(dot(n, frame.sun_dir.xyz), 0.0, 1.0);
    let offset = shadows.texels[cascade] * (1.0 + 1.5 * (1.0 - n_dot_l));
    let p = project(shadows.cascades[cascade], world + n * offset);
    if any(p.xy < vec2<f32>(0.0)) || any(p.xy > vec2<f32>(1.0)) || p.z > 1.0 {
        return 1.0;
    }
    let texel = 1.0 / f32(textureDimensions(sun_shadow_map).x);
    let lit = filtered(sun_shadow_map, p.xy, cascade, p.z, texel, u32(shadows.sun.y), pixel);
    // Fade out towards the shadow distance.
    return mix(lit, 1.0, smoothstep(shadows.sun.z, shadows.sun.w, view_depth));
}

// Point-light visibility for shadow slot `slot`: 1 = lit.
fn local_shadow(slot: u32, light: vec3<f32>, world: vec3<f32>, n: vec3<f32>, pixel: vec2<f32>) -> f32 {
    let d = world - light;
    let a = abs(d);
    var face: u32;
    if a.x >= a.y && a.x >= a.z {
        face = select(1u, 0u, d.x >= 0.0);
    } else if a.y >= a.z {
        face = select(3u, 2u, d.y >= 0.0);
    } else {
        face = select(5u, 4u, d.z >= 0.0);
    }
    let layer = slot * 6u + face;
    let distance = length(d);
    // Offset along the normal by about one face texel at this distance.
    let offset = distance * shadows.local.y * 2.0 + 0.01;
    let p = project(shadows.faces[layer], world + n * offset);
    return filtered(local_shadow_map, p.xy, layer, p.z, shadows.local.y, u32(shadows.local.x), pixel);
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
        var screen_occlusion = vec2<f32>(1.0);
        if shadows.local.z > 0.5 {
            screen_occlusion = textureLoad(occlusion_map, vec2<i32>(in.clip.xy), 0).rg;
        }
        let sun_visibility = sun_shadow(in.world, n, in.clip.xy) * screen_occlusion.g;
        let sun = direct_light(n, v, frame.sun_dir.xyz, frame.sun_color.rgb * sun_visibility, roughness, f0, metallic);
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
            if attenuation > 0.0 && light.shadow.x >= 0.0 {
                attenuation *= local_shadow(u32(light.shadow.x), light.position_range.xyz, in.world, n, in.clip.xy);
            }
            if attenuation > 0.0 {
                let lit = direct_light(n, v, to_light / distance, light.color.rgb * attenuation, roughness, f0, metallic);
                diffuse += lit[0];
                specular += lit[1];
            }
        }
        let n_dot_v = max(dot(n, v), 1e-4);
        // Rough surfaces see a blurred sky: fade the reflection towards the horizon colour.
        let reflected = mix(sky_color(reflect(-v, n)), frame.sky_horizon.rgb, roughness);
        // Ambient occlusion darkens only indirect light, as in Godot (ao_light_affect 0).
        let ao = screen_occlusion.r;
        specular += reflected * env_brdf(f0, roughness, n_dot_v) * ao;
        color = (diffuse + frame.ambient.rgb * ao) * albedo * (1.0 - metallic) + specular + material.emission.rgb;
    }
    return vec4<f32>(apply_fog(color, in.world), material.albedo.a);
}
