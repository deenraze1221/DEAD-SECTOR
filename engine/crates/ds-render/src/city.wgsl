// Stage 1 city shader: instanced geometry, sun + ambient lighting, exponential fog.
// Procedural surfaces, shadows and post-processing come in later stages.

struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    // Unit vector pointing towards the sun.
    sun_dir: vec4<f32>,
    // Linear colour × energy.
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    // rgb = linear fog colour, w = density.
    fog: vec4<f32>,
};

struct Material {
    albedo: vec4<f32>,
    emission: vec4<f32>,
    // x = flags (see FLAG_*).
    flags: vec4<u32>,
};

const FLAG_UNLIT: u32 = 1u;
const FLAG_ALPHA_TEST: u32 = 2u;

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

@fragment
fn fs_main(in: VertexOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    let flags = material.flags.x;
    // Untextured materials bind a 1×1 white texture.
    let texel = textureSample(label_texture, label_sampler, in.uv);
    if (flags & FLAG_ALPHA_TEST) != 0u && texel.a < 0.5 {
        discard;
    }
    let albedo = material.albedo.rgb * in.color * texel.rgb;

    var color = albedo;
    if (flags & FLAG_UNLIT) == 0u {
        var n = in.normal;
        let len = length(n);
        n = select(vec3<f32>(0.0, 1.0, 0.0), n / len, len > 1e-6);
        if !front {
            n = -n;
        }
        let diffuse = max(dot(n, frame.sun_dir.xyz), 0.0);
        color = albedo * (frame.ambient.rgb + frame.sun_color.rgb * diffuse) + material.emission.rgb;
    }

    // Godot-style exponential fog.
    let distance = length(in.world - frame.camera_pos.xyz);
    let fog = 1.0 - exp(-distance * frame.fog.w);
    color = mix(color, frame.fog.rgb, fog);
    return vec4<f32>(color, material.albedo.a);
}
