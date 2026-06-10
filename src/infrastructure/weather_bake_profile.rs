use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::Deserialize;

use crate::domain::WeatherBakeLayer;
use crate::domain::WeatherModelId;
use crate::error::WeatherBakeError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherBakeLayerSpec {
    pub layer: WeatherBakeLayer,
    pub model: WeatherModelId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherBakeProfile {
    pub layers: Vec<WeatherBakeLayerSpec>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct WeatherBakeProfileFile {
    layers: Vec<WeatherBakeLayerEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct WeatherBakeLayerEntry {
    variable: String,
    model: Option<String>,
}

pub fn load_weather_bake_profile(path: &Path) -> Result<WeatherBakeProfile, WeatherBakeError> {
    let raw = fs::read_to_string(path).map_err(|source| WeatherBakeError::ReadConfig {
        path: path.to_path_buf(),
        source,
    })?;
    let file: WeatherBakeProfileFile =
        toml::from_str(&raw).map_err(|source| WeatherBakeError::ParseConfig {
            path: path.to_path_buf(),
            source,
        })?;
    resolve_profile(path, file)
}

fn resolve_profile(
    path: &Path,
    file: WeatherBakeProfileFile,
) -> Result<WeatherBakeProfile, WeatherBakeError> {
    if file.layers.is_empty() {
        return Err(WeatherBakeError::EmptyLayers {
            path: path.to_path_buf(),
        });
    }

    let mut seen = HashSet::new();
    let mut layers = Vec::with_capacity(file.layers.len());
    for entry in file.layers {
        let Some(layer) = WeatherBakeLayer::from_id(entry.variable.trim()) else {
            return Err(WeatherBakeError::UnknownVariable {
                path: path.to_path_buf(),
                variable: entry.variable,
            });
        };
        if !seen.insert(layer) {
            return Err(WeatherBakeError::DuplicateVariable {
                path: path.to_path_buf(),
                variable: layer.id().to_string(),
            });
        }
        let Some(model_value) = entry.model else {
            return Err(WeatherBakeError::MissingLayerModel {
                path: path.to_path_buf(),
                variable: layer.id().to_string(),
            });
        };
        let model = parse_model(path, model_value.trim(), layer.id())?;
        layers.push(WeatherBakeLayerSpec { layer, model });
    }

    Ok(WeatherBakeProfile { layers })
}

fn parse_model(
    path: &Path,
    value: &str,
    variable: &str,
) -> Result<WeatherModelId, WeatherBakeError> {
    WeatherModelId::from_str(value).map_err(|_| WeatherBakeError::UnknownModel {
        path: path.to_path_buf(),
        variable: variable.to_string(),
        model: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{WeatherBakeLayerSpec, load_weather_bake_profile, resolve_profile};
    use crate::domain::WeatherBakeLayer;
    use crate::domain::WeatherModelId;
    use crate::error::WeatherBakeError;
    use crate::infrastructure::weather_bake_profile::WeatherBakeProfileFile;
    use std::path::Path;

    #[test]
    fn resolve_profile_requires_model_on_every_layer() {
        let error = resolve_profile(
            Path::new("weather_bake.toml"),
            WeatherBakeProfileFile {
                layers: vec![super::WeatherBakeLayerEntry {
                    variable: "temperature_2m".to_string(),
                    model: None,
                }],
            },
        )
        .expect_err("missing model");

        assert!(matches!(
            error,
            WeatherBakeError::MissingLayerModel { variable, .. } if variable == "temperature_2m"
        ));
    }

    #[test]
    fn resolve_profile_accepts_explicit_models() {
        let profile = resolve_profile(
            Path::new("weather_bake.toml"),
            WeatherBakeProfileFile {
                layers: vec![
                    super::WeatherBakeLayerEntry {
                        variable: "temperature_2m".to_string(),
                        model: Some("ecmwf_ifs".to_string()),
                    },
                    super::WeatherBakeLayerEntry {
                        variable: "snow_depth".to_string(),
                        model: Some("ecmwf_ifs025".to_string()),
                    },
                ],
            },
        )
        .expect("profile");

        assert_eq!(
            profile.layers,
            vec![
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::Temperature2m,
                    model: WeatherModelId::EcmwfIfs,
                },
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::SnowDepth,
                    model: WeatherModelId::EcmwfIfs025,
                },
            ]
        );
    }

    #[test]
    fn load_default_config_includes_new_tile_layers() {
        let profile = load_weather_bake_profile(Path::new("config/weather_bake.toml"))
            .expect("default config");
        let ids: Vec<_> = profile.layers.iter().map(|spec| spec.layer.id()).collect();
        assert!(ids.contains(&"snow_depth"));
        assert!(ids.contains(&"visibility"));
        assert!(ids.contains(&"shortwave_radiation"));
        assert!(
            profile
                .layers
                .iter()
                .all(|spec| !spec.model.as_str().is_empty())
        );
    }
}
