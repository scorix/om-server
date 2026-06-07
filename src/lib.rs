pub mod application;
pub mod domain;
pub mod r#gen;
pub mod infrastructure;
pub mod interfaces;

pub use application::spatial::SpatialService;
pub use domain::model::WeatherModelId;
pub use infrastructure::config::ServerConfig;
