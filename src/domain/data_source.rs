use std::path::PathBuf;

use anyhow::Result;

use super::model::WeatherModelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataLayout {
    Spatial,
    Timeseries,
}

impl DataLayout {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spatial => "spatial",
            Self::Timeseries => "timeseries",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectKey(pub String);

pub trait WeatherDataSource: Send + Sync {
    fn model_id(&self) -> WeatherModelId;

    fn supported_layouts(&self) -> &'static [DataLayout];

    fn spatial_object_key(&self, run_ref: &str, timestamp: &str) -> Result<ObjectKey>;

    fn timeseries_object_key(&self, variable: &str, chunk: &str) -> Result<ObjectKey>;
}

pub struct SourceRegistry {
    sources: Vec<Box<dyn WeatherDataSource>>,
}

impl SourceRegistry {
    pub fn with_defaults() -> Self {
        Self {
            sources: vec![Box::new(
                crate::infrastructure::ecmwf::EcmwfIfs025SpatialSource,
            )],
        }
    }

    pub fn list(&self) -> Vec<(WeatherModelId, Vec<DataLayout>)> {
        self.sources
            .iter()
            .map(|source| {
                (
                    source.model_id(),
                    source.supported_layouts().to_vec(),
                )
            })
            .collect()
    }

    pub fn get(&self, model: WeatherModelId) -> Option<&dyn WeatherDataSource> {
        self.sources
            .iter()
            .find(|source| source.model_id() == model)
            .map(|source| source.as_ref())
    }
}

pub trait OmFetcher: Send + Sync {
    fn sync_object(&self, object_key: &ObjectKey) -> Result<()>;

    fn synced_path(&self, object_key: &ObjectKey) -> PathBuf;
}
