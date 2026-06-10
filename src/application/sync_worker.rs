use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use tokio::sync::Notify;

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::domain::{
    ObjectFetcher, ObjectKey, SourceRegistry, SpatialObjectLocal, SpatialRun, SpatialRunCatalog,
    SpatialRunSnapshot, WeatherModelId,
};
use crate::error::{SyncError, SyncWorkerError};

pub struct SpatialSyncWorkerConfig {
    pub run_catalog: Arc<dyn SpatialRunCatalog>,
    pub sync_dir: PathBuf,
    pub models: Vec<WeatherModelId>,
    pub forecast_days: usize,
    pub interval: Duration,
    pub parallelism: usize,
    pub retry_attempts: usize,
    pub retry_delay: Duration,
    pub bake_wake: Option<Arc<Notify>>,
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
    parallelism: usize,
    retry_attempts: usize,
    retry_delay: Duration,
    bake_wake: Option<Arc<Notify>>,
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
            parallelism: config.parallelism.max(1),
            retry_attempts: config.retry_attempts.max(1),
            retry_delay: config.retry_delay,
            bake_wake: config.bake_wake,
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
            parallelism: self.parallelism,
            retry_attempts: self.retry_attempts,
            retry_delay: self.retry_delay,
            bake_wake: self.bake_wake.clone(),
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
            self.signal_bake_wake(model, &run.run_ref);
            return Ok(());
        }

        let planned_objects = run.planned_objects(self.forecast_days);
        tracing::info!(
            model = %model,
            reference_time = %run.reference_time,
            run_ref = %run.run_ref,
            total = planned_objects.len(),
            parallelism = self.parallelism,
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
        self.signal_bake_wake(model, &run.run_ref);
        Ok(())
    }

    fn signal_bake_wake(&self, model: WeatherModelId, run_ref: &str) {
        let Some(wake) = &self.bake_wake else {
            return;
        };
        tracing::info!(
            model = %model,
            run_ref,
            "signaling weather bake worker"
        );
        wake.notify_waiters();
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

        let fetcher = self.fetcher.clone();
        let run_ref = run.run_ref.clone();
        let parallelism = self.parallelism.min(total);
        let next_job = AtomicUsize::new(0);
        let completed = AtomicUsize::new(0);
        let failure = Mutex::new(None::<SyncWorkerError>);
        let slots: Vec<Mutex<Option<DownloadedObject>>> =
            (0..total).map(|_| Mutex::new(None)).collect();

        std::thread::scope(|scope| {
            for _ in 0..parallelism {
                scope.spawn(|| {
                    loop {
                        if failure.lock().expect("sync failure lock").is_some() {
                            break;
                        }
                        let index = next_job.fetch_add(1, Ordering::Relaxed);
                        if index >= total {
                            break;
                        }
                        let object = &planned_objects[index];
                        let object_key = ObjectKey(object.object_key.clone());
                        let local_path = fetcher.synced_path(&object_key);
                        let cached = local_path.exists();
                        if let Err(error) = sync_object_with_retry(
                            fetcher.as_ref(),
                            &object_key,
                            model,
                            self.retry_attempts,
                            self.retry_delay,
                        ) {
                            *failure.lock().expect("sync failure lock") =
                                Some(SyncWorkerError::Sync(error));
                            break;
                        }
                        *slots[index].lock().expect("sync slot lock") = Some(DownloadedObject {
                            cached,
                            object: SpatialObjectLocal {
                                object_key: object_key.0.clone(),
                                timestamp: object.timestamp.clone(),
                                valid_date: object.valid_date.clone(),
                                local_path: local_path.clone(),
                            },
                        });
                        let step = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        log_object_progress(
                            model,
                            &run_ref,
                            SyncObjectProgress {
                                step,
                                total,
                                object_key: &object_key.0,
                                timestamp: &object.timestamp,
                                valid_date: &object.valid_date,
                                cached,
                                size_bytes: local_file_size(&local_path),
                            },
                        );
                    }
                });
            }
        });

        if let Some(error) = failure.lock().expect("sync failure lock").take() {
            return Err(error);
        }

        let mut downloaded = 0usize;
        let mut skipped = 0usize;
        let objects = slots
            .into_iter()
            .enumerate()
            .map(|(index, slot)| {
                let slot = slot
                    .into_inner()
                    .expect("sync slot lock")
                    .unwrap_or_else(|| {
                        panic!(
                            "missing downloaded object at index {index}: {}",
                            planned_objects[index].object_key
                        )
                    });
                if slot.cached {
                    skipped += 1;
                } else {
                    downloaded += 1;
                }
                slot.object
            })
            .collect::<Vec<_>>();
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

struct DownloadedObject {
    object: SpatialObjectLocal,
    cached: bool,
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

fn sync_object_with_retry<F>(
    fetcher: &F,
    object_key: &ObjectKey,
    model: WeatherModelId,
    max_attempts: usize,
    base_delay: Duration,
) -> Result<(), SyncError>
where
    F: ObjectFetcher,
{
    let mut attempt = 1usize;
    let mut delay = base_delay;
    loop {
        match fetcher.sync_object(object_key) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= max_attempts => return Err(error),
            Err(error) => {
                remove_partial_download(fetcher.synced_path(object_key));
                tracing::warn!(
                    model = %model,
                    object_key = %object_key.0,
                    attempt,
                    max_attempts,
                    delay_secs = delay.as_secs(),
                    error = %error,
                    "spatial sync object failed, retrying"
                );
                thread::sleep(delay);
                attempt += 1;
                delay = delay.saturating_mul(2);
            }
        }
    }
}

fn remove_partial_download(dest: PathBuf) {
    if dest.exists() {
        return;
    }
    let partial = dest.with_extension("partial");
    let _ = std::fs::remove_file(partial);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::{format_file_size, format_file_size_label, sync_object_with_retry, sync_percent};
    use crate::domain::{ObjectFetcher, ObjectKey, SpatialObjectRef, WeatherModelId};
    use crate::error::{HttpError, SyncError};

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

    struct FlakyFetcher {
        attempts: AtomicUsize,
        fail_until: usize,
    }

    impl ObjectFetcher for FlakyFetcher {
        fn sync_object(&self, _object_key: &ObjectKey) -> Result<(), SyncError> {
            let attempt = self.attempts.fetch_add(1, Ordering::Relaxed) + 1;
            if attempt < self.fail_until {
                Err(SyncError::Download {
                    url: "https://example.test/missing.om".to_string(),
                    path: PathBuf::from("/tmp/missing.om"),
                    source: Box::new(HttpError::MissingFixture {
                        url: "https://example.test/missing.om".to_string(),
                    }),
                })
            } else {
                Ok(())
            }
        }

        fn synced_path(&self, object_key: &ObjectKey) -> PathBuf {
            PathBuf::from(&object_key.0)
        }
    }

    #[test]
    fn sync_object_with_retry_succeeds_after_transient_failures() {
        let fetcher = FlakyFetcher {
            attempts: AtomicUsize::new(0),
            fail_until: 3,
        };
        let key = ObjectKey("data_spatial/test.om".to_string());
        sync_object_with_retry(
            &fetcher,
            &key,
            WeatherModelId::EcmwfIfs,
            4,
            Duration::from_millis(1),
        )
        .expect("sync");
        assert_eq!(fetcher.attempts.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn sync_object_with_retry_returns_last_error_when_exhausted() {
        let fetcher = FlakyFetcher {
            attempts: AtomicUsize::new(0),
            fail_until: usize::MAX,
        };
        let key = ObjectKey("data_spatial/test.om".to_string());
        let error = sync_object_with_retry(
            &fetcher,
            &key,
            WeatherModelId::EcmwfIfs,
            3,
            Duration::from_millis(1),
        )
        .expect_err("sync");
        assert!(matches!(error, SyncError::Download { .. }));
        assert_eq!(fetcher.attempts.load(Ordering::Relaxed), 3);
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
