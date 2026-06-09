/// Maximum wind speed (m/s) used to encode U/V into PNG channels for particle layers.
pub const WIND_PARTICLE_MAX_SPEED_MS: f32 = 64.0;

/// Temperature colormap for weather tiles (values in °C).
pub fn temperature_rgba(celsius: f32) -> [u8; 4] {
    if !celsius.is_finite() {
        return [0, 0, 0, 0];
    }
    let stops: [(f32, [u8; 3]); 7] = [
        (-30.0, [49, 54, 149]),
        (-10.0, [69, 117, 180]),
        (0.0, [171, 217, 233]),
        (10.0, [166, 217, 106]),
        (20.0, [254, 224, 139]),
        (30.0, [244, 109, 67]),
        (40.0, [165, 0, 38]),
    ];

    if celsius <= stops[0].0 {
        return rgba(stops[0].1, 220);
    }
    if celsius >= stops[stops.len() - 1].0 {
        return rgba(stops[stops.len() - 1].1, 220);
    }

    for window in stops.windows(2) {
        let (left_temp, left_rgb) = window[0];
        let (right_temp, right_rgb) = window[1];
        if celsius >= left_temp && celsius <= right_temp {
            let span = right_temp - left_temp;
            let t = if span <= f32::EPSILON {
                0.0
            } else {
                (celsius - left_temp) / span
            };
            let rgb = [
                lerp_u8(left_rgb[0], right_rgb[0], t),
                lerp_u8(left_rgb[1], right_rgb[1], t),
                lerp_u8(left_rgb[2], right_rgb[2], t),
            ];
            return rgba(rgb, 220);
        }
    }
    rgba(stops[stops.len() - 1].1, 220)
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    let from = f32::from(from);
    let to = f32::from(to);
    (from + (to - from) * t.clamp(0.0, 1.0)).round() as u8
}

fn rgba(rgb: [u8; 3], alpha: u8) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], alpha]
}

/// Total cloud cover (0–100 %).
pub fn cloud_cover_rgba(percent: f32) -> [u8; 4] {
    scalar_stops_rgba(
        percent,
        &[
            (0.0, [30, 58, 95]),
            (25.0, [100, 149, 199]),
            (50.0, [180, 198, 214]),
            (75.0, [220, 226, 232]),
            (100.0, [248, 250, 252]),
        ],
        200,
    )
}

/// Snowfall water equivalent (mm); typical hourly values are small.
pub fn snowfall_rgba(mm: f32) -> [u8; 4] {
    if mm <= 0.0 || !mm.is_finite() {
        return [0, 0, 0, 0];
    }
    scalar_stops_rgba(
        mm,
        &[
            (0.1, [224, 243, 255]),
            (1.0, [147, 197, 253]),
            (3.0, [59, 130, 246]),
            (8.0, [29, 78, 216]),
            (20.0, [30, 58, 138]),
        ],
        220,
    )
}

/// Snow depth (m).
pub fn snow_depth_rgba(depth_m: f32) -> [u8; 4] {
    if depth_m <= 0.0 || !depth_m.is_finite() {
        return [0, 0, 0, 0];
    }
    scalar_stops_rgba(
        depth_m,
        &[
            (0.05, [224, 242, 254]),
            (0.3, [186, 230, 253]),
            (0.8, [125, 211, 252]),
            (1.5, [56, 189, 248]),
            (3.0, [2, 132, 199]),
        ],
        220,
    )
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

fn scalar_stops_rgba(value: f32, stops: &[(f32, [u8; 3])], alpha: u8) -> [u8; 4] {
    if !value.is_finite() {
        return [0, 0, 0, 0];
    }
    if value <= stops[0].0 {
        return rgba(stops[0].1, alpha);
    }
    if value >= stops[stops.len() - 1].0 {
        return rgba(stops[stops.len() - 1].1, alpha);
    }
    for window in stops.windows(2) {
        let (left, left_rgb) = window[0];
        let (right, right_rgb) = window[1];
        if value >= left && value <= right {
            let span = right - left;
            let t = if span <= f32::EPSILON {
                0.0
            } else {
                (value - left) / span
            };
            let rgb = [
                lerp_u8(left_rgb[0], right_rgb[0], t),
                lerp_u8(left_rgb[1], right_rgb[1], t),
                lerp_u8(left_rgb[2], right_rgb[2], t),
            ];
            return rgba(rgb, alpha);
        }
    }
    rgba(stops[stops.len() - 1].1, alpha)
}

#[cfg(test)]
mod tests {
    use super::{cloud_cover_rgba, snowfall_rgba, wind_particle_rgba};

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
    fn cloud_cover_is_opaque_for_valid_percent() {
        assert_eq!(cloud_cover_rgba(50.0)[3], 200);
    }

    #[test]
    fn snowfall_is_transparent_for_zero() {
        assert_eq!(snowfall_rgba(0.0), [0, 0, 0, 0]);
    }
}
