pub mod dwd;
pub mod ecmwf;
pub mod gfs;

use crate::domain::{SourceRegistry, WeatherDataSource};
use crate::error::{DataSourceError, OpenMeteoError, TimestampParseError};

pub fn source_registry() -> SourceRegistry {
    SourceRegistry::new(default_sources())
}

pub fn default_sources() -> Vec<Box<dyn WeatherDataSource>> {
    vec![
        Box::new(ecmwf::EcmwfIfs025Source),
        Box::new(gfs::Gfs025Source),
        Box::new(dwd::DwdIconSource),
    ]
}

const OPEN_METEO_S3_BASE_URL: &str = "https://openmeteo.s3.amazonaws.com";

/// Lists variable folder names under `data/{model_path}/` on the public Open-Meteo bucket.
///
/// Same information as the [S3 Explorer index](https://openmeteo.s3.amazonaws.com/index.html).
pub fn list_published_variables(model_path: &str) -> Result<Vec<String>, OpenMeteoError> {
    let prefix = format!("data/{model_path}/");
    let body = list_s3(&prefix, Some("/"))?;
    Ok(extract_xml_values(&body, "Prefix")
        .into_iter()
        .filter_map(|value| {
            value
                .strip_prefix(&prefix)
                .and_then(|name| name.strip_suffix('/'))
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .collect())
}

fn list_s3(prefix: &str, delimiter: Option<&str>) -> Result<String, OpenMeteoError> {
    use std::io::Read;

    let mut url = format!(
        "{OPEN_METEO_S3_BASE_URL}/?list-type=2&prefix={}",
        url_encode(prefix)
    );
    if let Some(delimiter) = delimiter {
        url.push_str("&delimiter=");
        url.push_str(&url_encode(delimiter));
    }
    let mut response = ureq::get(&url)
        .call()
        .map_err(|source| OpenMeteoError::ListRequest {
            url: url.clone(),
            source,
        })?;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|source| OpenMeteoError::ReadListResponse { url, source })?;
    Ok(body)
}

fn extract_xml_values(body: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find(&open) {
        let value_start = start + open.len();
        let Some(end) = rest[value_start..].find(&close) else {
            break;
        };
        values.push(xml_unescape(&rest[value_start..value_start + end]));
        rest = &rest[value_start + end + close.len()..];
    }
    values
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

pub(crate) fn spatial_object_key(
    model_path: &str,
    run_ref: &str,
    timestamp: &str,
) -> Result<crate::domain::ObjectKey, DataSourceError> {
    let (year, month, day) = parse_spatial_timestamp(timestamp)?;
    Ok(crate::domain::ObjectKey(format!(
        "data_spatial/{model_path}/{year:04}/{month:02}/{day:02}/{run_ref}/{timestamp}.om"
    )))
}

fn parse_spatial_timestamp(timestamp: &str) -> Result<(i32, u32, u32), TimestampParseError> {
    let date = timestamp
        .split('T')
        .next()
        .ok_or_else(|| TimestampParseError::InvalidFormat {
            timestamp: timestamp.to_string(),
        })?;
    let mut parts = date.split('-');
    let year = parts
        .next()
        .ok_or_else(|| TimestampParseError::MissingYear {
            timestamp: timestamp.to_string(),
        })?
        .parse::<i32>()
        .map_err(|source| TimestampParseError::ParseYear {
            timestamp: timestamp.to_string(),
            source,
        })?;
    let month = parts
        .next()
        .ok_or_else(|| TimestampParseError::MissingMonth {
            timestamp: timestamp.to_string(),
        })?
        .parse::<u32>()
        .map_err(|source| TimestampParseError::ParseMonth {
            timestamp: timestamp.to_string(),
            source,
        })?;
    let day = parts
        .next()
        .ok_or_else(|| TimestampParseError::MissingDay {
            timestamp: timestamp.to_string(),
        })?
        .parse::<u32>()
        .map_err(|source| TimestampParseError::ParseDay {
            timestamp: timestamp.to_string(),
            source,
        })?;
    Ok((year, month, day))
}

pub(crate) fn standard_variable_name(
    element: crate::domain::WeatherElement,
) -> Option<&'static str> {
    use crate::domain::WeatherElement;

    Some(match element {
        WeatherElement::Temperature2m => "temperature_2m",
        WeatherElement::DewPoint2m => "dew_point_2m",
        WeatherElement::RelativeHumidity2m => "relative_humidity_2m",
        WeatherElement::ApparentTemperature => "apparent_temperature",
        WeatherElement::Precipitation => "precipitation",
        WeatherElement::PrecipitationProbability => "precipitation_probability",
        WeatherElement::Rain => "rain",
        WeatherElement::Snowfall => "snowfall",
        WeatherElement::SnowDepth => "snow_depth",
        WeatherElement::SnowWaterEquivalent => "snow_water_equivalent",
        WeatherElement::FreezingLevelHeight => "freezing_level_height",
        WeatherElement::WeatherCode => "weather_code",
        WeatherElement::WindSpeed10m => "wind_speed_10m",
        WeatherElement::WindDirection10m => "wind_direction_10m",
        WeatherElement::WindGusts10m => "wind_gusts_10m",
        WeatherElement::WindSpeed80m => "wind_speed_80m",
        WeatherElement::WindGusts80m => "wind_gusts_80m",
        WeatherElement::Visibility => "visibility",
        WeatherElement::CloudCover => "cloud_cover",
        WeatherElement::CloudCoverLow => "cloud_cover_low",
        WeatherElement::CloudCoverMid => "cloud_cover_mid",
        WeatherElement::CloudCoverHigh => "cloud_cover_high",
        WeatherElement::SurfaceTemperature => "surface_temperature",
        WeatherElement::ShortwaveRadiation => "shortwave_radiation",
        WeatherElement::SunshineDuration => "sunshine_duration",
        WeatherElement::Cape => "cape",
        WeatherElement::LiftedIndex => "lifted_index",
        WeatherElement::PressureLevelTemperature
        | WeatherElement::PressureLevelRelativeHumidity
        | WeatherElement::PressureLevelWindSpeed
        | WeatherElement::PressureLevelWindDirection => return None,
    })
}
