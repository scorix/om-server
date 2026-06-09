use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use tokio::sync::Notify;

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::application::weather_bake::{
    BakeTickResult, WeatherBakeConfig, WeatherBakeUseCase, load_weather_tile_coords,
};
use crate::error::WeatherBakeError;
use crate::infrastructure::tile_index::TileCoord;

pub struct WeatherBakeWorkerConfig {
    pub bake: WeatherBakeConfig,
    pub catalog: Arc<ActiveSpatialCatalog>,
    pub interval: Duration,
    pub wake: Option<Arc<Notify>>,
}

struct CachedTileCoords {
    sqlite_mtime: SystemTime,
    complete_only: bool,
    coords: Arc<Vec<TileCoord>>,
}

pub struct WeatherBakeWorker {
    bake: WeatherBakeConfig,
    catalog: Arc<ActiveSpatialCatalog>,
    interval: Duration,
    wake: Option<Arc<Notify>>,
    busy: Arc<AtomicBool>,
    tile_coords: Arc<Mutex<Option<CachedTileCoords>>>,
}

impl WeatherBakeWorker {
    pub fn new(config: WeatherBakeWorkerConfig) -> Self {
        Self {
            bake: config.bake,
            catalog: config.catalog,
            interval: config.interval,
            wake: config.wake,
            busy: Arc::new(AtomicBool::new(false)),
            tile_coords: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run_forever(self) {
        tracing::info!(
            interval_secs = self.interval.as_secs(),
            model = %self.bake.model,
            layer_count = self.bake.plans.len(),
            layers = ?self.bake.plans.iter().map(|plan| plan.layer.id()).collect::<Vec<_>>(),
            "weather bake worker started"
        );
        loop {
            while self.run_tick_if_idle().await == TickOutcome::Progress {
                tracing::debug!("weather bake worker continuing pending run");
            }
            self.wait_for_next_tick().await;
        }
    }

    async fn run_tick_if_idle(&self) -> TickOutcome {
        if self
            .busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return TickOutcome::Busy;
        }

        let worker = self.clone_for_blocking();
        let result = tokio::task::spawn_blocking(move || worker.tick()).await;
        self.busy.store(false, Ordering::Release);
        match result {
            Ok(Ok(tick)) => Self::log_tick(&tick),
            Ok(Err(error)) => {
                tracing::error!(error = %error, "weather bake worker tick failed");
                TickOutcome::Error
            }
            Err(error) => {
                tracing::error!(error = %error, "weather bake worker task failed");
                TickOutcome::Error
            }
        }
    }

    async fn wait_for_next_tick(&self) {
        let sleep = tokio::time::sleep(self.interval);
        tokio::pin!(sleep);
        match &self.wake {
            Some(wake) => {
                tokio::select! {
                    () = &mut sleep => {}
                    () = wake.notified() => {
                        tracing::debug!("weather bake worker woke after spatial sync");
                    }
                }
            }
            None => {
                sleep.await;
            }
        }
    }

    fn clone_for_blocking(&self) -> Self {
        Self {
            bake: self.bake.clone(),
            catalog: self.catalog.clone(),
            interval: self.interval,
            wake: self.wake.clone(),
            busy: self.busy.clone(),
            tile_coords: self.tile_coords.clone(),
        }
    }

    fn tick(&self) -> Result<BakeTickResult, WeatherBakeError> {
        let tile_coords = self.resolve_tile_coords()?;
        WeatherBakeUseCase.bake_tick(&self.bake, &self.catalog, tile_coords.as_ref())
    }

    fn resolve_tile_coords(&self) -> Result<Arc<Vec<TileCoord>>, WeatherBakeError> {
        let sqlite_mtime = std::fs::metadata(&self.bake.sqlite_path)
            .map_err(|source| WeatherBakeError::ReadFile {
                path: self.bake.sqlite_path.clone(),
                source,
            })?
            .modified()
            .map_err(|source| WeatherBakeError::ReadFile {
                path: self.bake.sqlite_path.clone(),
                source,
            })?;

        let mut cached = self
            .tile_coords
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(entry) = cached.as_ref()
            && entry.sqlite_mtime == sqlite_mtime
            && entry.complete_only == self.bake.complete_only
        {
            return Ok(entry.coords.clone());
        }

        let coords = Arc::new(load_weather_tile_coords(&self.bake)?);
        tracing::info!(
            tile_count = coords.len(),
            sqlite = %self.bake.sqlite_path.display(),
            "built weather tile index"
        );
        *cached = Some(CachedTileCoords {
            sqlite_mtime,
            complete_only: self.bake.complete_only,
            coords: coords.clone(),
        });
        Ok(coords)
    }

    fn log_tick(tick: &BakeTickResult) -> TickOutcome {
        match tick {
            BakeTickResult::Idle { reason } => {
                tracing::info!(reason, "weather bake worker idle");
                TickOutcome::Idle
            }
            BakeTickResult::Progress {
                variable,
                run_ref,
                valid_time,
                completed,
                total,
            } => {
                tracing::info!(
                    variable,
                    run_ref,
                    valid_time,
                    completed,
                    total,
                    "weather bake worker progress"
                );
                TickOutcome::Progress
            }
            BakeTickResult::RunComplete { variable, run_ref } => {
                tracing::info!(variable, run_ref, "weather bake worker run complete");
                TickOutcome::RunComplete
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TickOutcome {
    Progress,
    Idle,
    RunComplete,
    Error,
    Busy,
}
