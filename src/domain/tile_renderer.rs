use anyhow::{bail, Result};

use super::om_dataset::OmDatasetMeta;

pub struct TileRequest {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub variable: String,
}

pub trait WeatherTileRenderer: Send + Sync {
    fn render(&self, dataset: &OmDatasetMeta, request: &TileRequest) -> Result<Vec<u8>>;
}

#[derive(Debug, Default)]
pub struct NoopWeatherTileRenderer;

impl WeatherTileRenderer for NoopWeatherTileRenderer {
    fn render(&self, _dataset: &OmDatasetMeta, _request: &TileRequest) -> Result<Vec<u8>> {
        bail!("weather tile rendering is not implemented")
    }
}
