//! Validates the engine scene shader with Dead Sector's surface hook spliced in.

#[test]
fn scene_shader_is_valid_with_dead_sector_hook() {
    let source = sector_render::SCENE_SHADER.replace("// SURFACE_HOOK", dead_sector::content::SURFACE_HOOK);
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
}
