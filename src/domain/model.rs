use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::error::ModelParseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherModelId {
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
}

impl WeatherModelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EcmwfIfs025 => "ecmwf_ifs025",
            Self::Gfs025 => "ncep_gfs025",
            Self::DwdIcon => "dwd_icon",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::EcmwfIfs025, Self::Gfs025, Self::DwdIcon]
    }
}

impl WeatherElement {
    pub fn as_str(self) -> &'static str {
        match self {
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
        }
    }

    pub fn open_meteo_s3_variable(self) -> Option<&'static str> {
        match self {
            Self::PressureLevelTemperature
            | Self::PressureLevelRelativeHumidity
            | Self::PressureLevelWindSpeed
            | Self::PressureLevelWindDirection => None,
            _ => Some(self.as_str()),
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
            "ecmwf_ifs025" => Ok(Self::EcmwfIfs025),
            "ncep_gfs025" | "gfs025" => Ok(Self::Gfs025),
            "dwd_icon" => Ok(Self::DwdIcon),
            other => Err(ModelParseError {
                value: other.to_string(),
            }),
        }
    }
}
