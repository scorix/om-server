use std::str::FromStr;

use crate::domain::WeatherModelId;

/// Raster weather tile layers baked from spatial `.om` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherBakeLayer {
    Temperature2m,
    CloudCover,
    Snowfall,
    Wind,
    SnowDepth,
}

impl WeatherBakeLayer {
    pub fn id(self) -> &'static str {
        match self {
            Self::Temperature2m => "temperature_2m",
            Self::CloudCover => "cloud_cover",
            Self::Snowfall => "snowfall",
            Self::Wind => "wind",
            Self::SnowDepth => "snow_depth",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "temperature_2m" => Some(Self::Temperature2m),
            "cloud_cover" => Some(Self::CloudCover),
            "snowfall" | "snowfall_water_equivalent" => Some(Self::Snowfall),
            "wind" | "wind_particles" => Some(Self::Wind),
            "snow_depth" => Some(Self::SnowDepth),
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
            Self::Wind => None,
        }
    }

    pub fn wind_spatial_variables(self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Wind => Some(("wind_u_component_10m", "wind_v_component_10m")),
            _ => None,
        }
    }

    pub fn available_for_model(model: WeatherModelId) -> Vec<Self> {
        let mut layers = vec![
            Self::Temperature2m,
            Self::CloudCover,
            Self::Snowfall,
            Self::Wind,
        ];
        if matches!(model, WeatherModelId::EcmwfIfs025 | WeatherModelId::DwdIcon) {
            layers.push(Self::SnowDepth);
        }
        layers
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
    use crate::domain::WeatherModelId;

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

    #[test]
    fn ecmwf_ifs_excludes_snow_depth() {
        let layers = WeatherBakeLayer::available_for_model(WeatherModelId::EcmwfIfs);
        assert!(!layers.contains(&WeatherBakeLayer::SnowDepth));
        assert!(layers.contains(&WeatherBakeLayer::CloudCover));
    }

    #[test]
    fn ifs025_includes_snow_depth() {
        let layers = WeatherBakeLayer::available_for_model(WeatherModelId::EcmwfIfs025);
        assert!(layers.contains(&WeatherBakeLayer::SnowDepth));
    }
}
