use std::path::PathBuf;

use crate::error::{DatasetError, SyncError};

use super::{DatasetMeta, ObjectKey};

#[derive(Debug, Clone)]
pub enum DatasetLocation {
    LocalFile(PathBuf),
    HttpRange { url: String },
}

pub trait DatasetReader: Send + Sync {
    fn read_meta(&self, location: DatasetLocation) -> Result<DatasetMeta, DatasetError>;
}

pub trait ObjectFetcher: Send + Sync {
    fn sync_object(&self, object_key: &ObjectKey) -> Result<(), SyncError>;

    fn synced_path(&self, object_key: &ObjectKey) -> PathBuf;
}
