use crate::domain::DatasetMeta;
use crate::error::TileRenderError;

pub struct TileRequest {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub variable: String,
}

pub trait WeatherTileRenderer: Send + Sync {
    fn render(
        &self,
        dataset: &DatasetMeta,
        request: &TileRequest,
    ) -> Result<Vec<u8>, TileRenderError>;
}

#[derive(Debug, Default)]
pub struct NoopWeatherTileRenderer;

impl WeatherTileRenderer for NoopWeatherTileRenderer {
    fn render(
        &self,
        _dataset: &DatasetMeta,
        _request: &TileRequest,
    ) -> Result<Vec<u8>, TileRenderError> {
        Err(TileRenderError::NotImplemented)
    }
}
