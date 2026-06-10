use std::str::FromStr;

/// Raster weather tile layers baked from spatial `.om` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherBakeLayer {
    Temperature2m,
    CloudCover,
    Snowfall,
    Wind,
    SnowDepth,
    Visibility,
    ShortwaveRadiation,
}

/// Physical value range a scalar layer is quantized into when baking grayscale tiles.
///
/// The same `[min, max]` must be mirrored by the clients' colormaps (web `raster-color`
/// range and the iOS Metal colormap LUT) so encoded grays decode to the same value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherValueRange {
    pub min: f32,
    pub max: f32,
    /// Values at or below this are rendered transparent (e.g. snowfall/snow depth ≤ 0).
    pub transparent_at_or_below: Option<f32>,
}

impl WeatherBakeLayer {
    pub fn id(self) -> &'static str {
        match self {
            Self::Temperature2m => "temperature_2m",
            Self::CloudCover => "cloud_cover",
            Self::Snowfall => "snowfall",
            Self::Wind => "wind",
            Self::SnowDepth => "snow_depth",
            Self::Visibility => "visibility",
            Self::ShortwaveRadiation => "shortwave_radiation",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "temperature_2m" => Some(Self::Temperature2m),
            "cloud_cover" => Some(Self::CloudCover),
            "snowfall" | "snowfall_water_equivalent" => Some(Self::Snowfall),
            "wind" | "wind_particles" => Some(Self::Wind),
            "snow_depth" => Some(Self::SnowDepth),
            "visibility" => Some(Self::Visibility),
            "shortwave_radiation" => Some(Self::ShortwaveRadiation),
            _ => None,
        }
    }

    /// Primary scalar variable in the spatial file, when applicable.
    pub fn spatial_variable(self) -> Option<&'static str> {
        match self {
            Self::Temperature2m => Some("temperature_2m"),
            Self::CloudCover => Some("cloud_cover"),
            Self::Snowfall => Some("snowfall_water_equivalent"),
            Self::SnowDepth => Some("snow_depth"),
            Self::Visibility => Some("visibility"),
            Self::ShortwaveRadiation => Some("shortwave_radiation"),
            Self::Wind => None,
        }
    }

    pub fn wind_spatial_variables(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Wind => Some(("wind_u_component_10m", "wind_v_component_10m")),
            _ => None,
        }
    }

    /// Value range used to quantize scalar values into grayscale tiles (`None` for wind,
    /// which uses its own U/V particle encoding).
    pub fn value_range(self) -> Option<WeatherValueRange> {
        let range = |min, max, transparent_at_or_below| WeatherValueRange {
            min,
            max,
            transparent_at_or_below,
        };
        match self {
            Self::Temperature2m => Some(range(-30.0, 40.0, None)),
            Self::CloudCover => Some(range(0.0, 100.0, None)),
            Self::Snowfall => Some(range(0.0, 20.0, Some(0.0))),
            Self::SnowDepth => Some(range(0.0, 3.0, Some(0.0))),
            // ECMWF visibility is in meters (0–24 km clamped).
            Self::Visibility => Some(range(0.0, 24_000.0, None)),
            Self::ShortwaveRadiation => Some(range(0.0, 1_000.0, None)),
            Self::Wind => None,
        }
    }
}

impl FromStr for WeatherBakeLayer {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::from_id(value).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::WeatherBakeLayer;

    #[test]
    fn snowfall_accepts_api_and_s3_aliases() {
        assert_eq!(
            WeatherBakeLayer::from_id("snowfall"),
            Some(WeatherBakeLayer::Snowfall)
        );
        assert_eq!(
            WeatherBakeLayer::from_id("snowfall_water_equivalent"),
            Some(WeatherBakeLayer::Snowfall)
        );
    }
}
