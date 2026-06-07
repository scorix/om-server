use std::collections::BTreeMap;
use std::collections::HashSet;
use std::io::{self, Read};
use std::path::PathBuf;

use om_server::domain::{
    DataLayout, DatasetLocation, DatasetReader, WeatherElement, WeatherModelId,
};
use om_server::error::{DatasetError, HttpError, OpenMeteoError};
use om_server::infrastructure::http::{HttpClient, UreqHttpClient};
use om_server::infrastructure::{OmfilesDatasetReader, open_meteo};
use omfiles::OmDataType;
use omfiles::reader::OmFileReader;
use omfiles::traits::{
    OmArrayVariable, OmFileReadable, OmFileReaderBackend, OmFileVariable, OmScalarVariable,
};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
enum RealS3TestError {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error(transparent)]
    Dataset(#[from] DatasetError),

    #[error(transparent)]
    OpenMeteo(#[from] OpenMeteoError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("S3 URL missing filename")]
    MissingFilename,

    #[error("S3 URL filename is not an .om file")]
    NotOmFile,

    #[error("parse valid time from {filename}")]
    InvalidValidTime { filename: String },

    #[error("expected HHMM valid time in {filename}")]
    InvalidHourMinute { filename: String },

    #[error("parse object key from {url}")]
    InvalidObjectKey { url: String },

    #[error("no S3 prefixes under {prefix}")]
    NoS3Prefixes { prefix: String },

    #[error("no latest spatial URL under {prefix}")]
    NoSpatialObject { prefix: String },

    #[error("no complete {steps}-step day under {prefix}")]
    NoCompleteDay { prefix: String, steps: usize },

    #[error("valid time missing date")]
    MissingValidDate,

    #[error("need at least two values to compute variance")]
    InsufficientVarianceSample,

    #[error("missing first valid time")]
    MissingFirstValidTime,

    #[error("missing last valid time")]
    MissingLastValidTime,

    #[error("Open-Meteo API response missing hourly.time")]
    MissingApiTimes,

    #[error("Open-Meteo API response missing hourly.temperature_2m")]
    MissingApiTemperatures,

    #[error("Open-Meteo API response missing time {valid_time}")]
    MissingApiTime { valid_time: String },

    #[error("Open-Meteo API missing temperature at {valid_time}")]
    MissingApiTemperature { valid_time: String },

    #[error("GET Open-Meteo API {url}")]
    OpenMeteoApiRequest {
        url: String,
        #[source]
        source: ureq::Error,
    },

    #[error("read Open-Meteo API response {url}")]
    OpenMeteoApiRead {
        url: String,
        #[source]
        source: io::Error,
    },
}

const S3_BASE_URL: &str = "https://openmeteo.s3.amazonaws.com";
const SHANGHAI_LAT: f64 = 31.2304;
const SHANGHAI_LON: f64 = 121.4737;
const EXPECTED_DAILY_SEQUENCE_STEPS: usize = 8;
const MAX_DELTA_VARIANCE_C2: f64 = 0.25;

#[derive(Debug)]
struct SpatialDaySequence {
    run_prefix: String,
    valid_date: String,
    urls: Vec<String>,
}

#[test]
#[ignore = "hits the public Open-Meteo S3 bucket"]
fn real_shanghai_temperature_sequence_matches_open_meteo_shape() {
    let sequence =
        discover_latest_ecmwf_spatial_day_urls().expect("discover real Open-Meteo day sequence");
    let valid_times = sequence
        .urls
        .iter()
        .map(|url| valid_time_from_spatial_url(url))
        .collect::<Result<Vec<_>, RealS3TestError>>()
        .expect("valid times from S3 URLs");
    eprintln!("run_prefix={}", sequence.run_prefix);
    eprintln!("valid_date={}", sequence.valid_date);
    eprintln!("coordinate=lat:{SHANGHAI_LAT},lon:{SHANGHAI_LON}");
    eprintln!("valid_times={valid_times:?}");
    let api_temperatures =
        fetch_open_meteo_temperatures(&valid_times).expect("Open-Meteo API temperatures");
    let om_temperatures = read_shanghai_temperatures_from_om(&sequence.urls)
        .expect("read Shanghai temperatures from OM");
    let deltas = om_temperatures
        .iter()
        .zip(api_temperatures.iter())
        .map(|(om, api)| om - api)
        .collect::<Vec<_>>();
    let variance = sample_variance(&deltas).expect("delta variance");
    let mean_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;

    for ((time, om), api) in valid_times
        .iter()
        .zip(om_temperatures.iter())
        .zip(api_temperatures.iter())
    {
        eprintln!(
            "valid_time_utc={time} om_temperature={om} api_temperature={api} delta={}",
            om - api
        );
        assert!(
            (-100.0..=350.0).contains(om),
            "temperature should be plausible in Celsius or Kelvin"
        );
    }
    eprintln!("mean_delta={mean_delta}");
    eprintln!("delta_variance={variance}");
    assert!(
        variance <= MAX_DELTA_VARIANCE_C2,
        "OM/API temperature deltas should have low sequence variance"
    );
}

#[test]
#[ignore = "hits the public Open-Meteo S3 bucket"]
fn reads_real_spatial_elements_for_all_models() {
    let registry = open_meteo::OpenMeteoSources.registry();
    let catalog = open_meteo::OpenMeteoS3Catalog::default();
    for model in WeatherModelId::all() {
        let source = registry
            .get(*model)
            .unwrap_or_else(|| panic!("missing source for {model}"));
        let published = catalog
            .list_published_variables(model.as_str())
            .unwrap_or_else(|error| panic!("list S3 variables for {model}: {error}"))
            .into_iter()
            .collect::<HashSet<_>>();
        for element in source.supported_elements(DataLayout::Timeseries) {
            let Some(variable) = source.variable_name(DataLayout::Timeseries, *element) else {
                continue;
            };
            assert!(
                published.contains(variable),
                "adapter declares {variable} for {model} timeseries but S3 index has no data/{model}/{variable}/"
            );
        }

        let url = discover_latest_spatial_url(model.as_str()).expect("discover latest .om");
        let local_path = sync_om_from_url(&url).expect("sync spatial .om");
        let meta = OmfilesDatasetReader
            .read_meta(DatasetLocation::LocalFile(local_path.clone()))
            .expect("read spatial meta");
        let spatial_variables = meta
            .variables
            .into_iter()
            .map(|variable| variable.name)
            .collect::<HashSet<_>>();
        eprintln!(
            "model={model} spatial_variables={} timeseries_variables={}",
            spatial_variables.len(),
            published.len()
        );
        for element in source.supported_elements(DataLayout::Spatial) {
            let Some(variable) = source.variable_name(DataLayout::Spatial, *element) else {
                continue;
            };
            assert!(
                spatial_variables.contains(variable),
                "adapter declares spatial {variable} for {model} but latest .om has no such variable"
            );
        }

        eprintln!("url={url} local_path={}", local_path.display());
        for element in smoke_elements(*model) {
            let variable = source
                .variable_name(DataLayout::Spatial, *element)
                .unwrap_or_else(|| panic!("missing variable mapping for {model} {element:?}"));
            let value = OmfilesDatasetReader::read_spatial_point_from_local(
                &local_path,
                variable,
                SHANGHAI_LAT,
                SHANGHAI_LON,
            )
            .unwrap_or_else(|error| panic!("read {model} {variable}: {error}"));
            eprintln!("  element={element:?} variable={variable} value={value}");
            assert_plausible(*element, value);
        }
    }
}

fn smoke_elements(model: WeatherModelId) -> &'static [WeatherElement] {
    use WeatherElement::*;
    match model {
        WeatherModelId::EcmwfIfs025 => &[Temperature2m],
        WeatherModelId::Gfs025 => &[Visibility],
        WeatherModelId::DwdIcon => &[Temperature2m],
    }
}

#[test]
#[ignore = "hits the public Open-Meteo S3 bucket"]
fn prints_real_ecmwf_spatial_variable_tree() {
    let url = std::env::var("OM_SERVER_REAL_S3_URL").unwrap_or_else(|_| {
        discover_latest_spatial_url(WeatherModelId::EcmwfIfs025.as_str())
            .expect("discover real Open-Meteo object")
    });
    let local_path = sync_om_from_url(&url).expect("sync spatial .om");
    let reader = OmFileReader::from_file(
        local_path
            .to_str()
            .expect("local cache path should be UTF-8"),
    )
    .expect("om reader");

    eprintln!("url={url} local_path={}", local_path.display());
    dump_tree(&reader, 0);
}

fn read_shanghai_temperatures_from_om(urls: &[String]) -> Result<Vec<f64>, RealS3TestError> {
    let handles = urls
        .iter()
        .map(|url| {
            let url = url.clone();
            std::thread::spawn(move || {
                eprintln!("read_om_start={url}");
                let temperature = read_shanghai_temperature_from_om(&url)?;
                eprintln!("read_om_done={url} temperature={temperature}");
                Ok::<_, RealS3TestError>(temperature)
            })
        })
        .collect::<Vec<_>>();

    handles
        .into_iter()
        .map(|handle| handle.join().expect("OM reader thread panicked"))
        .collect()
}

fn valid_time_from_spatial_url(url: &str) -> Result<String, RealS3TestError> {
    let filename = url
        .rsplit('/')
        .next()
        .ok_or(RealS3TestError::MissingFilename)?
        .strip_suffix(".om")
        .ok_or(RealS3TestError::NotOmFile)?;
    let (date, hour_minute) =
        filename
            .split_once('T')
            .ok_or_else(|| RealS3TestError::InvalidValidTime {
                filename: filename.to_string(),
            })?;
    if hour_minute.len() != 4 {
        return Err(RealS3TestError::InvalidHourMinute {
            filename: filename.to_string(),
        });
    }
    Ok(format!(
        "{date}T{}:{}",
        &hour_minute[0..2],
        &hour_minute[2..4]
    ))
}

fn read_shanghai_temperature_from_om(url: &str) -> Result<f64, RealS3TestError> {
    let local_path = sync_om_from_url(url)?;
    Ok(OmfilesDatasetReader::read_spatial_point_from_local(
        &local_path,
        "temperature_2m",
        SHANGHAI_LAT,
        SHANGHAI_LON,
    )?)
}

fn om_cache_dir() -> PathBuf {
    std::env::var("OM_SERVER_TEST_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("om-server-real-s3-cache"))
}

fn sync_om_from_url(url: &str) -> Result<PathBuf, RealS3TestError> {
    let object_key = url
        .strip_prefix(S3_BASE_URL)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .ok_or_else(|| RealS3TestError::InvalidObjectKey {
            url: url.to_string(),
        })?;
    let path = om_cache_dir().join(object_key);
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| {
            RealS3TestError::Http(HttpError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })
        })?;
    }
    UreqHttpClient
        .download_to(url, &path)
        .map_err(RealS3TestError::Http)?;
    Ok(path)
}

fn assert_plausible(element: WeatherElement, value: f64) {
    assert!(
        value.is_finite(),
        "{element:?} should be finite, got {value}"
    );
    match element {
        WeatherElement::Temperature2m
        | WeatherElement::DewPoint2m
        | WeatherElement::ApparentTemperature
        | WeatherElement::SurfaceTemperature => {
            assert!((-100.0..=350.0).contains(&value), "{element:?}={value}");
        }
        WeatherElement::RelativeHumidity2m
        | WeatherElement::CloudCover
        | WeatherElement::CloudCoverLow
        | WeatherElement::CloudCoverMid
        | WeatherElement::CloudCoverHigh
        | WeatherElement::PrecipitationProbability => {
            assert!((0.0..=100.0).contains(&value), "{element:?}={value}");
        }
        WeatherElement::WindDirection10m | WeatherElement::PressureLevelWindDirection => {
            assert!((0.0..=360.0).contains(&value), "{element:?}={value}");
        }
        WeatherElement::WeatherCode => {
            assert!((0.0..=100.0).contains(&value), "{element:?}={value}")
        }
        WeatherElement::Precipitation
        | WeatherElement::Rain
        | WeatherElement::Snowfall
        | WeatherElement::SnowDepth
        | WeatherElement::SnowWaterEquivalent
        | WeatherElement::WindSpeed10m
        | WeatherElement::WindGusts10m
        | WeatherElement::WindSpeed80m
        | WeatherElement::WindGusts80m
        | WeatherElement::WindUComponent10m
        | WeatherElement::WindVComponent10m
        | WeatherElement::Visibility
        | WeatherElement::ShortwaveRadiation
        | WeatherElement::SunshineDuration
        | WeatherElement::Cape
        | WeatherElement::LiftedIndex
        | WeatherElement::PressureLevelWindSpeed => {
            assert!(value >= 0.0, "{element:?}={value}");
        }
        WeatherElement::FreezingLevelHeight | WeatherElement::PressureLevelTemperature => {
            assert!((-500.0..=20_000.0).contains(&value), "{element:?}={value}");
        }
        WeatherElement::PressureLevelRelativeHumidity => {
            assert!((0.0..=100.0).contains(&value), "{element:?}={value}");
        }
    }
}

fn fetch_open_meteo_temperatures(valid_times: &[String]) -> Result<Vec<f64>, RealS3TestError> {
    let start_date = valid_times
        .first()
        .and_then(|time| time.split('T').next())
        .ok_or(RealS3TestError::MissingFirstValidTime)?;
    let end_date = valid_times
        .last()
        .and_then(|time| time.split('T').next())
        .ok_or(RealS3TestError::MissingLastValidTime)?;
    let url = format!(
        "https://api.open-meteo.com/v1/ecmwf?latitude={SHANGHAI_LAT}&longitude={SHANGHAI_LON}&models=ecmwf_ifs025&hourly=temperature_2m&temperature_unit=celsius&timezone=GMT&start_date={start_date}&end_date={end_date}"
    );
    let mut response =
        ureq::get(&url)
            .call()
            .map_err(|source| RealS3TestError::OpenMeteoApiRequest {
                url: url.clone(),
                source,
            })?;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|source| RealS3TestError::OpenMeteoApiRead {
            url: url.clone(),
            source,
        })?;
    let payload: Value = serde_json::from_str(&body)?;
    let times = payload
        .pointer("/hourly/time")
        .and_then(Value::as_array)
        .ok_or(RealS3TestError::MissingApiTimes)?;
    let temperatures = payload
        .pointer("/hourly/temperature_2m")
        .and_then(Value::as_array)
        .ok_or(RealS3TestError::MissingApiTemperatures)?;
    valid_times
        .iter()
        .map(|valid_time| {
            let index = times
                .iter()
                .position(|time| time.as_str() == Some(valid_time.as_str()))
                .ok_or_else(|| RealS3TestError::MissingApiTime {
                    valid_time: valid_time.clone(),
                })?;
            temperatures
                .get(index)
                .and_then(Value::as_f64)
                .ok_or_else(|| RealS3TestError::MissingApiTemperature {
                    valid_time: valid_time.clone(),
                })
        })
        .collect()
}

fn dump_tree<B>(reader: &OmFileReader<B>, depth: usize)
where
    B: OmFileReaderBackend,
{
    let indent = "  ".repeat(depth);
    let name = if reader.name().is_empty() {
        "<root>"
    } else {
        reader.name()
    };
    if reader.data_type().is_array() {
        if let Ok(array) = reader.expect_array() {
            eprintln!(
                "{indent}{name}: {:?} dims={:?} chunks={:?} compression={:?} scale_factor={} add_offset={} children={}",
                reader.data_type(),
                array.get_dimensions(),
                array.get_chunk_dimensions(),
                array.compression(),
                array.scale_factor(),
                array.add_offset(),
                reader.number_of_children()
            );
        }
    } else if reader.data_type().is_scalar() {
        let value = scalar_value(reader).unwrap_or_else(|| "<unreadable>".to_string());
        eprintln!(
            "{indent}{name}: {:?} value={value:?} children={}",
            reader.data_type(),
            reader.number_of_children()
        );
    } else {
        eprintln!(
            "{indent}{name}: {:?} children={}",
            reader.data_type(),
            reader.number_of_children()
        );
    }

    for index in 0..reader.number_of_children() {
        if let Some(child) = reader.get_child_by_index(index) {
            dump_tree(&child, depth + 1);
        }
    }
}

fn scalar_value<B>(reader: &OmFileReader<B>) -> Option<String>
where
    B: OmFileReaderBackend,
{
    let scalar = reader.expect_scalar().ok()?;
    match reader.data_type() {
        OmDataType::Int8 => scalar.read_scalar::<i8>().map(|value| value.to_string()),
        OmDataType::Uint8 => scalar.read_scalar::<u8>().map(|value| value.to_string()),
        OmDataType::Int16 => scalar.read_scalar::<i16>().map(|value| value.to_string()),
        OmDataType::Uint16 => scalar.read_scalar::<u16>().map(|value| value.to_string()),
        OmDataType::Int32 => scalar.read_scalar::<i32>().map(|value| value.to_string()),
        OmDataType::Uint32 => scalar.read_scalar::<u32>().map(|value| value.to_string()),
        OmDataType::Int64 => scalar.read_scalar::<i64>().map(|value| value.to_string()),
        OmDataType::Uint64 => scalar.read_scalar::<u64>().map(|value| value.to_string()),
        OmDataType::Float => scalar.read_scalar::<f32>().map(|value| value.to_string()),
        OmDataType::Double => scalar.read_scalar::<f64>().map(|value| value.to_string()),
        OmDataType::String => scalar.read_scalar::<String>(),
        _ => None,
    }
}

fn discover_latest_spatial_url(model: &str) -> Result<String, RealS3TestError> {
    let run_prefix = latest_spatial_run_prefix(model)?;
    let mut keys = list_object_keys(&run_prefix)?
        .into_iter()
        .filter(|key| key.ends_with(".om") && !key.contains("_model-level"))
        .collect::<Vec<_>>();
    keys.sort();
    keys.into_iter()
        .next_back()
        .map(|key| format!("{S3_BASE_URL}/{key}"))
        .ok_or_else(|| RealS3TestError::NoSpatialObject { prefix: run_prefix })
}

fn discover_latest_ecmwf_spatial_day_urls() -> Result<SpatialDaySequence, RealS3TestError> {
    let run_prefix = latest_spatial_run_prefix(WeatherModelId::EcmwfIfs025.as_str())?;
    let mut keys_by_date = BTreeMap::<String, Vec<String>>::new();
    for key in list_object_keys(&run_prefix)?
        .into_iter()
        .filter(|key| key.ends_with(".om") && !key.contains("_model-level"))
    {
        let valid_time = valid_time_from_spatial_url(&key)?;
        let valid_date = valid_time
            .split('T')
            .next()
            .ok_or(RealS3TestError::MissingValidDate)?
            .to_string();
        keys_by_date.entry(valid_date).or_default().push(key);
    }

    let (valid_date, mut keys) = keys_by_date
        .into_iter()
        .rev()
        .find(|(_, keys)| keys.len() == EXPECTED_DAILY_SEQUENCE_STEPS)
        .ok_or_else(|| RealS3TestError::NoCompleteDay {
            prefix: run_prefix.clone(),
            steps: EXPECTED_DAILY_SEQUENCE_STEPS,
        })?;
    keys.sort();
    Ok(SpatialDaySequence {
        run_prefix,
        valid_date,
        urls: keys
            .into_iter()
            .map(|key| format!("{S3_BASE_URL}/{key}"))
            .collect(),
    })
}

fn latest_spatial_run_prefix(model: &str) -> Result<String, RealS3TestError> {
    let spatial_prefix = format!("data_spatial/{model}/");
    let year_prefix = latest_common_prefix(&spatial_prefix)?;
    let month_prefix = latest_common_prefix(&year_prefix)?;
    let day_prefix = latest_common_prefix(&month_prefix)?;
    latest_common_prefix(&day_prefix)
}

fn latest_common_prefix(prefix: &str) -> Result<String, RealS3TestError> {
    list_common_prefixes(prefix)?
        .into_iter()
        .max()
        .ok_or_else(|| RealS3TestError::NoS3Prefixes {
            prefix: prefix.to_string(),
        })
}

fn list_common_prefixes(prefix: &str) -> Result<Vec<String>, RealS3TestError> {
    let body = list_s3(prefix, Some("/"), None)?;
    Ok(extract_xml_values(&body, "Prefix")
        .into_iter()
        .filter(|value| value != prefix)
        .collect())
}

fn list_object_keys(prefix: &str) -> Result<Vec<String>, RealS3TestError> {
    let mut keys = Vec::new();
    let mut continuation_token = None;
    loop {
        let body = list_s3(prefix, None, continuation_token.as_deref())?;
        keys.extend(extract_xml_values(&body, "Key"));
        continuation_token = extract_xml_values(&body, "NextContinuationToken")
            .into_iter()
            .next();
        if continuation_token.is_none() {
            break;
        }
    }
    Ok(keys)
}

fn list_s3(
    prefix: &str,
    delimiter: Option<&str>,
    continuation_token: Option<&str>,
) -> Result<String, RealS3TestError> {
    let mut url = format!("{S3_BASE_URL}/?list-type=2&prefix={}", url_encode(prefix));
    if let Some(delimiter) = delimiter {
        url.push_str("&delimiter=");
        url.push_str(&url_encode(delimiter));
    }
    if let Some(token) = continuation_token {
        url.push_str("&continuation-token=");
        url.push_str(&url_encode(token));
    }
    let mut response = ureq::get(&url).call().map_err(|source| {
        RealS3TestError::Http(HttpError::Request {
            url: url.clone(),
            source,
        })
    })?;
    let mut body = String::new();
    response
        .body_mut()
        .as_reader()
        .read_to_string(&mut body)
        .map_err(|source| RealS3TestError::Http(HttpError::ReadBody { url, source }))?;
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

fn sample_variance(values: &[f64]) -> Result<f64, RealS3TestError> {
    if values.len() < 2 {
        return Err(RealS3TestError::InsufficientVarianceSample);
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    Ok(values
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / (values.len() - 1) as f64)
}
