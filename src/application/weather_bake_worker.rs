use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Notify;

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::application::weather_bake::{
    BakeTickResult, WeatherBakeConfig, WeatherBakeUseCase, weather_tile_coords,
};
use crate::error::WeatherBakeError;
use crate::infrastructure::tile_index::TileCoord;

pub struct WeatherBakeWorkerConfig {
    pub bake: WeatherBakeConfig,
    pub catalog: Arc<ActiveSpatialCatalog>,
    pub interval: Duration,
    pub wake: Option<Arc<Notify>>,
}

pub struct WeatherBakeWorker {
    bake: WeatherBakeConfig,
    catalog: Arc<ActiveSpatialCatalog>,
    interval: Duration,
    wake: Option<Arc<Notify>>,
    busy: Arc<AtomicBool>,
    tile_coords: Arc<Mutex<Option<Arc<Vec<TileCoord>>>>>,
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
            layer_count = self.bake.plans.len(),
            layers = ?self
                .bake
                .plans
                .iter()
                .map(|plan| format!("{}:{}", plan.layer.id(), plan.model.as_str()))
                .collect::<Vec<_>>(),
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
        let mut cached = self
            .tile_coords
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(coords) = cached.as_ref() {
            return Ok(coords.clone());
        }

        let coords = Arc::new(weather_tile_coords());
        tracing::info!(
            tile_count = coords.len(),
            "built weather tile index (global-only)"
        );
        *cached = Some(coords.clone());
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
                model,
                run_ref,
                valid_time,
                completed,
                total,
            } => {
                tracing::info!(
                    variable,
                    model,
                    run_ref,
                    valid_time,
                    completed,
                    total,
                    "weather bake worker progress"
                );
                TickOutcome::Progress
            }
            BakeTickResult::RunComplete { layers, run_ref } => {
                tracing::info!(
                    run_ref,
                    layers = ?layers,
                    "weather bake worker all layers complete"
                );
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
