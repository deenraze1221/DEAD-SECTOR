// Dead Sector surface hook for Sector Engine's scene shader.
// Ports godot/shaders/surface.gdshader (procedural brick, tile, wood, grime and
// wet asphalt) and the blackout dimming of powered.gdshader.
// frame.game: x = wetness 0..1, y = blacked-out city quadrant (−1 = none).

const SHADING_POWERED: u32 = 16u;


fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), f.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), f.x),
        f.y,
    );
}

// GLSL mod(): floor-based, also for negative world coordinates.
fn gmod(x: f32, y: f32) -> f32 {
    return x - y * floor(x / y);
}

struct Surface {
    albedo: vec3<f32>,
    roughness: f32,
};

// The world-space grids are evaluated unconditionally so fwidth() stays in uniform control flow.
fn procedural_surface(kind: u32, tint: vec3<f32>, world: vec3<f32>, normal: vec3<f32>) -> Surface {
    var p: vec2<f32>;
    if abs(normal.y) > 0.7 {
        p = world.xz;
    } else if abs(normal.x) > 0.7 {
        p = vec2<f32>(world.z, world.y);
    } else {
        p = vec2<f32>(world.x, world.y);
    }
    var detail = 0.97 + hash(floor(p * 12.0)) * 0.06;

    let row = floor(p.y / 0.25);
    let brick_cell = vec2<f32>((p.x + gmod(row, 2.0) * 0.3) / 0.6, p.y / 0.25);
    let brick_width = max(fwidth(brick_cell), vec2<f32>(0.001));
    let tile_cell = p * 0.8;
    let tile_width = max(fwidth(tile_cell), vec2<f32>(0.001));

    if kind == 1u {
        let brick = fract(brick_cell);
        let edge = min(brick, 1.0 - brick);
        let line = 1.0 - smoothstep(vec2<f32>(0.014, 0.025), vec2<f32>(0.014, 0.025) + brick_width, edge);
        let mortar = max(line.x, line.y);
        detail *= mix(0.88 + hash(floor(brick_cell)) * 0.15, 0.65, mortar);
    }
    if kind == 3u {
        let edge = min(fract(tile_cell), 1.0 - fract(tile_cell));
        let line = 1.0 - smoothstep(vec2<f32>(0.008), vec2<f32>(0.008) + tile_width, edge);
        detail *= mix(0.96, 0.72, max(line.x, line.y));
    }
    if kind == 5u {
        detail *= 0.94 + 0.06 * sin(p.x * 70.0 + sin(p.y * 3.0) * 2.0);
    }
    var base = tint * detail;
    // Grime: dirt near the ground, rain streaks down walls, blotches on flat colour.
    let vertical = 1.0 - smoothstep(0.45, 0.9, abs(normal.y));
    let ground_dirt = (1.0 - smoothstep(0.0, 3.2, world.y)) * vertical;
    var streak_axis = world.x;
    if abs(normal.x) > 0.7 {
        streak_axis = world.z;
    }
    let streak = vnoise(vec2<f32>(streak_axis * 2.3, world.y * 0.22));
    let blotch = vnoise(p * 0.17) * 0.6 + vnoise(p * 0.9) * 0.4;
    let stain = vnoise(p * 0.45 + 7.3);
    let dirt = ground_dirt * (0.18 + 0.24 * blotch) + vertical * smoothstep(0.55, 0.9, streak) * 0.16
        + smoothstep(0.62, 0.88, stain) * 0.14;
    base *= 1.0 - clamp(dirt, 0.0, 0.5);
    base *= 0.9 + 0.1 * blotch;
    let luma = dot(base, vec3<f32>(0.299, 0.587, 0.114));
    base = mix(vec3<f32>(luma), base, 0.82);

    var out: Surface;
    out.albedo = base;
    out.roughness = 0.92;
    if kind == 4u {
        let wetness = frame.game.x;
        out.albedo *= mix(1.0, 0.55, wetness);
        out.roughness = mix(0.92, 0.2, wetness);
    }
    return out;
}

fn surface_hook(shading: u32, input: SurfaceInput) -> SurfaceOutput {
    // Evaluated for every material so fwidth() stays in uniform control flow.
    let procedural = procedural_surface(shading, input.tint, input.world, input.normal);
    var out = SurfaceOutput(input.albedo, input.roughness);
    if shading > 0u && shading < SHADING_POWERED {
        out.albedo = procedural.albedo;
        out.roughness = procedural.roughness;
    }
    if shading == SHADING_POWERED {
        let w = input.world;
        let quadrant = select(select(2.0, 3.0, w.z >= 0.0), select(0.0, 1.0, w.z >= 0.0), w.x < 0.0);
        out.albedo *= select(1.0, 0.06, abs(quadrant - frame.game.y) < 0.1);
    }
    return out;
}
