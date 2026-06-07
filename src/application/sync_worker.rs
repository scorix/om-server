use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::domain::{
    ObjectFetcher, SourceRegistry, SpatialObjectLocal, SpatialRun, SpatialRunCatalog,
    SpatialRunSnapshot, WeatherModelId,
};
use crate::error::SyncWorkerError;

pub struct SpatialSyncWorkerConfig {
    pub run_catalog: Arc<dyn SpatialRunCatalog>,
    pub sync_dir: PathBuf,
    pub models: Vec<WeatherModelId>,
    pub forecast_days: usize,
    pub interval: Duration,
}

pub struct SpatialSyncWorker<F> {
    registry: Arc<SourceRegistry>,
    fetcher: Arc<F>,
    catalog: Arc<ActiveSpatialCatalog>,
    run_catalog: Arc<dyn SpatialRunCatalog>,
    sync_dir: std::path::PathBuf,
    models: Vec<WeatherModelId>,
    forecast_days: usize,
    interval: Duration,
}

impl<F> SpatialSyncWorker<F>
where
    F: ObjectFetcher + Send + Sync + 'static,
{
    pub fn new(
        registry: SourceRegistry,
        fetcher: F,
        catalog: Arc<ActiveSpatialCatalog>,
        config: SpatialSyncWorkerConfig,
    ) -> Self {
        Self {
            registry: Arc::new(registry),
            fetcher: Arc::new(fetcher),
            catalog,
            run_catalog: config.run_catalog,
            sync_dir: config.sync_dir,
            models: config.models,
            forecast_days: config.forecast_days,
            interval: config.interval,
        }
    }

    pub async fn run_forever(self) {
        loop {
            match tokio::task::spawn_blocking({
                let worker = self.clone_for_blocking();
                move || worker.sync_all_models()
            })
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    tracing::error!(error = %error, "spatial sync worker sync failed");
                }
                Err(error) => {
                    tracing::error!(error = %error, "spatial sync worker task failed");
                }
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    fn clone_for_blocking(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            fetcher: self.fetcher.clone(),
            catalog: self.catalog.clone(),
            run_catalog: self.run_catalog.clone(),
            sync_dir: self.sync_dir.clone(),
            models: self.models.clone(),
            forecast_days: self.forecast_days,
            interval: self.interval,
        }
    }

    fn sync_all_models(&self) -> Result<(), SyncWorkerError> {
        for &model in &self.models {
            if let Err(error) = self.sync_model(model) {
                tracing::error!(model = %model, error = %error, "spatial model sync failed");
            }
        }
        if self.catalog.is_ready(&self.models) {
            tracing::info!(models = ?self.models, "spatial sync cycle complete");
        }
        Ok(())
    }

    fn sync_model(&self, model: WeatherModelId) -> Result<(), SyncWorkerError> {
        let run = self
            .run_catalog
            .resolve_spatial_run(model)
            .map_err(SyncWorkerError::OpenMeteo)?;
        if self.catalog.get(model).is_some_and(|active| {
            run.matches_snapshot(&active)
                && active
                    .objects
                    .iter()
                    .all(|object| object.local_path.exists())
        }) {
            let active = self.catalog.get(model).expect("snapshot checked above");
            self.log_cached_snapshot(model, &run.run_ref, &active);
            tracing::info!(
                model = %model,
                reference_time = %run.reference_time,
                run_ref = %run.run_ref,
                objects = active.objects.len(),
                "spatial run up to date"
            );
            self.catalog.mark_verified(model);
            return Ok(());
        }

        let planned_objects = run.planned_objects(self.forecast_days);
        tracing::info!(
            model = %model,
            reference_time = %run.reference_time,
            run_ref = %run.run_ref,
            total = planned_objects.len(),
            "syncing spatial run"
        );
        let snapshot = self.build_snapshot(model, &run, &planned_objects)?;
        let object_count = snapshot.objects.len();
        self.catalog
            .publish(&self.sync_dir, Arc::new(snapshot))
            .map_err(SyncWorkerError::ActiveCatalog)?;
        tracing::info!(
            model = %model,
            run_ref = %run.run_ref,
            objects = object_count,
            "published active spatial run"
        );
        self.catalog.mark_verified(model);
        Ok(())
    }

    fn build_snapshot(
        &self,
        model: WeatherModelId,
        run: &SpatialRun,
        planned_objects: &[crate::domain::SpatialObjectRef],
    ) -> Result<SpatialRunSnapshot, SyncWorkerError> {
        use crate::domain::ObjectKey;

        let total = planned_objects.len();
        if total == 0 {
            return Err(SyncWorkerError::EmptyRun { model });
        }

        let mut objects = Vec::with_capacity(total);
        let mut downloaded = 0usize;
        let mut skipped = 0usize;
        for (index, object) in planned_objects.iter().enumerate() {
            let object_key = ObjectKey(object.object_key.clone());
            let local_path = self.fetcher.synced_path(&object_key);
            let cached = local_path.exists();
            self.fetcher
                .sync_object(&object_key)
                .map_err(SyncWorkerError::Sync)?;
            log_object_progress(
                model,
                &run.run_ref,
                SyncObjectProgress {
                    step: index + 1,
                    total,
                    object_key: &object_key.0,
                    timestamp: &object.timestamp,
                    valid_date: &object.valid_date,
                    cached,
                    size_bytes: local_file_size(&local_path),
                },
            );
            if cached {
                skipped += 1;
            } else {
                downloaded += 1;
            }
            objects.push(SpatialObjectLocal {
                object_key: object_key.0.clone(),
                timestamp: object.timestamp.clone(),
                valid_date: object.valid_date.clone(),
                local_path,
            });
        }
        tracing::info!(
            model = %model,
            run_ref = %run.run_ref,
            total,
            downloaded,
            skipped,
            "spatial run download complete"
        );
        Ok(SpatialRunSnapshot {
            model,
            reference_time: run.reference_time.clone(),
            run_ref: run.run_ref.clone(),
            objects,
        })
    }

    fn log_cached_snapshot(
        &self,
        model: WeatherModelId,
        run_ref: &str,
        snapshot: &SpatialRunSnapshot,
    ) {
        let total = snapshot.objects.len();
        for (index, object) in snapshot.objects.iter().enumerate() {
            log_object_progress(
                model,
                run_ref,
                SyncObjectProgress {
                    step: index + 1,
                    total,
                    object_key: &object.object_key,
                    timestamp: &object.timestamp,
                    valid_date: &object.valid_date,
                    cached: object.local_path.exists(),
                    size_bytes: local_file_size(&object.local_path),
                },
            );
        }
        tracing::info!(
            model = %model,
            run_ref,
            total,
            downloaded = 0,
            skipped = total,
            "spatial run download complete"
        );
    }
}

struct SyncObjectProgress<'a> {
    step: usize,
    total: usize,
    object_key: &'a str,
    timestamp: &'a str,
    valid_date: &'a str,
    cached: bool,
    size_bytes: Option<u64>,
}

fn local_file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|meta| meta.len())
}

fn log_object_progress(model: WeatherModelId, run_ref: &str, progress: SyncObjectProgress<'_>) {
    tracing::info!(
        model = %model,
        run_ref,
        step = progress.step,
        total = progress.total,
        percent = sync_percent(progress.step, progress.total),
        object_key = progress.object_key,
        timestamp = progress.timestamp,
        valid_date = progress.valid_date,
        cached = progress.cached,
        size = %format_file_size_label(progress.size_bytes),
        "spatial sync progress"
    );
}

fn format_file_size_label(bytes: Option<u64>) -> String {
    bytes
        .map(format_file_size)
        .unwrap_or_else(|| "-".to_string())
}

fn format_file_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;

    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn sync_percent(step: usize, total: usize) -> u8 {
    ((step.saturating_mul(100)) / total.max(1)).min(100) as u8
}

#[cfg(test)]
mod tests {
    use super::{format_file_size, format_file_size_label, sync_percent};
    use crate::domain::{ObjectKey, SpatialObjectRef};

    #[test]
    fn sync_percent_reaches_one_hundred_on_last_step() {
        assert_eq!(sync_percent(1, 4), 25);
        assert_eq!(sync_percent(4, 4), 100);
    }

    #[test]
    fn format_file_size_uses_binary_units() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1536), "1.5 KiB");
        assert_eq!(format_file_size(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(format_file_size_label(None), "-");
    }

    #[test]
    fn listed_object_key_can_differ_from_reconstructed_spatial_path() {
        let listed = SpatialObjectRef {
            object_key: "data_spatial/ecmwf_ifs025/2026/06/08/00/2026-06-24T1200.om".to_string(),
            timestamp: "2026-06-24T1200".to_string(),
            valid_date: "2026-06-24".to_string(),
        };
        let reconstructed = crate::infrastructure::open_meteo::OpenMeteoSpatialLayout::ECMWF_IFS025
            .object_key("00", &listed.timestamp)
            .expect("object key");
        assert_ne!(ObjectKey(listed.object_key), reconstructed);
    }
}
