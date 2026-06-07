use crate::error::DataSourceError;

use super::model::{WeatherElement, WeatherModelId};

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

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement];

    fn variable_name(&self, layout: DataLayout, element: WeatherElement) -> Option<&'static str>;

    fn spatial_object_key(
        &self,
        run_ref: &str,
        timestamp: &str,
    ) -> Result<ObjectKey, DataSourceError>;

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError>;
}

pub struct SourceRegistry {
    sources: Vec<Box<dyn WeatherDataSource>>,
}

impl SourceRegistry {
    pub fn new(sources: Vec<Box<dyn WeatherDataSource>>) -> Self {
        Self { sources }
    }

    pub fn list(&self) -> Vec<(WeatherModelId, Vec<DataLayout>)> {
        self.sources
            .iter()
            .map(|source| (source.model_id(), source.supported_layouts().to_vec()))
            .collect()
    }

    pub fn get(&self, model: WeatherModelId) -> Option<&dyn WeatherDataSource> {
        self.sources
            .iter()
            .find(|source| source.model_id() == model)
            .map(|source| source.as_ref())
    }
}
