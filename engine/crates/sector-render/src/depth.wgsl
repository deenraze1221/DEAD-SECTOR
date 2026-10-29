// Depth-only geometry passes: sun and point-light shadow maps, and the camera
// depth + normal prepass used by SSAO and contact shadows. The vertex
// transform matches scene.wgsl exactly (and is invariant) so prepass depth
// equals main-pass depth.

struct DepthView {
    view_proj: mat4x4<f32>,
};

struct Material {
    albedo: vec4<f32>,
    emission: vec4<f32>,
    pbr: vec4<f32>,
    flags: vec4<u32>,
};

const FLAG_ALPHA_TEST: u32 = 2u;

@group(0) @binding(0) var<uniform> depth_view: DepthView;
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

struct DepthOut {
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_depth(v: VertexIn) -> DepthOut {
    let model = mat4x4<f32>(v.model0, v.model1, v.model2, v.model3);
    let world = model * vec4<f32>(v.position, 1.0);
    let a = v.model0.xyz;
    let b = v.model1.xyz;
    let c = v.model2.xyz;
    let det = dot(a, cross(b, c));
    var out: DepthOut;
    out.clip = depth_view.view_proj * world;
    out.uv = v.uv;
    out.normal = (cross(b, c) * v.normal.x + cross(c, a) * v.normal.y + cross(a, b) * v.normal.z) * sign(det);
    return out;
}

// Culled sun-shadow casters: the per-instance stream carries an index into
// the scene's instance buffer instead of the instance data itself.
struct InstanceData {
    model: mat4x4<f32>,
    color: vec4<f32>,
};

@group(2) @binding(0) var<storage, read> instance_data: array<InstanceData>;

struct CulledIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec3<f32>,
    @location(9) instance: u32,
};

@vertex
fn vs_shadow_culled(v: CulledIn) -> DepthOut {
    let model = instance_data[v.instance].model;
    let world = model * vec4<f32>(v.position, 1.0);
    let a = model[0].xyz;
    let b = model[1].xyz;
    let c = model[2].xyz;
    let det = dot(a, cross(b, c));
    var out: DepthOut;
    out.clip = depth_view.view_proj * world;
    out.uv = v.uv;
    out.normal = (cross(b, c) * v.normal.x + cross(c, a) * v.normal.y + cross(a, b) * v.normal.z) * sign(det);
    return out;
}

// Shadow casters with alpha-tested textures (labels, signs).
@fragment
fn fs_alpha_test(in: DepthOut) {
    if (material.flags.x & FLAG_ALPHA_TEST) != 0u
        && textureSample(label_texture, label_sampler, in.uv).a < 0.5 {
        discard;
    }
}

// Prepass: world normal encoded to 0..1.
@fragment
fn fs_prepass(in: DepthOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    if (material.flags.x & FLAG_ALPHA_TEST) != 0u
        && textureSample(label_texture, label_sampler, in.uv).a < 0.5 {
        discard;
    }
    let len = length(in.normal);
    var n = select(vec3<f32>(0.0, 1.0, 0.0), in.normal / len, len > 1e-6);
    if !front {
        n = -n;
    }
    return vec4<f32>(n * 0.5 + 0.5, 1.0);
}
