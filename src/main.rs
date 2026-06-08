use std::sync::Arc;

use clap::Parser;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use om_server::application::active_catalog::ActiveSpatialCatalog;
use om_server::application::spatial::SpatialService;
use om_server::application::sync_worker::{SpatialSyncWorker, SpatialSyncWorkerConfig};
use om_server::error::MainError;
use om_server::r#gen::FILE_DESCRIPTOR_SET;
use om_server::r#gen::om_spatial_service_server::OmSpatialServiceServer;
use om_server::infrastructure::config::ServerConfig;
use om_server::infrastructure::{
    OmfilesDatasetReader, S3ObjectFetcher, open_meteo::OpenMeteoSources,
};
use om_server::interfaces::grpc::spatial_service::GrpcSpatialService;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = ServerConfig::parse();
    let sync_models = config.parsed_sync_models()?;
    let fetcher = S3ObjectFetcher::new(config.s3_base_url.clone(), config.om_sync_dir.clone());
    let catalog = Arc::new(ActiveSpatialCatalog::load_persisted(
        &config.om_sync_dir,
        &fetcher,
        &sync_models,
    )?);
    let s3_catalog =
        om_server::infrastructure::open_meteo::OpenMeteoS3Catalog::new(config.s3_base_url.clone());
    let run_catalog = Arc::new(
        om_server::infrastructure::open_meteo::OpenMeteoSpatialRunCatalog::new(s3_catalog),
    );
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
        },
    );
    tokio::spawn(worker.run_forever());

    let service = Arc::new(SpatialService::new(
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
