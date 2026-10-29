// Screen-space occlusion from the depth + normal prepass:
//   R = ambient occlusion (normal-oriented hemisphere SSAO), then blurred.
//   G = sun contact shadow (short ray march towards the sun in view space).
// Depth uses reversed Z (near = 1, far = 0).

struct Occlusion {
    proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    // xyz = unit vector towards the sun in view space.
    sun_view: vec4<f32>,
    // x = SSAO radius, y = intensity, z = samples (0 = SSAO off), w = contact length (0 = off).
    params: vec4<f32>,
    // x = contact steps.
    params2: vec4<f32>,
};

@group(0) @binding(0) var<uniform> occlusion: Occlusion;
@group(0) @binding(1) var depth_texture: texture_depth_2d;
@group(0) @binding(2) var normal_texture: texture_2d<f32>;
@group(0) @binding(3) var raw_occlusion: texture_2d<f32>;

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

fn screen_size() -> vec2<f32> {
    return vec2<f32>(textureDimensions(depth_texture));
}

fn view_position(pixel: vec2<f32>, depth: f32) -> vec3<f32> {
    let uv = pixel / screen_size();
    let ndc = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let p = occlusion.inv_proj * ndc;
    return p.xyz / p.w;
}

fn depth_at(pixel: vec2<f32>) -> f32 {
    let size = vec2<i32>(textureDimensions(depth_texture));
    let p = clamp(vec2<i32>(pixel), vec2<i32>(0), size - 1);
    return textureLoad(depth_texture, p, 0);
}

// Projects a view-space point to pixel coordinates.
fn to_pixel(p: vec3<f32>) -> vec2<f32> {
    let clip = occlusion.proj * vec4<f32>(p, 1.0);
    let ndc = clip.xy / clip.w;
    return vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * screen_size();
}

// Interleaved gradient noise for per-pixel rotation and jitter.
fn ign(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

@fragment
fn fs_occlusion(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let pixel = floor(position.xy) + 0.5;
    let depth = depth_at(pixel);
    if depth <= 0.0 {
        // Background.
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    }
    let p = view_position(pixel, depth);
    let world_normal = textureLoad(normal_texture, vec2<i32>(pixel), 0).xyz * 2.0 - 1.0;
    let n = normalize((occlusion.view * vec4<f32>(world_normal, 0.0)).xyz);
    let noise = ign(pixel);

    var ao = 1.0;
    let samples = u32(occlusion.params.z);
    if samples > 0u {
        let radius = occlusion.params.x;
        let tangent_seed = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), abs(n.x) > 0.9);
        let t = normalize(cross(tangent_seed, n));
        let b = cross(n, t);
        var occluded = 0.0;
        for (var i = 0u; i < samples; i++) {
            // Fibonacci hemisphere, rotated per pixel, denser near the centre.
            let fi = f32(i) + 0.5;
            let z = 1.0 - fi / f32(samples);
            let r = sqrt(max(0.0, 1.0 - z * z));
            let phi = fi * 2.39996323 + noise * 6.28318531;
            let scale = mix(0.15, 1.0, (fi / f32(samples)) * (fi / f32(samples)));
            let offset = (t * (cos(phi) * r) + b * (sin(phi) * r) + n * z) * radius * scale;
            let s = p + offset;
            let scene = view_position(to_pixel(s), depth_at(to_pixel(s)));
            // Occluded when the surface there is in front of the sample, within range.
            let range = smoothstep(0.0, 1.0, radius / max(abs(p.z - scene.z), 1e-4));
            occluded += select(0.0, 1.0, scene.z >= s.z + 0.025) * range;
        }
        ao = clamp(1.0 - occluded / f32(samples) * occlusion.params.y, 0.0, 1.0);
    }

    var contact = 1.0;
    let length = occlusion.params.w;
    let steps = u32(occlusion.params2.x);
    if length > 0.0 && steps > 0u {
        let to_sun = occlusion.sun_view.xyz;
        let start = p + n * 0.02;
        for (var i = 0u; i < steps; i++) {
            let s = start + to_sun * (length * (f32(i) + noise) / f32(steps));
            let pixel_s = to_pixel(s);
            if any(pixel_s < vec2<f32>(0.0)) || any(pixel_s >= screen_size()) {
                break;
            }
            let scene_z = view_position(pixel_s, depth_at(pixel_s)).z;
            let gap = scene_z - s.z;
            // Something lies in front of the ray, but not so far in front that it is a separate object.
            if gap > 0.01 && gap < 0.25 {
                contact = 0.0;
                break;
            }
        }
        // Fade out with distance, where depth resolution is too coarse.
        contact = mix(contact, 1.0, smoothstep(30.0, 60.0, -p.z));
    }
    return vec4<f32>(ao, contact, 0.0, 1.0);
}

// 4×4 depth-aware blur of the AO channel; contact shadows stay sharp.
@fragment
fn fs_blur(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let centre = vec2<i32>(position.xy);
    let raw = textureLoad(raw_occlusion, centre, 0);
    let depth = depth_at(position.xy);
    if depth <= 0.0 {
        return raw;
    }
    let z = view_position(position.xy, depth).z;
    let size = vec2<i32>(textureDimensions(raw_occlusion));
    var sum = 0.0;
    var weight = 0.0;
    for (var y = -2; y < 2; y++) {
        for (var x = -2; x < 2; x++) {
            let q = clamp(centre + vec2<i32>(x, y), vec2<i32>(0), size - 1);
            let qz = view_position(vec2<f32>(q) + 0.5, textureLoad(depth_texture, q, 0)).z;
            let w = 1.0 / (1.0 + abs(qz - z) * 8.0);
            sum += textureLoad(raw_occlusion, q, 0).r * w;
            weight += w;
        }
    }
    return vec4<f32>(sum / weight, raw.g, 0.0, 1.0);
}
