//! Colour helpers.

/// sRGB transfer function → linear.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Parses `#rrggbb` into sRGB components in 0..=1.
pub fn parse_hex_color(text: &str) -> Option<[f32; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some([(value >> 16) as u8, (value >> 8) as u8, value as u8].map(|c| c as f32 / 255.0))
}

/// `#rrggbb` → linear RGB. Panics on malformed input; meant for colour constants.
pub fn linear_hex(text: &str) -> [f32; 3] {
    parse_hex_color(text).unwrap_or_else(|| panic!("bad colour constant {text:?}")).map(srgb_to_linear)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_srgb() {
        assert_eq!(parse_hex_color("#ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
        assert_eq!(parse_hex_color("ff8000"), None);
        assert_eq!(parse_hex_color("#ff80"), None);
        assert!((srgb_to_linear(0.5) - 0.21404).abs() < 1e-5);
        assert_eq!(srgb_to_linear(0.0), 0.0);
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(linear_hex("#ffffff"), [1.0; 3]);
    }
}
