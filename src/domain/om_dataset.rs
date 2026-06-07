use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariableMeta {
    pub name: String,
    pub data_type: String,
    pub dimensions: Vec<u64>,
    pub chunks: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OmDatasetMeta {
    pub local_path: PathBuf,
    pub variables: Vec<VariableMeta>,
}
