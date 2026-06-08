use std::collections::HashMap;

use omfiles::OmDataType;
use omfiles::reader::OmFileReader;
use omfiles::traits::{OmArrayVariable, OmFileReadable, OmFileReaderBackend, OmFileVariable};

use crate::domain::VariableMeta;
use crate::error::DatasetError;

#[derive(Debug, Clone)]
struct VariableEntry {
    name: String,
    path: Vec<u32>,
    data_type: String,
    dimensions: Vec<u64>,
    chunks: Vec<u64>,
}

#[derive(Debug, Default)]
pub struct VariableIndex {
    ordered: Vec<VariableEntry>,
    by_name: HashMap<String, usize>,
}

impl VariableIndex {
    pub fn build<B: OmFileReaderBackend>(reader: &OmFileReader<B>) -> Self {
        let mut ordered = Vec::new();
        let mut by_name = HashMap::new();
        visit(reader, &[], &mut ordered, &mut by_name);
        Self { ordered, by_name }
    }

    pub fn first_dimensions(&self, names: &[&str]) -> Option<&[u64]> {
        names
            .iter()
            .find_map(|name| self.entry(name))
            .map(|entry| entry.dimensions.as_slice())
    }

    pub fn metadata(&self) -> Vec<VariableMeta> {
        self.ordered
            .iter()
            .map(|entry| VariableMeta {
                name: entry.name.clone(),
                data_type: entry.data_type.clone(),
                dimensions: entry.dimensions.clone(),
                chunks: entry.chunks.clone(),
            })
            .collect()
    }

    pub fn with_variable<B: OmFileReaderBackend, T>(
        &self,
        root: &OmFileReader<B>,
        name: &str,
        read: impl FnOnce(&OmFileReader<B>) -> Result<T, DatasetError>,
    ) -> Result<T, DatasetError> {
        let entry = self
            .entry(name)
            .ok_or_else(|| DatasetError::VariableNotFound {
                variable: name.to_string(),
            })?;
        if entry.dimensions.is_empty() {
            return Err(DatasetError::VariableNotFound {
                variable: name.to_string(),
            });
        }
        if entry.path.is_empty() {
            if root.name() == entry.name {
                return read(root);
            }
            return Err(DatasetError::VariableNotFound {
                variable: name.to_string(),
            });
        }
        let variable =
            resolve_path(root, &entry.path).ok_or_else(|| DatasetError::VariableNotFound {
                variable: name.to_string(),
            })?;
        read(&variable)
    }

    fn entry(&self, name: &str) -> Option<&VariableEntry> {
        self.by_name
            .get(name)
            .and_then(|index| self.ordered.get(*index))
    }
}

fn resolve_path<B: OmFileReaderBackend>(
    root: &OmFileReader<B>,
    path: &[u32],
) -> Option<OmFileReader<B>> {
    let index = path.first().copied()?;
    let mut current = root.get_child_by_index(index)?;
    for &child_index in &path[1..] {
        current = current.get_child_by_index(child_index)?;
    }
    Some(current)
}

fn visit<B: OmFileReaderBackend>(
    reader: &OmFileReader<B>,
    path: &[u32],
    ordered: &mut Vec<VariableEntry>,
    by_name: &mut HashMap<String, usize>,
) {
    let name = reader.name();
    if !name.is_empty() && (reader.data_type().is_array() || reader.data_type().is_scalar()) {
        let mut entry = VariableEntry {
            name: name.to_string(),
            path: path.to_vec(),
            data_type: data_type_label(reader.data_type()).to_string(),
            dimensions: Vec::new(),
            chunks: Vec::new(),
        };
        if reader.data_type().is_array()
            && let Ok(array) = reader.expect_array()
        {
            entry.dimensions = array.get_dimensions().to_vec();
            entry.chunks = array.get_chunk_dimensions().to_vec();
        }
        by_name.insert(entry.name.clone(), ordered.len());
        ordered.push(entry);
    }

    for index in 0..reader.number_of_children() {
        if let Some(child) = reader.get_child_by_index(index) {
            let mut child_path = path.to_vec();
            child_path.push(index);
            visit(&child, &child_path, ordered, by_name);
        }
    }
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
