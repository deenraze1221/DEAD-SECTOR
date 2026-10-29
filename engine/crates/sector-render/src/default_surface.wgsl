// Default surface hook: materials are used exactly as authored.
fn surface_hook(shading: u32, input: SurfaceInput) -> SurfaceOutput {
    return SurfaceOutput(input.albedo, input.roughness);
}
