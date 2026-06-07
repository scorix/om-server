use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use omfiles::OmDataType;
use omfiles::reader::OmFileReader;
use omfiles::traits::{OmArrayVariable, OmFileReadable, OmFileReaderBackend, OmFileVariable};

use crate::domain::{OmDatasetMeta, OmReaderBackend, VariableMeta};

pub struct OmDatasetReader;

impl OmDatasetReader {
    pub fn read_meta(backend: OmReaderBackend) -> Result<OmDatasetMeta> {
        match backend {
            OmReaderBackend::LocalMmap(path) => {
                let reader = OmFileReader::from_file(
                    path.to_str()
                        .with_context(|| format!("non-UTF8 path {}", path.display()))?,
                )
                .with_context(|| format!("open om file {}", path.display()))?;
                Self::read_meta_from_reader(reader, path)
            }
            OmReaderBackend::RangeHttp { base_url } => {
                let backend = super::RangeHttpBackend::new(base_url)?;
                let reader = OmFileReader::new(Arc::new(backend))?;
                Self::read_meta_from_reader(reader, PathBuf::new())
            }
        }
    }

    pub fn read_meta_from_reader<B>(
        reader: OmFileReader<B>,
        local_path: PathBuf,
    ) -> Result<OmDatasetMeta>
    where
        B: OmFileReaderBackend,
    {
        let variables = collect_variables(&reader, "")?;
        Ok(OmDatasetMeta {
            local_path,
            variables,
        })
    }
}

fn collect_variables<B>(
    reader: &OmFileReader<B>,
    prefix: &str,
) -> Result<Vec<VariableMeta>>
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
) -> Result<()>
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
                .context("expected array variable metadata")?;
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
