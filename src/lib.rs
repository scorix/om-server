pub mod application;
pub mod domain;
pub mod error;
pub mod r#gen;
pub mod infrastructure;
pub mod interfaces;

pub use application::spatial::SpatialService;
pub use domain::model::WeatherModelId;
pub use error::{
    DataSourceError, DatasetError, GridError, HttpError, MainError, ModelParseError,
    OpenMeteoError, SpatialServiceError, SyncError, TileRenderError, TimestampParseError,
};
pub use infrastructure::config::ServerConfig;
