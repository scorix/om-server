use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::application::spatial::SpatialService;
use crate::r#gen::om_spatial_service_server::OmSpatialService as OmSpatialServiceRpc;

pub struct GrpcSpatialService {
    inner: Arc<SpatialService>,
}

impl GrpcSpatialService {
    pub fn new(inner: Arc<SpatialService>) -> Self {
        Self { inner }
    }
}

#[tonic::async_trait]
impl OmSpatialServiceRpc for GrpcSpatialService {
    async fn health(
        &self,
        _request: Request<crate::r#gen::HealthRequest>,
    ) -> Result<Response<crate::r#gen::HealthResponse>, Status> {
        Ok(Response::new(self.inner.health()))
    }

    async fn list_sources(
        &self,
        _request: Request<crate::r#gen::ListSourcesRequest>,
    ) -> Result<Response<crate::r#gen::ListSourcesResponse>, Status> {
        Ok(Response::new(self.inner.list_sources()))
    }

    async fn get_spatial_meta(
        &self,
        request: Request<crate::r#gen::GetSpatialMetaRequest>,
    ) -> Result<Response<crate::r#gen::GetSpatialMetaResponse>, Status> {
        self.inner
            .get_spatial_meta(request.into_inner())
            .map(Response::new)
            .map_err(|error| Status::invalid_argument(format!("{error:#}")))
    }

    async fn get_spatial_point_series(
        &self,
        request: Request<crate::r#gen::GetSpatialPointSeriesRequest>,
    ) -> Result<Response<crate::r#gen::GetSpatialPointSeriesResponse>, Status> {
        self.inner
            .get_spatial_point_series(request.into_inner())
            .map(Response::new)
            .map_err(|error| Status::invalid_argument(format!("{error:#}")))
    }
}
