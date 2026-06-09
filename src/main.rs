use std::path::PathBuf;

use clap::Parser;

use om_server::application::sync_worker::{SpatialSyncWorker, SpatialSyncWorkerConfig};
use om_server::application::weather_bake::{WeatherBakeConfig, build_bake_plans};
use om_server::application::weather_bake_wake::WeatherBakeWake;
use om_server::application::weather_bake_worker::{WeatherBakeWorker, WeatherBakeWorkerConfig};
use om_server::application::{ActiveSpatialCatalog, SpatialService};
use om_server::domain::WeatherBakeLayer;
use om_server::error::MainError;
use om_server::r#gen::FILE_DESCRIPTOR_SET;
use om_server::r#gen::om_spatial_service_server::OmSpatialServiceServer;
use om_server::infrastructure::config::ServerConfig;
use om_server::infrastructure::{
    OmfilesDatasetReader, S3ObjectFetcher, open_meteo::OpenMeteoSources,
};
use om_server::interfaces::grpc::spatial_service::GrpcSpatialService;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "om-server")]
struct Cli {
    #[command(flatten)]
    serve: ServerConfig,
}

#[tokio::main]
async fn main() -> Result<(), MainError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let cli = Cli::parse();
    run_serve(cli.serve).await
}

async fn run_serve(config: ServerConfig) -> Result<(), MainError> {
    let sync_models = config.parsed_sync_models()?;
    let fetcher = S3ObjectFetcher::new(config.s3_base_url.clone(), config.om_sync_dir.clone());
    let catalog = std::sync::Arc::new(ActiveSpatialCatalog::load_persisted(
        &config.om_sync_dir,
        &fetcher,
        &sync_models,
    )?);
    let s3_catalog =
        om_server::infrastructure::open_meteo::OpenMeteoS3Catalog::new(config.s3_base_url.clone());
    let run_catalog = std::sync::Arc::new(
        om_server::infrastructure::open_meteo::OpenMeteoSpatialRunCatalog::new(s3_catalog),
    );
    let bake_config = weather_bake_config_from_env(&config);
    let bake_wake = WeatherBakeWake::new();
    let worker = SpatialSyncWorker::new(
        OpenMeteoSources.registry(),
        S3ObjectFetcher::new(config.s3_base_url.clone(), config.om_sync_dir.clone()),
        catalog.clone(),
        SpatialSyncWorkerConfig {
            run_catalog,
            sync_dir: config.om_sync_dir.clone(),
            models: sync_models.clone(),
            forecast_days: config.sync_forecast_days as usize,
            interval: config.sync_interval(),
            parallelism: config.sync_parallelism,
            bake_wake: Some(bake_wake.subscribe()),
        },
    );
    tokio::spawn(worker.run_forever());

    if let Some(bake) = bake_config.clone() {
        let bake_worker = WeatherBakeWorker::new(WeatherBakeWorkerConfig {
            bake,
            catalog: catalog.clone(),
            interval: weather_bake_interval_from_env(),
            wake: Some(bake_wake.subscribe()),
        });
        tokio::spawn(bake_worker.run_forever());
    }

    let service = std::sync::Arc::new(SpatialService::new(
        OpenMeteoSources.registry(),
        fetcher,
        OmfilesDatasetReader,
        catalog,
    ));
    let grpc = GrpcSpatialService::new(service);
    let addr = config
        .grpc_bind
        .parse()
        .map_err(|source| MainError::ParseAddress {
            address: config.grpc_bind.clone(),
            source,
        })?;

    tracing::info!(
        %addr,
        sync_dir = %config.om_sync_dir.display(),
        sync_models = ?config.sync_models,
        weather_bake_enabled = bake_config.is_some(),
        "om-server listening"
    );
    let (_health_reporter, health_service) = tonic_health::server::health_reporter();
    let reflection = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()
        .map_err(|source| MainError::Reflection { source })?;
    Server::builder()
        .add_service(health_service)
        .add_service(reflection)
        .add_service(OmSpatialServiceServer::new(grpc))
        .serve(addr)
        .await?;
    Ok(())
}

fn weather_bake_config_from_env(config: &ServerConfig) -> Option<WeatherBakeConfig> {
    let output_dir = std::env::var("OM_SERVER_WEATHER_BAKE_OUTPUT_DIR")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)?;
    let sqlite_path = std::env::var("OM_SERVER_WEATHER_BAKE_SQLITE")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/processed/opensnowmap/snowbuddy_osm.sqlite"));
    let manifest_dir = std::env::var("OM_SERVER_WEATHER_BAKE_MANIFEST_DIR")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/manifests"));
    let cache_dir = std::env::var("OM_SERVER_WEATHER_BAKE_CACHE_DIR")
        .ok()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from("data/cache/weather-pmtiles")));

    let model = config
        .parsed_sync_models()
        .ok()
        .and_then(|models| models.first().copied())
        .unwrap_or(om_server::domain::WeatherModelId::EcmwfIfs);

    let variables = std::env::var("OM_SERVER_WEATHER_BAKE_VARIABLES")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| WeatherBakeLayer::from_id(part.trim()))
                .collect::<Vec<_>>()
        })
        .filter(|layers| !layers.is_empty());

    Some(WeatherBakeConfig {
        sqlite_path,
        cache_dir,
        model,
        complete_only: true,
        plans: build_bake_plans(output_dir, manifest_dir, model, variables),
    })
}

fn weather_bake_interval_from_env() -> std::time::Duration {
    std::env::var("OM_SERVER_WEATHER_BAKE_INTERVAL_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|&secs| secs > 0)
        .map(std::time::Duration::from_secs)
        .unwrap_or_else(|| std::time::Duration::from_secs(60))
}
