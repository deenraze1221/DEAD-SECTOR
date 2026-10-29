// Sector Engine atmosphere layers: additive ground decals (round light pools)
// and two scrolling noise fog sheets that follow the camera. Shares the frame
// uniform layout with scene.wgsl (only the fields used here).

struct PointLight {
    position_range: vec4<f32>,
    color: vec4<f32>,
};

struct Frame {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    sun_color: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    sky_top: vec4<f32>,
    sky_horizon: vec4<f32>,
    ground_horizon: vec4<f32>,
    ground_bottom: vec4<f32>,
    params: vec4<f32>,
    game: vec4<f32>,
    lights: array<PointLight, 16>,
};

struct Effects {
    // rgb = linear decal colour, a = decal opacity.
    pool: vec4<f32>,
    // rgb = linear fog-sheet colour, a = density.
    mist: vec4<f32>,
    // x = scroll time in seconds, y/z = sheet heights, w = sheet half size.
    params: vec4<f32>,
    // x = fade start, y = fade end (distance from the camera).
    reach: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;
@group(1) @binding(0) var<uniform> effects: Effects;
@group(1) @binding(1) var mist_noise: texture_2d<f32>;
@group(1) @binding(2) var mist_sampler: sampler;

struct QuadOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    // −1..1 across the quad.
    @location(1) local: vec2<f32>,
};

fn quad_corner(index: u32) -> vec2<f32> {
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    return corners[index];
}

fn horizontal_quad(index: u32, centre: vec3<f32>, half_size: f32) -> QuadOut {
    let corner = quad_corner(index);
    let world = centre + vec3<f32>(corner.x, 0.0, corner.y) * half_size;
    var out: QuadOut;
    out.clip = frame.view_proj * vec4<f32>(world, 1.0);
    out.world = world;
    out.local = corner;
    return out;
}

fn fog_amount(world: vec3<f32>) -> f32 {
    return 1.0 - exp(min(0.0, -length(world - frame.camera_pos.xyz) * frame.fog.w));
}

// One round decal per instance: xyz = centre, w = radius (0 hides it).
@vertex
fn vs_pool(@builtin(vertex_index) vertex: u32, @location(0) decal: vec4<f32>) -> QuadOut {
    return horizontal_quad(vertex, decal.xyz, decal.w);
}

@fragment
fn fs_pool(in: QuadOut) -> @location(0) vec4<f32> {
    // atmosphere.gd's haze texture: (1 − r)² alpha falloff.
    let falloff = pow(max(0.0, 1.0 - length(in.local)), 2.0);
    let light = effects.pool.rgb * effects.pool.a * falloff * (1.0 - fog_amount(in.world));
    return vec4<f32>(light, 1.0);
}

// Two horizontal sheets centred under the camera.
@vertex
fn vs_mist(@builtin(vertex_index) vertex: u32, @builtin(instance_index) sheet: u32) -> QuadOut {
    let height = select(effects.params.y, effects.params.z, sheet == 1u);
    return horizontal_quad(vertex, vec3<f32>(frame.camera_pos.x, height, frame.camera_pos.z), effects.params.w);
}

@fragment
fn fs_mist(in: QuadOut) -> @location(0) vec4<f32> {
    let scroll = effects.params.x;
    let uv = in.world.xz * 0.045;
    let n = textureSample(mist_noise, mist_sampler, uv + vec2<f32>(scroll * 0.012, scroll * 0.007)).r;
    let m = textureSample(mist_noise, mist_sampler, uv * 2.3 - vec2<f32>(scroll * 0.009, 0.0)).r;
    let cloud = smoothstep(0.38, 0.82, n * 0.65 + m * 0.35);
    let reach = 1.0 - smoothstep(effects.reach.x, effects.reach.y, length(in.world.xz - frame.camera_pos.xz));
    let color = mix(effects.mist.rgb, frame.fog.rgb, fog_amount(in.world));
    return vec4<f32>(color, cloud * reach * effects.mist.a);
}
