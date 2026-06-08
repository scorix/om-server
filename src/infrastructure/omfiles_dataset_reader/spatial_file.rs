use std::path::{Path, PathBuf};
use std::sync::Arc;

use omfiles::reader::OmFileReader;
use omfiles::traits::{OmFileReadable, OmFileReaderBackend, OmScalarVariable};

use crate::domain::{DatasetMeta, SpatialGrid, SpatialGridMetadata};
use crate::error::DatasetError;
use crate::infrastructure::spatial_grid_cache;

use super::spatial_sampler::SpatialSampler;
use super::variable_index::VariableIndex;

pub struct OmSpatialFile<B> {
    reader: OmFileReader<B>,
    variables: VariableIndex,
    local_path: PathBuf,
}

pub struct OmSpatialFileSession<'a, B> {
    reader: &'a OmFileReader<B>,
    variables: &'a VariableIndex,
}

pub fn open_local(path: &Path) -> Result<OmSpatialFile<impl OmFileReaderBackend>, DatasetError> {
    let path_str = path.to_str().ok_or_else(|| DatasetError::NonUtf8Path {
        path: path.to_path_buf(),
    })?;
    let reader = OmFileReader::from_file(path_str).map_err(|source| DatasetError::OpenFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(OmSpatialFile::new(reader, path.to_path_buf()))
}

impl<B: OmFileReaderBackend> OmSpatialFile<B> {
    pub fn new(reader: OmFileReader<B>, local_path: PathBuf) -> Self {
        let variables = VariableIndex::build(&reader);
        Self {
            reader,
            variables,
            local_path,
        }
    }

    pub fn meta(&self) -> DatasetMeta {
        DatasetMeta {
            local_path: self.local_path.clone(),
            variables: self.variables.metadata(),
        }
    }

    pub fn session(&self) -> OmSpatialFileSession<'_, B> {
        OmSpatialFileSession {
            reader: &self.reader,
            variables: &self.variables,
        }
    }

    pub fn sample_point(
        &self,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError> {
        self.session()
            .sample_point(variable_name, latitude, longitude)
    }

    pub fn sample_points(
        &self,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<Option<f64>>, DatasetError> {
        self.session()
            .sample_points(variable_names, latitude, longitude)
    }
}

impl<'a, B: OmFileReaderBackend> OmSpatialFileSession<'a, B> {
    pub fn variables(&self) -> &VariableIndex {
        self.variables
    }

    pub fn with_variable<T>(
        &self,
        name: &str,
        read: impl FnOnce(&OmFileReader<B>) -> Result<T, DatasetError>,
    ) -> Result<T, DatasetError> {
        self.variables.with_variable(self.reader, name, read)
    }

    pub fn grid(&self, dimensions: &[u64]) -> Result<Arc<SpatialGrid>, DatasetError> {
        spatial_grid(self.reader, dimensions)
    }

    pub fn sample_point(
        &self,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError> {
        let sampler = SpatialSampler::prepare(self, &[variable_name], latitude, longitude)?
            .ok_or_else(|| DatasetError::VariableNotFound {
                variable: variable_name.to_string(),
            })?;
        sampler.read(variable_name)
    }

    pub fn sample_points(
        &self,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<Option<f64>>, DatasetError> {
        if variable_names.is_empty() {
            return Ok(Vec::new());
        }
        let Some(sampler) = SpatialSampler::prepare(self, variable_names, latitude, longitude)?
        else {
            return Ok(vec![None; variable_names.len()]);
        };
        sampler.read_many(variable_names)
    }
}

pub struct OmSpatialFileSessionWithIndex<'a, B> {
    reader: &'a OmFileReader<B>,
    variables: VariableIndex,
}

impl<'a, B: OmFileReaderBackend> OmSpatialFileSessionWithIndex<'a, B> {
    pub fn from_reader(reader: &'a OmFileReader<B>) -> Self {
        Self {
            reader,
            variables: VariableIndex::build(reader),
        }
    }

    pub fn session(&'a self) -> OmSpatialFileSession<'a, B> {
        OmSpatialFileSession {
            reader: self.reader,
            variables: &self.variables,
        }
    }

    pub fn sample_point(
        &self,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError> {
        self.session()
            .sample_point(variable_name, latitude, longitude)
    }

    pub fn sample_points(
        &self,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Vec<Option<f64>>, DatasetError> {
        self.session()
            .sample_points(variable_names, latitude, longitude)
    }
}

pub fn spatial_grid<B: OmFileReaderBackend>(
    reader: &OmFileReader<B>,
    dimensions: &[u64],
) -> Result<Arc<SpatialGrid>, DatasetError> {
    spatial_grid_cache::get_or_insert(SpatialGridMetadata {
        dimensions: dimensions.to_vec(),
        coordinates: read_string_child(reader, "coordinates").ok_or(
            DatasetError::MissingMetadata {
                field: "coordinates",
            },
        )?,
        crs_wkt: read_string_child(reader, "crs_wkt")
            .ok_or(DatasetError::MissingMetadata { field: "crs_wkt" })?,
    })
}

fn read_string_child<B: OmFileReaderBackend>(
    reader: &OmFileReader<B>,
    name: &str,
) -> Option<String> {
    let child = reader.get_child_by_name(name)?;
    child.expect_scalar().ok()?.read_scalar::<String>()
}
