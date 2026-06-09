pub mod application;
pub mod domain;
pub mod error;
pub mod r#gen;
pub mod infrastructure;
pub mod interfaces;

pub use application::active_catalog::ActiveSpatialCatalog;
pub use application::spatial::SpatialService;
pub use application::sync_worker::SpatialSyncWorker;
pub use domain::model::WeatherModelId;
pub use error::{
    ActiveCatalogError, DataSourceError, DatasetError, GridError, HttpError, MainError,
    ModelParseError, OpenMeteoError, SpatialServiceError, SyncError, SyncWorkerError,
    TileRenderError, TimestampParseError, WeatherBakeError,
};
pub use infrastructure::config::ServerConfig;
