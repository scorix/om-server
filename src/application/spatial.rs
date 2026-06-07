use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use crate::domain::{
    DataLayout, DatasetLocation, DatasetReader, ObjectFetcher, ObjectKey, SourceRegistry,
    WeatherDataSource, WeatherModelId,
};
use crate::error::{DatasetError, SpatialServiceError};
use crate::r#gen::{
    GetSpatialMetaRequest, GetSpatialMetaResponse, GetSpatialPointSeriesRequest,
    GetSpatialPointSeriesResponse, HealthResponse, ListSourcesResponse, Source,
    SpatialElementValue as ProtoSpatialElementValue, SpatialPointSample as ProtoSpatialPointSample,
    VariableMeta as ProtoVariableMeta,
};
use crate::infrastructure::open_meteo::OpenMeteoS3Catalog;
use crate::infrastructure::{OmfilesDatasetReader, S3ObjectFetcher};

pub struct SpatialService<F = S3ObjectFetcher, R = OmfilesDatasetReader> {
    registry: SourceRegistry,
    fetcher: Arc<F>,
    dataset_reader: Arc<R>,
    sync_on_request: bool,
    s3_catalog: OpenMeteoS3Catalog,
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
        sync_on_request: bool,
    ) -> Self {
        Self {
            registry,
            fetcher: Arc::new(fetcher),
            dataset_reader: Arc::new(dataset_reader),
            sync_on_request,
            s3_catalog: OpenMeteoS3Catalog::default(),
        }
    }

    pub fn with_s3_catalog(
        registry: SourceRegistry,
        fetcher: F,
        dataset_reader: R,
        sync_on_request: bool,
        s3_catalog: OpenMeteoS3Catalog,
    ) -> Self {
        Self {
            registry,
            fetcher: Arc::new(fetcher),
            dataset_reader: Arc::new(dataset_reader),
            sync_on_request,
            s3_catalog,
        }
    }

    pub fn health(&self) -> HealthResponse {
        HealthResponse {
            ok: true,
            service: "om-server".to_string(),
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
        let model = WeatherModelId::from_str(&request.model).map_err(|source| {
            SpatialServiceError::UnsupportedModel {
                model: request.model.clone(),
                source,
            }
        })?;
        let source = self
            .registry
            .get(model)
            .ok_or_else(|| SpatialServiceError::UnknownModel {
                model: request.model.clone(),
            })?;
        let object_key = source.spatial_object_key(&request.run_ref, &request.timestamp)?;
        if self.sync_on_request {
            self.fetcher.sync_object(&object_key)?;
        }
        let local_path = self.fetcher.synced_path(&object_key);
        if !local_path.exists() {
            return Err(SpatialServiceError::NotSynced {
                object_key: object_key.0,
                path: local_path,
            });
        }
        let meta = self
            .dataset_reader
            .read_meta(DatasetLocation::LocalFile(local_path.clone()))?;
        Ok(GetSpatialMetaResponse {
            model: model.to_string(),
            object_key: object_key.0,
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
        let model = WeatherModelId::from_str(&request.model).map_err(|source| {
            SpatialServiceError::UnsupportedModel {
                model: request.model.clone(),
                source,
            }
        })?;
        let source = self
            .registry
            .get(model)
            .ok_or_else(|| SpatialServiceError::UnknownModel {
                model: request.model.clone(),
            })?;
        let elements = SpatialPointReader::element_names(source);
        let forecast_days = request.days.clamp(1, 16) as usize;
        let run = self
            .s3_catalog
            .load_latest_spatial_run(model)
            .map_err(SpatialServiceError::OpenMeteo)?;
        let run_ref = run.run_ref.clone();
        let grouped = run.grouped_by_date();
        let mut samples = Vec::new();
        for (_date, day_objects) in grouped.into_iter().take(forecast_days) {
            for object in day_objects {
                let object_key = source.spatial_object_key(&run_ref, &object.timestamp)?;
                if self.sync_on_request {
                    self.fetcher.sync_object(&object_key)?;
                }
                let local_path = self.fetcher.synced_path(&object_key);
                if !local_path.exists() {
                    return Err(SpatialServiceError::NotSynced {
                        object_key: object_key.0,
                        path: local_path,
                    });
                }
                let values = SpatialPointReader::read_at(
                    source,
                    &local_path,
                    request.latitude,
                    request.longitude,
                )?;
                samples.push(ProtoSpatialPointSample {
                    timestamp: object.timestamp,
                    valid_date: object.valid_date,
                    values,
                });
            }
        }
        if samples.is_empty() {
            return Err(SpatialServiceError::EmptyPointSeries);
        }
        Ok(GetSpatialPointSeriesResponse {
            model: model.to_string(),
            run_ref,
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
        source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|&element| {
                let variable = source
                    .variable_name(DataLayout::Spatial, element)
                    .ok_or_else(|| DatasetError::VariableNotFound {
                        variable: element.as_str().to_string(),
                    })?;
                let value = OmfilesDatasetReader::read_spatial_point_from_local(
                    local_path,
                    variable,
                    latitude,
                    longitude,
                )?;
                Ok(ProtoSpatialElementValue {
                    element: element.as_str().to_string(),
                    value,
                })
            })
            .collect()
    }
}
