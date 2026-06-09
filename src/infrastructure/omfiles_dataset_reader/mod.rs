mod spatial_file;
mod spatial_sampler;
mod variable_index;

pub use spatial_file::open_local;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use omfiles::reader::OmFileReader;
use omfiles::traits::OmFileReaderBackend;

use crate::domain::{DatasetLocation, DatasetMeta, DatasetReader, SpatialGrid};
use crate::error::DatasetError;

use spatial_file::{OmSpatialFile, OmSpatialFileSessionWithIndex, spatial_grid};

#[derive(Debug, Default, Clone, Copy)]
pub struct OmfilesDatasetReader;

impl DatasetReader for OmfilesDatasetReader {
    fn read_meta(&self, location: DatasetLocation) -> Result<DatasetMeta, DatasetError> {
        match location {
            DatasetLocation::LocalFile(path) => Ok(open_local(&path)?.meta()),
            DatasetLocation::HttpRange { url } => {
                let backend = super::HttpRangeReader::new(url.clone())?;
                let reader = OmFileReader::new(std::sync::Arc::new(backend)).map_err(|source| {
                    DatasetError::OpenFile {
                        path: PathBuf::from(url),
                        source,
                    }
                })?;
                Ok(OmSpatialFile::new(reader, PathBuf::new()).meta())
            }
        }
    }
}

impl OmfilesDatasetReader {
    pub fn read_meta_from_reader<B>(
        reader: OmFileReader<B>,
        local_path: PathBuf,
    ) -> Result<DatasetMeta, DatasetError>
    where
        B: OmFileReaderBackend,
    {
        Ok(OmSpatialFile::new(reader, local_path).meta())
    }

    pub fn read_spatial_grid_from_reader<B>(
        reader: &OmFileReader<B>,
        dimensions: &[u64],
    ) -> Result<Arc<SpatialGrid>, DatasetError>
    where
        B: OmFileReaderBackend,
    {
        spatial_grid(reader, dimensions)
    }

    pub fn read_spatial_point_from_local(
        path: &Path,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError> {
        open_local(path)?.sample_point(variable_name, latitude, longitude)
    }

    pub fn read_spatial_points_from_local(
        path: &Path,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<Option<f64>>, DatasetError> {
        open_local(path)?.sample_points(variable_names, latitude, longitude)
    }

    pub fn read_spatial_point_from_reader<B>(
        reader: &OmFileReader<B>,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError>
    where
        B: OmFileReaderBackend,
    {
        OmSpatialFileSessionWithIndex::from_reader(reader).sample_point(
            variable_name,
            latitude,
            longitude,
        )
    }

    pub fn read_spatial_points_from_reader<B>(
        reader: &OmFileReader<B>,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<Option<f64>>, DatasetError>
    where
        B: OmFileReaderBackend,
    {
        OmSpatialFileSessionWithIndex::from_reader(reader).sample_points(
            variable_names,
            latitude,
            longitude,
        )
    }
}
