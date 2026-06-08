use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::domain::{
    DataLayout, DatasetLocation, DatasetReader, ObjectFetcher, ObjectKey, SourceRegistry,
    WeatherDataSource, WeatherElement, WeatherModelId,
};
use crate::error::{DatasetError, SpatialServiceError};
use crate::r#gen::{
    GetSpatialMetaRequest, GetSpatialMetaResponse, GetSpatialPointSeriesRequest,
    GetSpatialPointSeriesResponse, ListSourcesResponse, Source,
    SpatialElementValue as ProtoSpatialElementValue, SpatialPointSample as ProtoSpatialPointSample,
    VariableMeta as ProtoVariableMeta,
};
use crate::infrastructure::OmfilesDatasetReader;

pub struct SpatialService<F = crate::infrastructure::S3ObjectFetcher, R = OmfilesDatasetReader> {
    registry: SourceRegistry,
    fetcher: Arc<F>,
    dataset_reader: Arc<R>,
    active_catalog: Arc<ActiveSpatialCatalog>,
}

impl<F, R> SpatialService<F, R>
where
    F: ObjectFetcher + Send + Sync + 'static,
    R: DatasetReader + Send + Sync + 'static,
{
    pub fn new(
        registry: SourceRegistry,
        fetcher: F,
        dataset_reader: R,
        active_catalog: Arc<ActiveSpatialCatalog>,
    ) -> Self {
        Self {
            registry,
            fetcher: Arc::new(fetcher),
            dataset_reader: Arc::new(dataset_reader),
            active_catalog,
        }
    }

    pub fn list_sources(&self) -> ListSourcesResponse {
        ListSourcesResponse {
            sources: self
                .registry
                .list()
                .into_iter()
                .map(|(model, layouts)| Source {
                    model: model.to_string(),
                    layouts: layouts
                        .into_iter()
                        .map(|layout| layout.as_str().to_string())
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn get_spatial_meta(
        &self,
        request: GetSpatialMetaRequest,
    ) -> Result<GetSpatialMetaResponse, SpatialServiceError> {
        let model = parse_model(&request.model)?;
        let (object_key, local_path) =
            self.resolve_object(model, &request.run_ref, &request.timestamp)?;
        let meta = self
            .dataset_reader
            .read_meta(DatasetLocation::LocalFile(local_path.clone()))?;
        Ok(GetSpatialMetaResponse {
            model: model.to_string(),
            object_key,
            local_path: local_path.display().to_string(),
            variables: meta
                .variables
                .into_iter()
                .map(|variable| ProtoVariableMeta {
                    name: variable.name,
                    data_type: variable.data_type,
                    dimensions: variable.dimensions,
                    chunks: variable.chunks,
                })
                .collect(),
        })
    }

    pub fn get_spatial_point_series(
        &self,
        request: GetSpatialPointSeriesRequest,
    ) -> Result<GetSpatialPointSeriesResponse, SpatialServiceError> {
        let model = parse_model(&request.model)?;
        let source = self.require_source(model, &request.model)?;
        let snapshot = self.require_snapshot(model)?;
        let elements = SpatialPointReader::element_names(source);
        let forecast_days = request.days.clamp(1, 16) as usize;
        let mut samples = Vec::new();
        for (_date, day_objects) in snapshot.grouped_by_date().into_iter().take(forecast_days) {
            for object in day_objects {
                let values = SpatialPointReader::read_at(
                    source,
                    &object.local_path,
                    request.latitude,
                    request.longitude,
                )?;
                samples.push(ProtoSpatialPointSample {
                    timestamp: object.timestamp.clone(),
                    valid_date: object.valid_date.clone(),
                    values,
                });
            }
        }
        if samples.is_empty() {
            return Err(SpatialServiceError::EmptyPointSeries);
        }
        Ok(GetSpatialPointSeriesResponse {
            model: model.to_string(),
            run_ref: snapshot.run_ref.clone(),
            elements,
            samples,
        })
    }

    pub fn fetcher(&self) -> Arc<F> {
        self.fetcher.clone()
    }

    pub fn synced_path_for(&self, object_key: &ObjectKey) -> PathBuf {
        self.fetcher.synced_path(object_key)
    }

    pub fn active_catalog(&self) -> Arc<ActiveSpatialCatalog> {
        self.active_catalog.clone()
    }

    fn require_source(
        &self,
        model: WeatherModelId,
        model_name: &str,
    ) -> Result<&dyn WeatherDataSource, SpatialServiceError> {
        self.registry
            .get(model)
            .ok_or_else(|| SpatialServiceError::UnknownModel {
                model: model_name.to_string(),
            })
    }

    fn require_snapshot(
        &self,
        model: WeatherModelId,
    ) -> Result<Arc<crate::domain::SpatialRunSnapshot>, SpatialServiceError> {
        self.active_catalog
            .get(model)
            .ok_or_else(|| SpatialServiceError::NotReady {
                model: model.to_string(),
            })
    }

    fn resolve_object(
        &self,
        model: WeatherModelId,
        run_ref: &str,
        timestamp: &str,
    ) -> Result<(String, PathBuf), SpatialServiceError> {
        let snapshot = self.require_snapshot(model)?;
        if snapshot.run_ref != run_ref {
            return Err(SpatialServiceError::NotReady {
                model: model.to_string(),
            });
        }
        let object = snapshot
            .objects
            .iter()
            .find(|object| object.timestamp == timestamp)
            .ok_or_else(|| SpatialServiceError::NotReady {
                model: model.to_string(),
            })?;
        Ok((object.object_key.clone(), object.local_path.clone()))
    }
}

fn parse_model(model: &str) -> Result<WeatherModelId, SpatialServiceError> {
    WeatherModelId::from_str(model).map_err(|source| SpatialServiceError::UnsupportedModel {
        model: model.to_string(),
        source,
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SpatialPointReader;

impl SpatialPointReader {
    pub fn element_names(source: &dyn WeatherDataSource) -> Vec<String> {
        source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect()
    }

    pub fn read_at(
        source: &dyn WeatherDataSource,
        local_path: &Path,
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<ProtoSpatialElementValue>, DatasetError> {
        let mut elements = Vec::new();
        let mut variable_names = Vec::new();
        for &element in source.supported_elements(DataLayout::Spatial) {
            let Some(variable) = source.variable_name(DataLayout::Spatial, element) else {
                continue;
            };
            elements.push(element);
            variable_names.push(variable);
        }
        let raw_values = OmfilesDatasetReader::read_spatial_points_from_local(
            local_path,
            &variable_names,
            latitude,
            longitude,
        )?;
        let mut values = Vec::new();
        for (element, value) in elements.into_iter().zip(raw_values) {
            let Some(value) = value else {
                continue;
            };
            values.push(ProtoSpatialElementValue {
                element: element.as_str().to_string(),
                // Snowfall: mm water equivalent on S3 → cm via WeatherElement::SNOWFALL_CM_PER_WATER_EQUIVALENT_MM (7/10).
                value: element.normalize_spatial_value(value),
            });
        }
        append_derived_wind_speed(&mut values);
        Ok(values)
    }
}

fn append_derived_wind_speed(values: &mut Vec<ProtoSpatialElementValue>) {
    if values
        .iter()
        .any(|value| value.element == WeatherElement::WindSpeed10m.as_str())
    {
        return;
    }
    let u = values
        .iter()
        .find(|value| value.element == WeatherElement::WindUComponent10m.as_str())
        .map(|value| value.value);
    let v = values
        .iter()
        .find(|value| value.element == WeatherElement::WindVComponent10m.as_str())
        .map(|value| value.value);
    let Some((u, v)) = u.zip(v) else {
        return;
    };
    values.push(ProtoSpatialElementValue {
        element: WeatherElement::WindSpeed10m.as_str().to_string(),
        value: wind_speed_kmh_from_components(u, v),
    });
}

fn wind_speed_kmh_from_components(u: f64, v: f64) -> f64 {
    (u.mul_add(u, v * v)).sqrt() * 3.6
}
