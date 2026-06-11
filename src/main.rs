use clap::Parser;

use om_server::application::sync_worker::{SpatialSyncWorker, SpatialSyncWorkerConfig};
use om_server::application::weather_bake::WeatherBakeConfig;
use om_server::application::weather_bake::build_bake_plans;
use om_server::application::weather_bake_wake::WeatherBakeWake;
use om_server::application::weather_bake_worker::{WeatherBakeWorker, WeatherBakeWorkerConfig};
use om_server::application::{ActiveSpatialCatalog, SpatialService};
use om_server::domain::WeatherModelId;
use om_server::error::MainError;
use om_server::r#gen::FILE_DESCRIPTOR_SET;
use om_server::r#gen::om_spatial_service_server::OmSpatialServiceServer;
use om_server::infrastructure::config::ServerConfig;
use om_server::infrastructure::weather_bake_profile::load_weather_bake_profile;
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
    let blend_profile = load_weather_bake_profile(&config.weather_bake_config)?;
    let sync_models = sync_models_for_server(&config, &blend_profile)?;
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
    let bake_config = weather_bake_config(&config)?;
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
            retry_attempts: config.sync_retry_attempts,
            retry_delay: config.sync_retry_delay(),
            bake_wake: Some(bake_wake.subscribe()),
        },
    );
    tokio::spawn(worker.run_forever());

    if let Some(bake) = bake_config.clone() {
        let bake_worker = WeatherBakeWorker::new(WeatherBakeWorkerConfig {
            bake,
            catalog: catalog.clone(),
            interval: config.weather_bake_interval(),
            wake: Some(bake_wake.subscribe()),
        });
        tokio::spawn(bake_worker.run_forever());
    }

    let service = std::sync::Arc::new(SpatialService::new(
        OpenMeteoSources.registry(),
        fetcher,
        OmfilesDatasetReader,
        catalog,
        blend_profile,
        config.weather_manifest_dir.clone(),
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
        weather_manifest_dir = %config.weather_manifest_dir.display(),
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

fn sync_models_for_server(
    config: &ServerConfig,
    bake_profile: &om_server::infrastructure::weather_bake_profile::WeatherBakeProfile,
) -> Result<Vec<WeatherModelId>, MainError> {
    let mut models = config.parsed_sync_models()?;
    for spec in &bake_profile.layers {
        if !models.contains(&spec.model) {
            models.push(spec.model);
        }
    }
    Ok(models)
}

fn weather_bake_config(config: &ServerConfig) -> Result<Option<WeatherBakeConfig>, MainError> {
    let Some(output_dir) = config
        .weather_bake_output_dir
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
    else {
        return Ok(None);
    };
    let profile = load_weather_bake_profile(&config.weather_bake_config)?;
    Ok(Some(WeatherBakeConfig {
        cache_dir: Some(config.weather_bake_cache_dir.clone()),
        timeline_model: profile.timeline_model,
        plans: build_bake_plans(
            output_dir.clone(),
            config.weather_manifest_dir.clone(),
            &profile,
        ),
    }))
}
