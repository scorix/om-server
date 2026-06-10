use crate::domain::weather_bake_layer::WeatherValueRange;

/// Maximum wind speed (m/s) used to encode U/V into PNG channels for particle layers.
pub const WIND_PARTICLE_MAX_SPEED_MS: f32 = 64.0;

/// Grayscale value reserved for "no data / transparent". Valid samples encode to 1..=255 so
/// the clients can colorize gray 1..255 and treat gray 0 as transparent.
pub const WEATHER_NODATA_GRAY: u8 = 0;

/// Quantizes a scalar value into a grayscale RGBA pixel (`R=G=B=gray`, `A=255`).
///
/// Colorization happens on the client (web `raster-color`, iOS Metal LUT) using the same
/// [`WeatherValueRange`]. `None` and out-of-domain / below-threshold values encode to the
/// transparent sentinel ([`WEATHER_NODATA_GRAY`], alpha 0).
pub fn encode_value_gray(value: Option<f32>, range: WeatherValueRange) -> [u8; 4] {
    let Some(value) = value else {
        return transparent();
    };
    if !value.is_finite() {
        return transparent();
    }
    if let Some(threshold) = range.transparent_at_or_below
        && value <= threshold
    {
        return transparent();
    }
    let span = range.max - range.min;
    let t = if span <= f32::EPSILON {
        0.0
    } else {
        ((value - range.min) / span).clamp(0.0, 1.0)
    };
    // Map valid values into 1..=255; 0 is reserved for transparency.
    let gray = 1 + (t * 254.0).round() as u8;
    [gray, gray, gray, 255]
}

fn transparent() -> [u8; 4] {
    [0, 0, 0, 0]
}

/// Encodes 10 m wind U/V (m/s) for client-side particle shaders.
///
/// R/G store normalized vector components around 128; B stores speed; A marks valid pixels.
pub fn wind_particle_rgba(u_ms: f32, v_ms: f32) -> [u8; 4] {
    if !u_ms.is_finite() || !v_ms.is_finite() {
        return [0, 0, 0, 0];
    }
    let speed = (u_ms * u_ms + v_ms * v_ms).sqrt();
    if speed < 0.05 {
        return [0, 0, 0, 0];
    }
    let scale = 127.0 / WIND_PARTICLE_MAX_SPEED_MS;
    let r = (128.0 + u_ms * scale).clamp(0.0, 255.0) as u8;
    let g = (128.0 + v_ms * scale).clamp(0.0, 255.0) as u8;
    let b = (speed / WIND_PARTICLE_MAX_SPEED_MS * 255.0).clamp(0.0, 255.0) as u8;
    [r, g, b, 255]
}

#[cfg(test)]
mod tests {
    use super::{encode_value_gray, wind_particle_rgba};
    use crate::domain::weather_bake_layer::WeatherValueRange;

    fn range(min: f32, max: f32, transparent_at_or_below: Option<f32>) -> WeatherValueRange {
        WeatherValueRange {
            min,
            max,
            transparent_at_or_below,
        }
    }

    #[test]
    fn wind_particle_encodes_calm_as_transparent() {
        assert_eq!(wind_particle_rgba(0.0, 0.0), [0, 0, 0, 0]);
    }

    #[test]
    fn wind_particle_encodes_east_wind_in_red_channel() {
        let rgba = wind_particle_rgba(10.0, 0.0);
        assert!(rgba[0] > 128);
        assert_eq!(rgba[3], 255);
    }

    #[test]
    fn encode_value_gray_marks_missing_as_transparent() {
        assert_eq!(
            encode_value_gray(None, range(0.0, 100.0, None)),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn encode_value_gray_marks_below_threshold_as_transparent() {
        assert_eq!(
            encode_value_gray(Some(0.0), range(0.0, 20.0, Some(0.0))),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn encode_value_gray_maps_valid_value_into_opaque_gray() {
        let pixel = encode_value_gray(Some(0.0), range(-30.0, 40.0, None));
        assert_eq!(pixel[3], 255);
        assert_eq!(pixel[0], pixel[1]);
        assert_eq!(pixel[1], pixel[2]);
        assert!(pixel[0] >= 1);
    }

    #[test]
    fn encode_value_gray_clamps_above_max_to_full_scale() {
        let pixel = encode_value_gray(Some(999.0), range(0.0, 100.0, None));
        assert_eq!(pixel, [255, 255, 255, 255]);
    }
}
