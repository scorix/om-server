use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::error::ModelParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherModelId {
    EcmwfIfs,
    EcmwfIfs025,
    Gfs025,
    DwdIcon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherElement {
    Temperature2m,
    DewPoint2m,
    RelativeHumidity2m,
    ApparentTemperature,
    Precipitation,
    PrecipitationProbability,
    Rain,
    Snowfall,
    SnowDepth,
    SnowWaterEquivalent,
    FreezingLevelHeight,
    WeatherCode,
    WindSpeed10m,
    WindDirection10m,
    WindGusts10m,
    WindUComponent10m,
    WindVComponent10m,
    WindSpeed80m,
    WindGusts80m,
    Visibility,
    CloudCover,
    CloudCoverLow,
    CloudCoverMid,
    CloudCoverHigh,
    SurfaceTemperature,
    ShortwaveRadiation,
    SunshineDuration,
    PressureLevelTemperature,
    PressureLevelRelativeHumidity,
    PressureLevelWindSpeed,
    PressureLevelWindDirection,
    Cape,
    LiftedIndex,
    BoundaryLayerHeight,
    ConvectiveInhibition,
    DirectRadiation,
    PressureMsl,
    Showers,
    SoilMoisture0To7cm,
    SoilMoisture7To28cm,
    SoilMoisture28To100cm,
    SoilMoisture100To255cm,
    SoilTemperature0To7cm,
    SoilTemperature7To28cm,
    SoilTemperature28To100cm,
    SoilTemperature100To255cm,
    Temperature2mMax,
    Temperature2mMin,
    TotalColumnIntegratedWaterVapour,
    WindUComponent100m,
    WindUComponent200m,
    WindVComponent100m,
    WindVComponent200m,
    RoughnessLength,
    /// Passthrough for S3 variables without a dedicated variant (e.g. pressure levels).
    RawVariable(&'static str),
}

impl WeatherModelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EcmwfIfs => "ecmwf_ifs",
            Self::EcmwfIfs025 => "ecmwf_ifs025",
            Self::Gfs025 => "ncep_gfs025",
            Self::DwdIcon => "dwd_icon",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::EcmwfIfs,
            Self::EcmwfIfs025,
            Self::Gfs025,
            Self::DwdIcon,
        ]
    }
}

impl WeatherElement {
    pub const fn raw(name: &'static str) -> Self {
        Self::RawVariable(name)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RawVariable(name) => name,
            Self::Temperature2m => "temperature_2m",
            Self::DewPoint2m => "dew_point_2m",
            Self::RelativeHumidity2m => "relative_humidity_2m",
            Self::ApparentTemperature => "apparent_temperature",
            Self::Precipitation => "precipitation",
            Self::PrecipitationProbability => "precipitation_probability",
            Self::Rain => "rain",
            Self::Snowfall => "snowfall",
            Self::SnowDepth => "snow_depth",
            Self::SnowWaterEquivalent => "snow_water_equivalent",
            Self::FreezingLevelHeight => "freezing_level_height",
            Self::WeatherCode => "weather_code",
            Self::WindSpeed10m => "wind_speed_10m",
            Self::WindDirection10m => "wind_direction_10m",
            Self::WindGusts10m => "wind_gusts_10m",
            Self::WindUComponent10m => "wind_u_component_10m",
            Self::WindVComponent10m => "wind_v_component_10m",
            Self::WindSpeed80m => "wind_speed_80m",
            Self::WindGusts80m => "wind_gusts_80m",
            Self::Visibility => "visibility",
            Self::CloudCover => "cloud_cover",
            Self::CloudCoverLow => "cloud_cover_low",
            Self::CloudCoverMid => "cloud_cover_mid",
            Self::CloudCoverHigh => "cloud_cover_high",
            Self::SurfaceTemperature => "surface_temperature",
            Self::ShortwaveRadiation => "shortwave_radiation",
            Self::SunshineDuration => "sunshine_duration",
            Self::PressureLevelTemperature => "pressure_level_temperature",
            Self::PressureLevelRelativeHumidity => "pressure_level_relative_humidity",
            Self::PressureLevelWindSpeed => "pressure_level_wind_speed",
            Self::PressureLevelWindDirection => "pressure_level_wind_direction",
            Self::Cape => "cape",
            Self::LiftedIndex => "lifted_index",
            Self::BoundaryLayerHeight => "boundary_layer_height",
            Self::ConvectiveInhibition => "convective_inhibition",
            Self::DirectRadiation => "direct_radiation",
            Self::PressureMsl => "pressure_msl",
            Self::Showers => "showers",
            Self::SoilMoisture0To7cm => "soil_moisture_0_to_7cm",
            Self::SoilMoisture7To28cm => "soil_moisture_7_to_28cm",
            Self::SoilMoisture28To100cm => "soil_moisture_28_to_100cm",
            Self::SoilMoisture100To255cm => "soil_moisture_100_to_255cm",
            Self::SoilTemperature0To7cm => "soil_temperature_0_to_7cm",
            Self::SoilTemperature7To28cm => "soil_temperature_7_to_28cm",
            Self::SoilTemperature28To100cm => "soil_temperature_28_to_100cm",
            Self::SoilTemperature100To255cm => "soil_temperature_100_to_255cm",
            Self::Temperature2mMax => "temperature_2m_max",
            Self::Temperature2mMin => "temperature_2m_min",
            Self::TotalColumnIntegratedWaterVapour => "total_column_integrated_water_vapour",
            Self::WindUComponent100m => "wind_u_component_100m",
            Self::WindUComponent200m => "wind_u_component_200m",
            Self::WindVComponent100m => "wind_v_component_100m",
            Self::WindVComponent200m => "wind_v_component_200m",
            Self::RoughnessLength => "roughness_length",
        }
    }

    /// Open-Meteo fixed ratio when deriving API [`snowfall`] (cm) from model
    /// [`snowfall_water_equivalent`] (mm): 7 cm fresh snow ≈ 10 mm liquid water.
    ///
    /// [`snowfall`]: https://open-meteo.com/en/docs
    /// [`snowfall_water_equivalent`]: https://github.com/open-meteo/open-data
    pub const SNOWFALL_CM_PER_WATER_EQUIVALENT_MM: f64 = 7.0 / 10.0;

    pub fn open_meteo_s3_variable(self) -> Option<&'static str> {
        match self {
            Self::RawVariable(name) => Some(name),
            Self::PressureLevelTemperature
            | Self::PressureLevelRelativeHumidity
            | Self::PressureLevelWindSpeed
            | Self::PressureLevelWindDirection => None,
            // S3 carries model-native snowfall water equivalent (mm), not API name `snowfall`.
            Self::Snowfall => Some("snowfall_water_equivalent"),
            _ => Some(self.as_str()),
        }
    }

    /// Converts a raw S3 scalar to Open-Meteo API units for spatial gRPC responses.
    ///
    /// [`Self::Snowfall`]: S3 stores `snowfall_water_equivalent` in mm; multiply by
    /// [`Self::SNOWFALL_CM_PER_WATER_EQUIVALENT_MM`] (7/10) to match API `snowfall` in cm.
    pub fn normalize_spatial_value(self, raw: f64) -> f64 {
        match self {
            Self::Snowfall => raw * Self::SNOWFALL_CM_PER_WATER_EQUIVALENT_MM,
            _ => raw,
        }
    }
}

impl Display for WeatherModelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WeatherModelId {
    type Err = ModelParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ecmwf_ifs" => Ok(Self::EcmwfIfs),
            "ecmwf_ifs025" => Ok(Self::EcmwfIfs025),
            "ncep_gfs025" | "gfs025" => Ok(Self::Gfs025),
            "dwd_icon" => Ok(Self::DwdIcon),
            other => Err(ModelParseError {
                value: other.to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WeatherElement;

    #[test]
    fn raw_variable_uses_s3_name_verbatim() {
        let element = WeatherElement::raw("geopotential_height_500hPa");
        assert_eq!(element.as_str(), "geopotential_height_500hPa");
        assert_eq!(
            element.open_meteo_s3_variable(),
            Some("geopotential_height_500hPa")
        );
    }

    #[test]
    fn snowfall_maps_to_s3_variable_and_normalizes_to_api_cm() {
        assert_eq!(WeatherElement::Snowfall.as_str(), "snowfall");
        assert_eq!(
            WeatherElement::Snowfall.open_meteo_s3_variable(),
            Some("snowfall_water_equivalent")
        );
        assert_eq!(WeatherElement::Snowfall.normalize_spatial_value(10.0), 7.0);
    }
}
