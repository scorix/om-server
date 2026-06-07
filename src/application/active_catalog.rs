use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::domain::{
    ObjectFetcher, ObjectKey, SpatialObjectLocal, SpatialRunSnapshot, WeatherModelId,
};
use crate::error::ActiveCatalogError;

#[derive(Debug, Default)]
pub struct ActiveSpatialCatalog {
    snapshots: RwLock<HashMap<WeatherModelId, Arc<SpatialRunSnapshot>>>,
    verified: RwLock<HashSet<WeatherModelId>>,
}

impl ActiveSpatialCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, model: WeatherModelId) -> Option<Arc<SpatialRunSnapshot>> {
        self.snapshots
            .read()
            .expect("active spatial catalog lock poisoned")
            .get(&model)
            .cloned()
    }

    pub fn is_ready(&self, models: &[WeatherModelId]) -> bool {
        let snapshots = self
            .snapshots
            .read()
            .expect("active spatial catalog lock poisoned");
        let verified = self
            .verified
            .read()
            .expect("active spatial catalog lock poisoned");
        models
            .iter()
            .all(|model| verified.contains(model) && snapshots.contains_key(model))
    }

    pub fn mark_verified(&self, model: WeatherModelId) {
        self.verified
            .write()
            .expect("active spatial catalog lock poisoned")
            .insert(model);
    }

    pub fn publish(
        &self,
        sync_dir: &Path,
        snapshot: Arc<SpatialRunSnapshot>,
    ) -> Result<(), ActiveCatalogError> {
        persist_snapshot(sync_dir, &snapshot)?;
        self.snapshots
            .write()
            .expect("active spatial catalog lock poisoned")
            .insert(snapshot.model, snapshot);
        Ok(())
    }

    pub fn load_persisted<F>(
        sync_dir: &Path,
        fetcher: &F,
        models: &[WeatherModelId],
    ) -> Result<Self, ActiveCatalogError>
    where
        F: ObjectFetcher,
    {
        let catalog = Self::new();
        for &model in models {
            let Some(snapshot) = read_persisted_snapshot(sync_dir, fetcher, model)? else {
                continue;
            };
            catalog
                .snapshots
                .write()
                .expect("active spatial catalog lock poisoned")
                .insert(model, Arc::new(snapshot));
        }
        Ok(catalog)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedSnapshot {
    #[serde(default)]
    reference_time: String,
    run_ref: String,
    objects: Vec<PersistedObject>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedObject {
    timestamp: String,
    valid_date: String,
    object_key: String,
}

fn active_manifest_path(sync_dir: &Path, model: WeatherModelId) -> PathBuf {
    sync_dir
        .join("active")
        .join(format!("{}.json", model.as_str()))
}

fn persist_snapshot(
    sync_dir: &Path,
    snapshot: &SpatialRunSnapshot,
) -> Result<(), ActiveCatalogError> {
    let manifest = PersistedSnapshot {
        reference_time: snapshot.reference_time.clone(),
        run_ref: snapshot.run_ref.clone(),
        objects: snapshot
            .objects
            .iter()
            .map(|object| PersistedObject {
                timestamp: object.timestamp.clone(),
                valid_date: object.valid_date.clone(),
                object_key: object.object_key.clone(),
            })
            .collect(),
    };
    let active_dir = sync_dir.join("active");
    fs::create_dir_all(&active_dir).map_err(|source| ActiveCatalogError::WriteManifest {
        path: active_dir,
        source,
    })?;
    let path = active_manifest_path(sync_dir, snapshot.model);
    let temp_path = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(&manifest).map_err(|source| {
        ActiveCatalogError::SerializeManifest {
            model: snapshot.model,
            source,
        }
    })?;
    fs::write(&temp_path, body).map_err(|source| ActiveCatalogError::WriteManifest {
        path: temp_path.clone(),
        source,
    })?;
    fs::rename(&temp_path, &path).map_err(|source| ActiveCatalogError::WriteManifest {
        path: path.clone(),
        source,
    })?;
    Ok(())
}

fn read_persisted_snapshot<F>(
    sync_dir: &Path,
    fetcher: &F,
    model: WeatherModelId,
) -> Result<Option<SpatialRunSnapshot>, ActiveCatalogError>
where
    F: ObjectFetcher,
{
    let path = active_manifest_path(sync_dir, model);
    if !path.exists() {
        return Ok(None);
    }
    let body = fs::read(&path).map_err(|source| ActiveCatalogError::ReadManifest {
        path: path.clone(),
        source,
    })?;
    let manifest: PersistedSnapshot =
        serde_json::from_slice(&body).map_err(|source| ActiveCatalogError::ParseManifest {
            path: path.clone(),
            source,
        })?;
    let mut objects = Vec::with_capacity(manifest.objects.len());
    for object in manifest.objects {
        let object_key = object.object_key.clone();
        let local_path = if Path::new(&object_key).is_absolute() {
            PathBuf::from(&object_key)
        } else {
            fetcher.synced_path(&ObjectKey(object_key.clone()))
        };
        if !local_path.exists() {
            return Ok(None);
        }
        objects.push(SpatialObjectLocal {
            object_key,
            timestamp: object.timestamp,
            valid_date: object.valid_date,
            local_path,
        });
    }
    if objects.is_empty() {
        return Ok(None);
    }
    Ok(Some(SpatialRunSnapshot {
        model,
        reference_time: manifest.reference_time,
        run_ref: manifest.run_ref,
        objects,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HttpError;
    use crate::infrastructure::S3ObjectFetcher;
    use crate::infrastructure::http::HttpClient;

    struct FixtureClient;

    impl HttpClient for FixtureClient {
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpError> {
            unimplemented!()
        }

        fn download_to(&self, _url: &str, _path: &std::path::Path) -> Result<(), HttpError> {
            unimplemented!()
        }

        fn get_range(&self, _url: &str, _offset: u64, _count: u64) -> Result<Vec<u8>, HttpError> {
            unimplemented!()
        }

        fn probe_content_length(&self, _url: &str) -> Result<u64, HttpError> {
            unimplemented!()
        }
    }

    #[test]
    fn persists_and_reloads_active_snapshot_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fetcher =
            S3ObjectFetcher::with_client("https://example.test", temp.path(), FixtureClient);
        let local_path = temp
            .path()
            .join("data_spatial/ecmwf_ifs025/2026/06/08/00/file.om");
        std::fs::create_dir_all(local_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&local_path, b"fixture").expect("write");

        let snapshot = Arc::new(SpatialRunSnapshot {
            model: WeatherModelId::EcmwfIfs025,
            reference_time: "2026/06/08/00".to_string(),
            run_ref: "2026/06/08/00".to_string(),
            objects: vec![SpatialObjectLocal {
                object_key: "data_spatial/ecmwf_ifs025/2026/06/08/00/file.om".to_string(),
                timestamp: "2026-06-08T0000".to_string(),
                valid_date: "2026-06-08".to_string(),
                local_path: local_path.clone(),
            }],
        });
        let catalog = ActiveSpatialCatalog::new();
        catalog
            .publish(temp.path(), snapshot)
            .expect("publish snapshot");

        let loaded = ActiveSpatialCatalog::load_persisted(
            temp.path(),
            &fetcher,
            &[WeatherModelId::EcmwfIfs025],
        )
        .expect("load persisted");
        let restored = loaded.get(WeatherModelId::EcmwfIfs025).expect("snapshot");
        assert_eq!(restored.run_ref, "2026/06/08/00");
        assert_eq!(restored.objects[0].local_path, local_path);
    }

    #[test]
    fn persisted_snapshot_is_not_ready_until_worker_verifies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let fetcher =
            S3ObjectFetcher::with_client("https://example.test", temp.path(), FixtureClient);
        let local_path = temp
            .path()
            .join("data_spatial/ecmwf_ifs025/2026/06/08/00/file.om");
        std::fs::create_dir_all(local_path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&local_path, b"fixture").expect("write");

        let snapshot = Arc::new(SpatialRunSnapshot {
            model: WeatherModelId::EcmwfIfs025,
            reference_time: "2026/06/08/00".to_string(),
            run_ref: "2026/06/08/00".to_string(),
            objects: vec![SpatialObjectLocal {
                object_key: "data_spatial/ecmwf_ifs025/2026/06/08/00/file.om".to_string(),
                timestamp: "2026-06-08T0000".to_string(),
                valid_date: "2026-06-08".to_string(),
                local_path: local_path.clone(),
            }],
        });
        let catalog = ActiveSpatialCatalog::new();
        catalog
            .publish(temp.path(), snapshot)
            .expect("publish snapshot");

        let loaded = ActiveSpatialCatalog::load_persisted(
            temp.path(),
            &fetcher,
            &[WeatherModelId::EcmwfIfs025],
        )
        .expect("load persisted");
        assert!(loaded.get(WeatherModelId::EcmwfIfs025).is_some());
        assert!(!loaded.is_ready(&[WeatherModelId::EcmwfIfs025]));

        loaded.mark_verified(WeatherModelId::EcmwfIfs025);
        assert!(loaded.is_ready(&[WeatherModelId::EcmwfIfs025]));
    }
}
