//! The sRGB transfer function on 8-bit values, with lookup tables so that
//! per-pixel blending costs no transcendental functions.
use std::sync::OnceLock;

/// Linear light of each sRGB-encoded byte.
fn decoding() -> &'static [f32; 256] {
    static TABLE: OnceLock<[f32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0; 256];
        for (byte, value) in table.iter_mut().enumerate() {
            *value = decode_continuous(byte as f64 / 255.0) as f32;
        }
        table
    })
}

/// The linear value at which encoding moves from each byte to the next: the
/// decoded midpoint `(k + 0.5) / 255` between encoded bytes `k` and `k + 1`.
fn thresholds() -> &'static [f32; 255] {
    static TABLE: OnceLock<[f32; 255]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0; 255];
        for (byte, value) in table.iter_mut().enumerate() {
            *value = decode_continuous((byte as f64 + 0.5) / 255.0) as f32;
        }
        table
    })
}

fn decode_continuous(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear light of an sRGB-encoded byte.
pub(crate) fn decode(byte: u8) -> f32 {
    decoding()[usize::from(byte)]
}

/// The nearest sRGB-encoded byte of a linear value, clamped to `0..=1`: the
/// rounding the runtime's own encoding and `FrameImage` use.
pub(crate) fn encode(linear: f32) -> u8 {
    let linear = if linear.is_nan() { 0.0 } else { linear };
    // At most 255 thresholds lie below any value, so the count fits a byte.
    thresholds().partition_point(|threshold| *threshold <= linear) as u8
}

/// The nearest byte of a linear alpha in `0..=1`.
pub(crate) fn alpha(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_inverts_decoding_for_every_byte() {
        for byte in 0..=255u8 {
            assert_eq!(encode(decode(byte)), byte);
        }
        assert_eq!(encode(-1.0), 0);
        assert_eq!(encode(2.0), 255);
        assert_eq!(encode(f32::NAN), 0);
    }

    #[test]
    fn encoding_matches_the_color_conversion_of_the_runtime() {
        for step in 0..=1000 {
            let linear = step as f32 / 1000.0;
            let color = pert_ui_runtime::Color::rgba(linear, 0.0, 0.0, 1.0).unwrap();
            assert_eq!(encode(linear), color.to_srgb8()[0], "{linear}");
        }
    }
}
