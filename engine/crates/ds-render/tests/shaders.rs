//! Parses and validates every WGSL module without a GPU.

fn validate(name: &str, source: &str) {
    let module =
        naga::front::wgsl::parse_str(source).unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default())
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(source)));
}

#[test]
fn city_shader_is_valid() {
    validate("city.wgsl", include_str!("../src/city.wgsl"));
}

#[test]
fn effects_shader_is_valid() {
    validate("effects.wgsl", include_str!("../src/effects.wgsl"));
}

#[test]
fn post_shader_is_valid() {
    validate("post.wgsl", include_str!("../src/post.wgsl"));
}
