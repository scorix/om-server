use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;

use crate::domain::WeatherModelId;
use crate::error::MainError;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    #[arg(long, default_value = "127.0.0.1:50051", env = "OM_SERVER_GRPC_BIND")]
    pub grpc_bind: String,
    #[arg(long, default_value = "data/manifests", env = "OM_SERVER_WEATHER_BAKE_MANIFEST_DIR")]
    pub weather_manifest_dir: PathBuf,
    #[arg(long, default_value = "data/sync/om", env = "OM_SERVER_OM_SYNC_DIR")]
    pub om_sync_dir: PathBuf,
    #[arg(
        long,
        default_value = "https://openmeteo.s3.amazonaws.com",
        env = "OM_SERVER_S3_BASE_URL"
    )]
    pub s3_base_url: String,
    #[arg(long, default_value = "3600", env = "OM_SERVER_SYNC_INTERVAL_SECS")]
    pub sync_interval_secs: u64,
    #[arg(long, default_value_t = 16, env = "OM_SERVER_SYNC_FORECAST_DAYS")]
    pub sync_forecast_days: u32,
    #[arg(
        long,
        value_delimiter = ',',
        default_value = "ecmwf_ifs",
        env = "OM_SERVER_SYNC_MODELS"
    )]
    pub sync_models: Vec<String>,
    #[arg(long, default_value_t = 4, env = "OM_SERVER_SYNC_PARALLELISM")]
    pub sync_parallelism: usize,
    /// Total download attempts per spatial object (initial try plus retries).
    #[arg(long, default_value_t = 4, env = "OM_SERVER_SYNC_RETRY_ATTEMPTS")]
    pub sync_retry_attempts: usize,
    /// Base delay between object download retries; doubled after each failure.
    #[arg(long, default_value_t = 2, env = "OM_SERVER_SYNC_RETRY_DELAY_SECS")]
    pub sync_retry_delay_secs: u64,
    /// When set, enables the weather PMTiles bake worker.
    #[arg(long, env = "OM_SERVER_WEATHER_BAKE_OUTPUT_DIR")]
    pub weather_bake_output_dir: Option<PathBuf>,
    #[arg(
        long,
        default_value = "data/cache/weather-pmtiles",
        env = "OM_SERVER_WEATHER_BAKE_CACHE_DIR"
    )]
    pub weather_bake_cache_dir: PathBuf,
    #[arg(
        long,
        default_value = "config/weather_bake.toml",
        env = "OM_SERVER_WEATHER_BAKE_CONFIG"
    )]
    pub weather_bake_config: PathBuf,
    #[arg(long, default_value_t = 60, env = "OM_SERVER_WEATHER_BAKE_INTERVAL_SECS")]
    pub weather_bake_interval_secs: u64,
}

impl ServerConfig {
    pub fn sync_interval(&self) -> Duration {
        Duration::from_secs(self.sync_interval_secs)
    }

    pub fn sync_retry_delay(&self) -> Duration {
        Duration::from_secs(self.sync_retry_delay_secs)
    }

    pub fn weather_bake_interval(&self) -> Duration {
        if self.weather_bake_interval_secs > 0 {
            Duration::from_secs(self.weather_bake_interval_secs)
        } else {
            Duration::from_secs(60)
        }
    }

    pub fn parsed_sync_models(&self) -> Result<Vec<WeatherModelId>, MainError> {
        self.sync_models
            .iter()
            .map(|model| {
                WeatherModelId::from_str(model).map_err(|source| MainError::InvalidSyncModel {
                    model: model.clone(),
                    source,
                })
            })
            .collect()
    }
}
