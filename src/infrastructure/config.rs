use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use clap::Parser;

use crate::domain::WeatherModelId;
use crate::error::MainError;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    #[arg(long, default_value = "127.0.0.1:50051")]
    pub grpc_bind: String,
    #[arg(long, default_value = "data/sync/om")]
    pub om_sync_dir: PathBuf,
    #[arg(long, default_value = "https://openmeteo.s3.amazonaws.com")]
    pub s3_base_url: String,
    #[arg(long, default_value = "3600")]
    pub sync_interval_secs: u64,
    #[arg(long, default_value_t = 16)]
    pub sync_forecast_days: u32,
    #[arg(long, value_delimiter = ',', default_value = "ecmwf_ifs")]
    pub sync_models: Vec<String>,
    #[arg(long, default_value_t = 4)]
    pub sync_parallelism: usize,
}

impl ServerConfig {
    pub fn sync_interval(&self) -> Duration {
        Duration::from_secs(self.sync_interval_secs)
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
