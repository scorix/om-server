use std::collections::HashSet;

use serde::Deserialize;

use crate::domain::WeatherModelId;
use crate::error::OpenMeteoError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunManifest {
    pub reference_time: String,
    pub valid_times: Vec<String>,
}

impl RunManifest {
    pub fn spatial_run_prefix(&self, model: WeatherModelId) -> Result<String, OpenMeteoError> {
        let (year, month, day, run_ref) = Self::parse_reference_time(&self.reference_time)?;
        Ok(format!(
            "data_spatial/{}/{year:04}/{month:02}/{day:02}/{run_ref}/",
            model.as_str()
        ))
    }

    pub fn spatial_timestamps(&self) -> Result<HashSet<String>, OpenMeteoError> {
        self.valid_times
            .iter()
            .map(|valid_time| Self::valid_time_to_spatial_timestamp(valid_time))
            .collect()
    }

    fn parse_reference_time(
        reference_time: &str,
    ) -> Result<(i32, u32, u32, String), OpenMeteoError> {
        let (date, time) = reference_time.split_once('T').ok_or_else(|| {
            OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            }
        })?;
        let mut date_parts = date.split('-');
        let year = date_parts
            .next()
            .ok_or_else(|| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?
            .parse::<i32>()
            .map_err(|_| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?;
        let month = date_parts
            .next()
            .ok_or_else(|| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?
            .parse::<u32>()
            .map_err(|_| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?;
        let day = date_parts
            .next()
            .ok_or_else(|| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?
            .parse::<u32>()
            .map_err(|_| OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            })?;
        let compact_time = time.trim_end_matches('Z').replace(':', "");
        let run_ref = if compact_time.len() >= 4 {
            format!("{}Z", &compact_time[..4])
        } else {
            return Err(OpenMeteoError::InvalidManifestReferenceTime {
                reference_time: reference_time.to_string(),
            });
        };
        Ok((year, month, day, run_ref))
    }

    fn valid_time_to_spatial_timestamp(valid_time: &str) -> Result<String, OpenMeteoError> {
        let valid_time = valid_time.trim_end_matches('Z');
        let (date, time) =
            valid_time
                .split_once('T')
                .ok_or_else(|| OpenMeteoError::InvalidManifestValidTime {
                    valid_time: valid_time.to_string(),
                })?;
        Ok(format!("{date}T{}", time.replace(':', "")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::WeatherModelId;

    #[test]
    fn maps_reference_time_to_spatial_run_prefix() {
        let manifest = RunManifest {
            reference_time: "2026-06-07T06:00:00Z".to_string(),
            valid_times: Vec::new(),
        };
        assert_eq!(
            manifest
                .spatial_run_prefix(WeatherModelId::EcmwfIfs025)
                .expect("prefix"),
            "data_spatial/ecmwf_ifs025/2026/06/07/0600Z/"
        );
    }

    #[test]
    fn maps_manifest_valid_time_to_spatial_timestamp() {
        let manifest = RunManifest {
            reference_time: String::new(),
            valid_times: vec!["2026-06-07T06:00Z".to_string()],
        };
        let timestamps = manifest.spatial_timestamps().expect("timestamps");
        assert!(timestamps.contains("2026-06-07T0600"));
    }
}
