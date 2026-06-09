use std::fs::File;
use std::path::Path;

use pmtiles::{Compression, PmTilesStreamWriter, PmTilesWriter, TileCoord, TileType};

use crate::error::WeatherBakeError;

#[derive(Debug, Clone)]
pub struct PmtilesMetadata {
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub bounds: Option<(f64, f64, f64, f64)>,
    pub json: String,
}

#[derive(Debug, Clone)]
pub struct PmtilesTile {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub data: Vec<u8>,
}

pub struct PngPmtilesWriter {
    writer: PmTilesStreamWriter<File>,
}

impl PngPmtilesWriter {
    pub fn create(path: &Path, metadata: &PmtilesMetadata) -> Result<Self, WeatherBakeError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| WeatherBakeError::WriteFile {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let file = File::create(path).map_err(|source| WeatherBakeError::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
        let mut builder = PmTilesWriter::new(TileType::Png)
            .internal_compression(Compression::None)
            .tile_compression(Compression::None)
            .min_zoom(metadata.min_zoom)
            .max_zoom(metadata.max_zoom)
            .metadata(&metadata.json);
        if let Some((west, south, east, north)) = metadata.bounds {
            builder = builder.bounds(west, south, east, north);
        }
        let writer = builder
            .create(file)
            .map_err(|source| WeatherBakeError::PmtilesWrite {
                path: path.to_path_buf(),
                message: source.to_string(),
            })?;
        Ok(Self { writer })
    }

    pub fn add_tile(&mut self, tile: &PmtilesTile) -> Result<(), WeatherBakeError> {
        let coord = TileCoord::new(tile.z, tile.x, tile.y).map_err(|source| {
            WeatherBakeError::PmtilesWrite {
                path: Path::new("").to_path_buf(),
                message: source.to_string(),
            }
        })?;
        self.writer
            .add_tile(coord, &tile.data)
            .map_err(|source| WeatherBakeError::PmtilesWrite {
                path: Path::new("").to_path_buf(),
                message: source.to_string(),
            })
    }

    pub fn finalize(self) -> Result<(), WeatherBakeError> {
        self.writer
            .finalize()
            .map_err(|source| WeatherBakeError::PmtilesWrite {
                path: Path::new("").to_path_buf(),
                message: source.to_string(),
            })
    }
}

pub fn write_png_pmtiles(
    path: &Path,
    metadata: &PmtilesMetadata,
    tiles: &[PmtilesTile],
) -> Result<(), WeatherBakeError> {
    let mut writer = PngPmtilesWriter::create(path, metadata)?;
    for tile in tiles {
        writer.add_tile(tile)?;
    }
    writer.finalize()
}

pub fn sha256_file(path: &Path) -> Result<String, WeatherBakeError> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|source| WeatherBakeError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
