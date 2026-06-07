use std::sync::Arc;

use anyhow::Context;
use clap::Parser;
use tonic::transport::Server;
use tracing_subscriber::EnvFilter;

use om_server::application::spatial::SpatialService;
use om_server::domain::SourceRegistry;
use om_server::r#gen::om_spatial_service_server::OmSpatialServiceServer;
use om_server::infrastructure::S3OmFetcher;
use om_server::ServerConfig;
use om_server::interfaces::grpc::spatial_service::GrpcSpatialService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    let config = ServerConfig::parse();
    let registry = SourceRegistry::with_defaults();
    let fetcher = S3OmFetcher::new(config.s3_base_url.clone(), config.om_sync_dir.clone());
    let service = Arc::new(SpatialService::new(
        registry,
        fetcher,
        config.sync_on_request,
    ));
    let grpc = GrpcSpatialService::new(service);
    let addr = config
        .grpc_bind
        .parse()
        .with_context(|| format!("parse grpc bind address {}", config.grpc_bind))?;

    tracing::info!(%addr, sync_dir = %config.om_sync_dir.display(), "om-server listening");
    Server::builder()
        .add_service(OmSpatialServiceServer::new(grpc))
        .serve(addr)
        .await?;
    Ok(())
}
