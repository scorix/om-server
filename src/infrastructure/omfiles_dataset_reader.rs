use std::path::{Path, PathBuf};
use std::sync::Arc;

use omfiles::OmDataType;
use omfiles::reader::OmFileReader;
use omfiles::traits::{
    OmArrayVariable, OmFileReadable, OmFileReaderBackend, OmFileVariable, OmScalarVariable,
};

use crate::domain::{
    DatasetLocation, DatasetMeta, DatasetReader, SpatialGrid, SpatialGridMetadata, VariableMeta,
};
use crate::error::DatasetError;

#[derive(Debug, Default, Clone, Copy)]
pub struct OmfilesDatasetReader;

impl DatasetReader for OmfilesDatasetReader {
    fn read_meta(&self, location: DatasetLocation) -> Result<DatasetMeta, DatasetError> {
        match location {
            DatasetLocation::LocalFile(path) => {
                let path_str = path
                    .to_str()
                    .ok_or_else(|| DatasetError::NonUtf8Path { path: path.clone() })?;
                let reader =
                    OmFileReader::from_file(path_str).map_err(|source| DatasetError::OpenFile {
                        path: path.clone(),
                        source,
                    })?;
                Self::read_meta_from_reader(reader, path)
            }
            DatasetLocation::HttpRange { url } => {
                let backend = super::HttpRangeReader::new(url.clone())?;
                let reader = OmFileReader::new(Arc::new(backend)).map_err(|source| {
                    DatasetError::OpenFile {
                        path: PathBuf::from(url),
                        source,
                    }
                })?;
                Self::read_meta_from_reader(reader, PathBuf::new())
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
        let variables = collect_variables(&reader, "")?;
        Ok(DatasetMeta {
            local_path,
            variables,
        })
    }

    pub fn read_spatial_grid_from_reader<B>(
        reader: &OmFileReader<B>,
        dimensions: &[u64],
    ) -> Result<SpatialGrid, DatasetError>
    where
        B: OmFileReaderBackend,
    {
        SpatialGrid::from_metadata(SpatialGridMetadata {
            dimensions: dimensions.to_vec(),
            coordinates: read_string_child(reader, "coordinates").ok_or(
                DatasetError::MissingMetadata {
                    field: "coordinates",
                },
            )?,
            crs_wkt: read_string_child(reader, "crs_wkt")
                .ok_or(DatasetError::MissingMetadata { field: "crs_wkt" })?,
        })
        .map_err(DatasetError::Grid)
    }

    pub fn read_spatial_point_from_local(
        path: &Path,
        variable_name: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, DatasetError> {
        let path_str = path.to_str().ok_or_else(|| DatasetError::NonUtf8Path {
            path: path.to_path_buf(),
        })?;
        let reader =
            OmFileReader::from_file(path_str).map_err(|source| DatasetError::OpenFile {
                path: path.to_path_buf(),
                source,
            })?;
        Self::read_spatial_point_from_reader(&reader, variable_name, latitude, longitude)
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
        let variable =
            find_variable(reader, variable_name).ok_or_else(|| DatasetError::VariableNotFound {
                variable: variable_name.to_string(),
            })?;
        let array = variable
            .expect_array()
            .map_err(|source| DatasetError::NotArray {
                variable: variable_name.to_string(),
                source,
            })?;
        let grid = Self::read_spatial_grid_from_reader(reader, array.get_dimensions())?;
        let point = grid
            .point_window(latitude, longitude)
            .map_err(DatasetError::Grid)?;
        let values =
            array
                .read::<f32>(&point.ranges)
                .map_err(|source| DatasetError::ReadVariable {
                    variable: variable_name.to_string(),
                    source,
                })?;
        point
            .value(
                values
                    .as_slice()
                    .ok_or_else(|| DatasetError::NonContiguousValues {
                        variable: variable_name.to_string(),
                    })?,
            )
            .map_err(DatasetError::Grid)
    }
}

fn find_variable<B>(reader: &OmFileReader<B>, name: &str) -> Option<OmFileReader<B>>
where
    B: OmFileReaderBackend,
{
    for index in 0..reader.number_of_children() {
        let child = reader.get_child_by_index(index)?;
        if child.name() == name {
            return Some(child);
        }
        if let Some(found) = find_variable(&child, name) {
            return Some(found);
        }
    }
    None
}

fn collect_variables<B>(
    reader: &OmFileReader<B>,
    prefix: &str,
) -> Result<Vec<VariableMeta>, DatasetError>
where
    B: OmFileReaderBackend,
{
    let mut variables = Vec::new();
    visit_variables(reader, prefix, &mut variables)?;
    Ok(variables)
}

fn visit_variables<B>(
    reader: &OmFileReader<B>,
    prefix: &str,
    out: &mut Vec<VariableMeta>,
) -> Result<(), DatasetError>
where
    B: OmFileReaderBackend,
{
    let name = reader.name();
    if !name.is_empty() && (reader.data_type().is_array() || reader.data_type().is_scalar()) {
        let full_name = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix}/{name}")
        };
        let mut meta = VariableMeta {
            name: full_name,
            data_type: data_type_label(reader.data_type()).to_string(),
            dimensions: Vec::new(),
            chunks: Vec::new(),
        };
        if reader.data_type().is_array() {
            let array = reader
                .expect_array()
                .map_err(|source| DatasetError::ExpectedArray { source })?;
            meta.dimensions = array.get_dimensions().to_vec();
            meta.chunks = array.get_chunk_dimensions().to_vec();
        }
        out.push(meta);
    }

    for index in 0..reader.number_of_children() {
        if let Some(child) = reader.get_child_by_index(index) {
            let child_prefix = if name.is_empty() {
                prefix.to_string()
            } else if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{prefix}/{name}")
            };
            visit_variables(&child, &child_prefix, out)?;
        }
    }
    Ok(())
}

fn data_type_label(data_type: OmDataType) -> &'static str {
    match data_type {
        OmDataType::None => "none",
        OmDataType::Int8 => "int8",
        OmDataType::Uint8 => "uint8",
        OmDataType::Int16 => "int16",
        OmDataType::Uint16 => "uint16",
        OmDataType::Int32 => "int32",
        OmDataType::Uint32 => "uint32",
        OmDataType::Int64 => "int64",
        OmDataType::Uint64 => "uint64",
        OmDataType::Float => "float",
        OmDataType::Double => "double",
        OmDataType::String => "string",
        OmDataType::Int8Array => "int8_array",
        OmDataType::Uint8Array => "uint8_array",
        OmDataType::Int16Array => "int16_array",
        OmDataType::Uint16Array => "uint16_array",
        OmDataType::Int32Array => "int32_array",
        OmDataType::Uint32Array => "uint32_array",
        OmDataType::Int64Array => "int64_array",
        OmDataType::Uint64Array => "uint64_array",
        OmDataType::FloatArray => "float_array",
        OmDataType::DoubleArray => "double_array",
        OmDataType::StringArray => "string_array",
    }
}

fn read_string_child<B>(reader: &OmFileReader<B>, name: &str) -> Option<String>
where
    B: OmFileReaderBackend,
{
    let child = reader.get_child_by_name(name)?;
    child.expect_scalar().ok()?.read_scalar::<String>()
}
