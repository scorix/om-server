use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use crate::domain::{
    DatasetLocation, DatasetReader, ObjectFetcher, ObjectKey, SourceRegistry, WeatherModelId,
};
use crate::error::SpatialServiceError;
use crate::r#gen::{
    GetSpatialMetaRequest, GetSpatialMetaResponse, HealthResponse, ListSourcesResponse, Source,
    VariableMeta as ProtoVariableMeta,
};
use crate::infrastructure::{OmfilesDatasetReader, S3ObjectFetcher};

pub struct SpatialService<F = S3ObjectFetcher, R = OmfilesDatasetReader> {
    registry: SourceRegistry,
    fetcher: Arc<F>,
    dataset_reader: Arc<R>,
    sync_on_request: bool,
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

    pub fn fetcher(&self) -> Arc<F> {
        self.fetcher.clone()
    }

    pub fn synced_path_for(&self, object_key: &ObjectKey) -> PathBuf {
        self.fetcher.synced_path(object_key)
    }
}
