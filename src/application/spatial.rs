use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::domain::{ObjectKey, OmFetcher, OmReaderBackend, SourceRegistry, WeatherModelId};
use crate::r#gen::{
    GetSpatialMetaRequest, GetSpatialMetaResponse, HealthResponse, ListSourcesResponse, Source,
    VariableMeta as ProtoVariableMeta,
};
use crate::infrastructure::{OmDatasetReader, S3OmFetcher};

pub struct SpatialService<F = S3OmFetcher> {
    registry: SourceRegistry,
    fetcher: Arc<F>,
    sync_on_request: bool,
}

impl<F> SpatialService<F>
where
    F: OmFetcher + Send + Sync + 'static,
{
    pub fn new(
        registry: SourceRegistry,
        fetcher: F,
        sync_on_request: bool,
    ) -> Self {
        Self {
            registry,
            fetcher: Arc::new(fetcher),
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
    ) -> Result<GetSpatialMetaResponse> {
        let model = WeatherModelId::from_str(&request.model)
            .with_context(|| format!("unsupported model {}", request.model))?;
        let source = self
            .registry
            .get(model)
            .with_context(|| format!("unknown model {}", request.model))?;
        let object_key = source.spatial_object_key(&request.run_ref, &request.timestamp)?;
        if self.sync_on_request {
            self.fetcher.sync_object(&object_key)?;
        }
        let local_path = self.fetcher.synced_path(&object_key);
        if !local_path.exists() {
            bail!(
                "object {} is not synced at {}",
                object_key.0,
                local_path.display()
            );
        }
        let meta = OmDatasetReader::read_meta(OmReaderBackend::LocalMmap(local_path.clone()))?;
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
