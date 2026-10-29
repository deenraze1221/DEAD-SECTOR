// Post chain: Godot glow (gaussian mip chain), filmic tonemap, glow soft-light
// blend and brightness/contrast/saturation, then pixels.gdshader (540-line
// pixel grid, lens rain, sharpening, grade, vignette, scanlines, grain).

@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
}

// ---- glow -------------------------------------------------------------------

struct GlowPass {
    // xy = destination texel size in UV, z = glow strength, w = 1 on the first level.
    params: vec4<f32>,
    // x = HDR threshold, y = HDR scale, z = bloom, w = luminance cap.
    threshold: vec4<f32>,
};

@group(0) @binding(0) var<uniform> glow_pass: GlowPass;
@group(0) @binding(1) var glow_source: texture_2d<f32>;
@group(0) @binding(2) var linear_sampler: sampler;

fn glow_tap(uv: vec2<f32>, weight: f32) -> vec4<f32> {
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return vec4<f32>(0.0);
    }
    return textureSampleLevel(glow_source, linear_sampler, uv, 0.0) * weight;
}

fn destination_uv(position: vec4<f32>) -> vec2<f32> {
    return position.xy * glow_pass.params.xy;
}

@fragment
fn fs_glow_horizontal(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = destination_uv(position);
    // Reading from the 2× larger level, so step half a destination texel.
    let step = vec2<f32>(glow_pass.params.x * 0.5, 0.0);
    var color = glow_tap(uv, 0.174938);
    color += glow_tap(uv + step, 0.165569) + glow_tap(uv - step, 0.165569);
    color += glow_tap(uv + step * 2.0, 0.140367) + glow_tap(uv - step * 2.0, 0.140367);
    color += glow_tap(uv + step * 3.0, 0.106595) + glow_tap(uv - step * 3.0, 0.106595);
    color *= glow_pass.params.z;
    if glow_pass.params.w > 0.5 {
        let t = glow_pass.threshold;
        let luminance = max(color.r, max(color.g, color.b));
        let feedback = max(smoothstep(t.x, t.x + t.y, luminance), t.z);
        color = min(color * feedback, vec4<f32>(t.w));
    }
    return color;
}

@fragment
fn fs_glow_vertical(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = destination_uv(position);
    let step = vec2<f32>(0.0, glow_pass.params.y);
    var color = glow_tap(uv, 0.288713);
    color += glow_tap(uv + step, 0.233062) + glow_tap(uv - step, 0.233062);
    color += glow_tap(uv + step * 2.0, 0.122581) + glow_tap(uv - step * 2.0, 0.122581);
    return color * glow_pass.params.z;
}

// ---- tonemap ----------------------------------------------------------------

struct Tonemap {
    // x = exposure, y = white, z = glow intensity.
    params: vec4<f32>,
    // x = brightness, y = contrast, z = saturation.
    bcs: vec4<f32>,
};

@group(0) @binding(0) var<uniform> tonemap: Tonemap;
@group(0) @binding(1) var hdr_scene: texture_2d<f32>;
@group(0) @binding(2) var glow_level3: texture_2d<f32>;
@group(0) @binding(3) var glow_level5: texture_2d<f32>;
@group(0) @binding(4) var glow_sampler: sampler;

fn filmic(color: vec3<f32>, white: f32) -> vec3<f32> {
    // Godot's Uncharted 2 curve with its ×2 exposure bias baked into A and B.
    let a = 0.22 * 4.0;
    let b = 0.30 * 2.0;
    let c = 0.10;
    let d = 0.20;
    let e = 0.01;
    let f = 0.30;
    let mapped = ((color * (a * color + c * b) + d * e) / (color * (a * color + b) + d * f)) - e / f;
    let w = ((white * (a * white + c * b) + d * e) / (white * (a * white + b) + d * f)) - e / f;
    return mapped / w;
}

fn linear_to_srgb(linear: vec3<f32>) -> vec3<f32> {
    let c = clamp(linear, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, 12.92 * c, c < vec3<f32>(0.0031308));
}

fn soft_light(base: f32, blend: f32) -> f32 {
    if blend <= 0.5 {
        return base - (1.0 - 2.0 * blend) * base * (1.0 - base);
    }
    if base <= 0.25 {
        return base + (2.0 * blend - 1.0) * (4.0 * base * (4.0 * base + 1.0) * (base - 1.0) + 7.0 * base);
    }
    return base + (2.0 * blend - 1.0) * (sqrt(base) - base);
}

@fragment
fn fs_tonemap(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(hdr_scene));
    let uv = position.xy / size;
    let exposure = tonemap.params.x;
    let white = tonemap.params.y;
    var color = textureLoad(hdr_scene, vec2<i32>(position.xy), 0).rgb * exposure;
    color = linear_to_srgb(filmic(max(color, vec3<f32>(0.0)), white));

    var glow = (textureSampleLevel(glow_level3, glow_sampler, uv, 0.0).rgb
        + textureSampleLevel(glow_level5, glow_sampler, uv, 0.0).rgb) * tonemap.params.z;
    glow = linear_to_srgb(filmic(max(glow * exposure, vec3<f32>(0.0)), white));
    glow = glow * 0.5 + 0.5;
    color = vec3<f32>(soft_light(color.r, glow.r), soft_light(color.g, glow.g), soft_light(color.b, glow.b));

    let bcs = tonemap.bcs;
    color = mix(vec3<f32>(0.0), color, bcs.x);
    color = mix(vec3<f32>(0.5), color, bcs.y);
    color = mix(vec3<f32>(dot(vec3<f32>(1.0), color) * 0.33333), color, bcs.z);
    return vec4<f32>(color, 1.0);
}

// ---- pixels.gdshader --------------------------------------------------------

struct Pixels {
    // xy = output size in pixels, z = time, w = detail.
    output: vec4<f32>,
    // x = lens rain, y = hurt, z = low health (0/1), w = 1 to encode for an sRGB target.
    effects: vec4<f32>,
};

@group(0) @binding(0) var<uniform> pixels: Pixels;
@group(0) @binding(1) var ldr_scene: texture_2d<f32>;
@group(0) @binding(2) var nearest_sampler: sampler;

fn h21(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn screen(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(ldr_scene, nearest_sampler, uv, 0.0).rgb;
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let x = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(pow((x + 0.055) / 1.055, vec3<f32>(2.4)), x / 12.92, x <= vec3<f32>(0.04045));
}

@fragment
fn fs_pixels(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let resolution = pixels.output.xy;
    let time = pixels.output.z;
    let detail = pixels.output.w;
    let rain = pixels.effects.x;
    let hurt = pixels.effects.y;
    let screen_uv = position.xy / resolution;
    // The scene is rendered directly at the pixel grid (≤ 540 lines).
    let grid = vec2<f32>(textureDimensions(ldr_scene));
    var uv = (floor(screen_uv * grid) + 0.5) / grid;

    var wet = 0.0;
    if rain > 0.01 {
        let cells = vec2<f32>(16.0, 9.0);
        for (var layer = 0; layer < 2; layer++) {
            let lf = f32(layer);
            let cuv = screen_uv * cells * select(1.9, 1.0, layer == 0) + vec2<f32>(lf * 7.3, 0.0);
            let id = floor(cuv);
            let seed = h21(id + lf * 31.0);
            let age = fract(time * (0.12 + seed * 0.1) + seed * 5.0);
            var centre = id + vec2<f32>(0.25 + 0.5 * h21(id + 1.7), 0.2 + 0.5 * h21(id + 4.1));
            centre.y -= age * age * 0.55;
            let delta = (cuv - centre) * vec2<f32>(1.0, 0.8);
            let drop = smoothstep(0.2 + 0.08 * seed, 0.0, length(delta)) * smoothstep(0.0, 0.15, age)
                * smoothstep(1.0, 0.7, age) * step(seed, rain * 0.55);
            wet = max(wet, drop);
            uv -= normalize(delta + vec2<f32>(0.0001)) * drop * 0.012;
        }
    }
    var color = screen(uv);
    let texel = 1.0 / grid;
    let neighbors = (screen(uv + vec2<f32>(texel.x, 0.0)) + screen(uv - vec2<f32>(texel.x, 0.0))
        + screen(uv + vec2<f32>(0.0, texel.y)) + screen(uv - vec2<f32>(0.0, texel.y))) * 0.25;
    color = max(vec3<f32>(0.0), color + (color - neighbors) * 0.07 * detail);
    color = mix(color, color * vec3<f32>(0.92, 1.02, 1.08) + vec3<f32>(0.012, 0.018, 0.022), wet * 0.8);
    let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
    color = mix(vec3<f32>(luminance), color, 0.9) * vec3<f32>(0.97, 1.0, 1.02);
    // Grim grade: crush blacks, cold green-teal shadows, warmer highlights, firmer contrast.
    let shade = smoothstep(0.0, 0.55, luminance);
    color *= mix(vec3<f32>(0.86, 1.0, 0.99), vec3<f32>(1.03, 1.0, 0.96), shade);
    color = pow(max(color, vec3<f32>(0.0)), vec3<f32>(1.04));
    color = mix(color, color * color * (3.0 - 2.0 * color), 0.2);
    let vignette = 1.0 - 0.36 * smoothstep(0.2, 0.8, length(screen_uv - 0.5));
    color *= 1.0 - 0.025 * step(0.5, fract(screen_uv.y * resolution.y * 0.5));
    color = mix(color, vec3<f32>(0.6, 0.025, 0.015), smoothstep(0.28, 0.7, length(screen_uv - 0.5)) * hurt * 0.5);
    if pixels.effects.z > 0.5 {
        color = mix(color, vec3<f32>(0.5, 0.025, 0.015), smoothstep(0.26, 0.68, length(screen_uv - 0.5)) * 0.55);
    }
    let grain = fract(sin(dot(position.xy + floor(time * 24.0), vec2<f32>(12.9898, 78.233))) * 43758.5453) - 0.5;
    var out = color * vignette + grain * 0.011 * detail;
    if pixels.effects.w > 0.5 {
        out = srgb_to_linear(out);
    }
    return vec4<f32>(out, 1.0);
}
