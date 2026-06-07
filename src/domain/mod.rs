pub mod catalog;
pub mod data_source;
pub mod dataset;
pub mod grid;
pub mod model;
pub mod ports;
pub mod spatial_run;
pub mod spatial_snapshot;
pub mod tile_renderer;

pub use catalog::{
    ModelPriorityPolicy, ModelPriorityRule, PolicyContext, ResolvedElement,
    RuleBasedModelPriorityPolicy, WeatherElementCatalog,
};
pub use data_source::{DataLayout, ObjectKey, SourceRegistry, WeatherDataSource};
pub use dataset::{DatasetMeta, VariableMeta};
pub use grid::{InterpolationWindow, PointWindow, SpatialGrid, SpatialGridMetadata};
pub use model::{WeatherElement, WeatherModelId};
pub use ports::{DatasetLocation, DatasetReader, ObjectFetcher};
pub use spatial_run::{SpatialObjectRef, SpatialRun, SpatialRunCatalog};
pub use spatial_snapshot::{SpatialObjectLocal, SpatialRunSnapshot};
pub use tile_renderer::{NoopWeatherTileRenderer, TileRequest, WeatherTileRenderer};
