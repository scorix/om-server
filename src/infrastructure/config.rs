use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    #[arg(long, default_value = "127.0.0.1:50051")]
    pub grpc_bind: String,
    #[arg(long, default_value = "ecmwf_ifs025")]
    pub source: String,
    #[arg(long, default_value = "data/sync/om")]
    pub om_sync_dir: PathBuf,
    #[arg(long, default_value = "https://openmeteo.s3.amazonaws.com")]
    pub s3_base_url: String,
    #[arg(long, default_value_t = true)]
    pub sync_on_request: bool,
}
