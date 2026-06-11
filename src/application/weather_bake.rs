use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::application::active_catalog::ActiveSpatialCatalog;
use crate::domain::WeatherBakeLayer;
use crate::domain::WeatherModelId;
use crate::domain::spatial_snapshot::{SpatialObjectLocal, SpatialRunSnapshot};
use crate::domain::weather_field::{RegularLatLonField, SpatialFieldRegridder};
use crate::domain::weather_temporal::{bracket_objects, layer_valid_times_on_shared_timeline};
use crate::error::WeatherBakeError;
use crate::infrastructure::pmtiles_writer::{self, PmtilesMetadata, PmtilesTile};
use crate::infrastructure::tile_index::{self, TileCoord};
use crate::infrastructure::weather_bake_profile::WeatherBakeProfile;
use crate::infrastructure::weather_tile_renderer::{
    ScalarWeatherTileRenderer, WindWeatherTileRenderer,
};

pub const GLOBAL_MAX_ZOOM: u8 = 4;
pub const MIN_ZOOM: u8 = 0;
pub const MAX_ZOOM: u8 = GLOBAL_MAX_ZOOM;
pub const DEFAULT_VARIABLE: &str = "temperature_2m";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPmtilesArtifact {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub tile_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPmtilesManifest {
    pub model: String,
    pub variable: String,
    pub run_ref: String,
    pub default_valid_time: String,
    pub valid_times: Vec<String>,
    pub global_max_zoom: u8,
    pub regional_min_zoom: u8,
    pub regional_max_zoom: u8,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub generated_at: DateTime<Utc>,
    pub artifacts: BTreeMap<String, WeatherPmtilesArtifact>,
}

#[derive(Debug, Clone)]
pub struct WeatherBakePlan {
    pub layer: WeatherBakeLayer,
    /// Spatial model this layer is baked from (a variable may pin a different model).
    pub model: WeatherModelId,
    pub output_dir: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WeatherBakeConfig {
    pub cache_dir: Option<PathBuf>,
    /// Spatial model whose synced timesteps define the shared map timeline grid.
    pub timeline_model: WeatherModelId,
    pub plans: Vec<WeatherBakePlan>,
}

impl Default for WeatherBakeConfig {
    fn default() -> Self {
        let layer = WeatherBakeLayer::Temperature2m;
        Self {
            cache_dir: Some(PathBuf::from("data/cache/weather-pmtiles")),
            timeline_model: WeatherModelId::EcmwfIfs,
            plans: vec![WeatherBakePlan {
                layer,
                model: WeatherModelId::EcmwfIfs,
                output_dir: PathBuf::from("data/processed/weather/temperature_2m"),
                manifest_path: PathBuf::from(
                    "data/manifests/weather_pmtiles_temperature_2m_manifest.json",
                ),
            }],
        }
    }
}

pub struct WeatherBakeUseCase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BakeTickResult {
    Idle {
        reason: &'static str,
    },
    Progress {
        variable: String,
        model: String,
        run_ref: String,
        valid_time: String,
        completed: usize,
        total: usize,
    },
    RunComplete {
        run_ref: String,
        /// Each entry is `variable:model` (e.g. `snow_depth:ecmwf_ifs025`).
        layers: Vec<String>,
    },
}

impl WeatherBakeUseCase {
    /// Drops baked PMTiles and resets manifests when the catalog timeline no longer matches
    /// what is published (e.g. after bake-logic or `data_spatial` sync changes).
    pub fn prepare(
        &self,
        config: &WeatherBakeConfig,
        catalog: &ActiveSpatialCatalog,
    ) -> Result<(), WeatherBakeError> {
        if config.plans.is_empty() {
            return Ok(());
        }
        let Some((timeline_model, timeline_snapshot)) = shared_timeline(config, catalog) else {
            return Ok(());
        };
        let timeline_objects: Vec<_> = timeline_snapshot.objects.iter().collect();
        for plan in &config.plans {
            let Some(snapshot) = catalog.get(plan.model) else {
                continue;
            };
            let snapshot = snapshot.as_ref();
            if snapshot.objects.is_empty() {
                continue;
            }
            let layer_objects = layer_valid_times_on_timeline(
                &timeline_objects,
                snapshot,
                timeline_model,
                plan.model,
            )?;
            if layer_objects.is_empty() {
                continue;
            }
            let _ = resolve_manifest_work(config, plan, snapshot, &layer_objects)?;
        }
        Ok(())
    }

    /// Incremental bake step across all configured layers: at most one valid_time per call.
    pub fn bake_tick(
        &self,
        config: &WeatherBakeConfig,
        catalog: &ActiveSpatialCatalog,
        tile_coords: &[TileCoord],
    ) -> Result<BakeTickResult, WeatherBakeError> {
        if config.plans.is_empty() {
            return Ok(BakeTickResult::Idle {
                reason: "no bake layers configured",
            });
        }

        let Some((timeline_model, timeline_snapshot)) = shared_timeline(config, catalog) else {
            return Ok(BakeTickResult::Idle {
                reason: "shared timeline unavailable",
            });
        };
        let timeline_objects: Vec<_> = timeline_snapshot.objects.iter().collect();

        let mut any_snapshot = false;
        let mut representative_run_ref: Option<String> = None;
        for plan in &config.plans {
            let Some(snapshot) = catalog.get(plan.model) else {
                continue;
            };
            let snapshot = snapshot.as_ref();
            if snapshot.objects.is_empty() {
                continue;
            }
            let layer_objects = layer_valid_times_on_timeline(
                &timeline_objects,
                snapshot,
                timeline_model,
                plan.model,
            )?;
            if layer_objects.is_empty() {
                continue;
            }
            any_snapshot = true;
            representative_run_ref.get_or_insert_with(|| snapshot.run_ref.clone());

            if let tick @ BakeTickResult::Progress { .. } = self.bake_tick_plan(
                config,
                plan,
                snapshot,
                timeline_model,
                &layer_objects,
                tile_coords,
            )? {
                return Ok(tick);
            }
        }

        if !any_snapshot {
            return Ok(BakeTickResult::Idle {
                reason: "active spatial snapshot unavailable",
            });
        }

        if all_layers_complete(config, catalog)? {
            return Ok(BakeTickResult::RunComplete {
                run_ref: representative_run_ref.unwrap_or_default(),
                layers: config
                    .plans
                    .iter()
                    .map(|plan| format!("{}:{}", plan.layer.id(), plan.model.as_str()))
                    .collect(),
            });
        }

        Ok(BakeTickResult::Idle {
            reason: "all layers up to date",
        })
    }

    fn bake_tick_plan(
        &self,
        config: &WeatherBakeConfig,
        plan: &WeatherBakePlan,
        snapshot: &SpatialRunSnapshot,
        timeline_model: WeatherModelId,
        timeline_objects: &[&SpatialObjectLocal],
        tile_coords: &[TileCoord],
    ) -> Result<BakeTickResult, WeatherBakeError> {
        let variable = plan.layer.id().to_string();
        let mut work = resolve_manifest_work(config, plan, snapshot, timeline_objects)?;
        let pending = pending_timeline_times(timeline_objects, &work.manifest);
        if pending.is_empty() {
            if work.uses_staging {
                promote_staging_manifest(&plan.manifest_path)?;
            }
            gc_old_runs(
                &plan.output_dir,
                config.cache_dir.as_deref(),
                &snapshot.run_ref,
            )?;
            return Ok(BakeTickResult::Idle {
                reason: "layer up to date",
            });
        }

        let object = pending[0];
        let artifact = bake_timestep(
            plan,
            snapshot,
            timeline_model,
            &object.timestamp,
            tile_coords,
        )?;
        work.manifest
            .artifacts
            .insert(object.timestamp.clone(), artifact);
        if work.manifest.default_valid_time.is_empty() {
            work.manifest.default_valid_time = object.timestamp.clone();
        }
        write_manifest(&work.path, &work.manifest)?;

        if pending_timeline_times(timeline_objects, &work.manifest).is_empty() && work.uses_staging
        {
            promote_staging_manifest(&plan.manifest_path)?;
            gc_old_runs(
                &plan.output_dir,
                config.cache_dir.as_deref(),
                &snapshot.run_ref,
            )?;
        }

        let completed = work
            .manifest
            .artifacts
            .values()
            .filter(|artifact| artifact_is_complete(artifact))
            .count();
        Ok(BakeTickResult::Progress {
            variable,
            model: plan.model.as_str().to_string(),
            run_ref: snapshot.run_ref.clone(),
            valid_time: object.timestamp.clone(),
            completed,
            total: timeline_objects.len(),
        })
    }
}

/// Weather tiles are global-only (zoom 0..=GLOBAL_MAX_ZOOM). om-server intentionally does not
/// depend on ski-resort regional coverage, so the index needs no external sqlite/catalog input.
pub fn weather_tile_coords() -> Vec<TileCoord> {
    tile_index::global_tile_coords(MIN_ZOOM, GLOBAL_MAX_ZOOM)
}

pub fn build_bake_plans(
    output_dir: PathBuf,
    manifest_dir: PathBuf,
    profile: &WeatherBakeProfile,
) -> Vec<WeatherBakePlan> {
    profile
        .layers
        .iter()
        .map(|spec| WeatherBakePlan {
            layer: spec.layer,
            model: spec.model,
            output_dir: output_dir.join(spec.layer.id()),
            manifest_path: manifest_dir
                .join(format!("weather_pmtiles_{}_manifest.json", spec.layer.id())),
        })
        .collect()
}

fn shared_timeline(
    config: &WeatherBakeConfig,
    catalog: &ActiveSpatialCatalog,
) -> Option<(WeatherModelId, Arc<SpatialRunSnapshot>)> {
    let snapshot = catalog.get(config.timeline_model)?;
    if snapshot.objects.is_empty() {
        return None;
    }
    Some((config.timeline_model, snapshot))
}

fn all_layers_complete(
    config: &WeatherBakeConfig,
    catalog: &ActiveSpatialCatalog,
) -> Result<bool, WeatherBakeError> {
    let Some((timeline_model, timeline_snapshot)) = shared_timeline(config, catalog) else {
        return Ok(false);
    };
    let timeline_objects: Vec<_> = timeline_snapshot.objects.iter().collect();
    for plan in &config.plans {
        let Some(snapshot) = catalog.get(plan.model) else {
            return Ok(false);
        };
        let snapshot = snapshot.as_ref();
        if snapshot.objects.is_empty() {
            continue;
        }
        let layer_objects =
            layer_valid_times_on_timeline(&timeline_objects, snapshot, timeline_model, plan.model)?;
        let work = resolve_manifest_work(config, plan, snapshot, &layer_objects)?;
        if !pending_timeline_times(&layer_objects, &work.manifest).is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn layer_valid_times_on_timeline<'a>(
    timeline_objects: &[&'a SpatialObjectLocal],
    layer_snapshot: &'a SpatialRunSnapshot,
    timeline_model: WeatherModelId,
    layer_model: WeatherModelId,
) -> Result<Vec<&'a SpatialObjectLocal>, WeatherBakeError> {
    layer_valid_times_on_shared_timeline(
        timeline_objects,
        &layer_snapshot.objects,
        timeline_model == layer_model,
    )
    .map_err(WeatherBakeError::Timestamp)
}

fn pending_timeline_times<'a>(
    timeline_objects: &[&'a SpatialObjectLocal],
    manifest: &WeatherPmtilesManifest,
) -> Vec<&'a SpatialObjectLocal> {
    let completed: HashSet<_> = manifest
        .artifacts
        .iter()
        .filter(|(_, artifact)| artifact_is_complete(artifact))
        .map(|(valid_time, _)| valid_time.as_str())
        .collect();
    timeline_objects
        .iter()
        .copied()
        .filter(|object| !completed.contains(object.timestamp.as_str()))
        .collect()
}

fn artifact_is_complete(artifact: &WeatherPmtilesArtifact) -> bool {
    !artifact.path.is_empty()
        && artifact.sha256.len() == 64
        && artifact.size_bytes > 0
        && Path::new(&artifact.path).exists()
}

struct ManifestWork {
    path: PathBuf,
    manifest: WeatherPmtilesManifest,
    uses_staging: bool,
}

/// Resolves where incremental bake progress should be written.
///
/// When a new `run_ref` is baking while an older published manifest is still
/// served, progress is written to a staging file and only swapped into the
/// published manifest once every timestep for the new run is complete.
fn resolve_manifest_work(
    config: &WeatherBakeConfig,
    plan: &WeatherBakePlan,
    snapshot: &SpatialRunSnapshot,
    timeline_objects: &[&SpatialObjectLocal],
) -> Result<ManifestWork, WeatherBakeError> {
    let published_path = &plan.manifest_path;
    let published = read_manifest(published_path)?;
    let has_published = published.is_some();
    let expected_times = expected_valid_times(timeline_objects);

    if let Some(existing) = published.as_ref()
        && existing.run_ref == snapshot.run_ref
    {
        if manifest_matches_plan(existing, plan, &expected_times) {
            return Ok(ManifestWork {
                path: published_path.clone(),
                manifest: existing.clone(),
                uses_staging: false,
            });
        }
        tracing::info!(
            variable = plan.layer.id(),
            model = plan.model.as_str(),
            run_ref = snapshot.run_ref,
            "weather bake manifest stale; purging baked artifacts"
        );
        purge_layer_bake(config, plan, existing)?;
        let fresh = init_manifest(config, plan, snapshot, timeline_objects);
        write_manifest(published_path, &fresh)?;
        return Ok(ManifestWork {
            path: published_path.clone(),
            manifest: fresh,
            uses_staging: false,
        });
    }

    let staging_path = staging_manifest_path(published_path);
    if let Some(staging) = read_manifest(&staging_path)?
        && staging.run_ref == snapshot.run_ref
    {
        if manifest_matches_plan(&staging, plan, &expected_times) {
            return Ok(ManifestWork {
                path: staging_path,
                manifest: staging,
                uses_staging: true,
            });
        }
        tracing::info!(
            variable = plan.layer.id(),
            model = plan.model.as_str(),
            run_ref = snapshot.run_ref,
            "weather bake staging manifest stale; purging in-progress bake"
        );
        purge_layer_bake(config, plan, &staging)?;
        std::fs::remove_file(&staging_path).map_err(|source| WeatherBakeError::WriteFile {
            path: staging_path.clone(),
            source,
        })?;
    } else if staging_path.exists() {
        std::fs::remove_file(&staging_path).map_err(|source| WeatherBakeError::WriteFile {
            path: staging_path.clone(),
            source,
        })?;
    }

    let uses_staging = has_published;
    gc_old_runs(
        &plan.output_dir,
        config.cache_dir.as_deref(),
        &snapshot.run_ref,
    )?;
    let fresh = init_manifest(config, plan, snapshot, timeline_objects);
    Ok(ManifestWork {
        path: if uses_staging {
            staging_path
        } else {
            published_path.clone()
        },
        manifest: fresh,
        uses_staging,
    })
}

fn expected_valid_times(timeline_objects: &[&SpatialObjectLocal]) -> Vec<String> {
    timeline_objects
        .iter()
        .map(|object| object.timestamp.clone())
        .collect()
}

fn manifest_matches_plan(
    manifest: &WeatherPmtilesManifest,
    plan: &WeatherBakePlan,
    expected_times: &[String],
) -> bool {
    manifest.variable == plan.layer.id()
        && manifest.model == plan.model.to_string()
        && manifest.valid_times == expected_times
}

fn purge_layer_bake(
    config: &WeatherBakeConfig,
    plan: &WeatherBakePlan,
    manifest: &WeatherPmtilesManifest,
) -> Result<(), WeatherBakeError> {
    purge_run_dir(&plan.output_dir, &manifest.run_ref)?;
    if let Some(cache_dir) = config.cache_dir.as_deref() {
        purge_run_dir(cache_dir, &manifest.run_ref)?;
    }
    Ok(())
}

fn purge_run_dir(base: &Path, run_ref: &str) -> Result<(), WeatherBakeError> {
    let run_dir = base.join(run_ref);
    if !run_dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(&run_dir).map_err(|source| WeatherBakeError::WriteFile {
        path: run_dir,
        source,
    })
}

fn init_manifest(
    _config: &WeatherBakeConfig,
    plan: &WeatherBakePlan,
    snapshot: &SpatialRunSnapshot,
    timeline_objects: &[&SpatialObjectLocal],
) -> WeatherPmtilesManifest {
    WeatherPmtilesManifest {
        model: plan.model.to_string(),
        variable: plan.layer.id().to_string(),
        run_ref: snapshot.run_ref.clone(),
        default_valid_time: timeline_objects
            .first()
            .map(|object| object.timestamp.clone())
            .unwrap_or_default(),
        valid_times: timeline_objects
            .iter()
            .map(|object| object.timestamp.clone())
            .collect(),
        global_max_zoom: GLOBAL_MAX_ZOOM,
        // Regional (resort) tiles were removed; keep the fields for manifest schema compatibility
        // but pin them to the global max so consumers see no zoom beyond GLOBAL_MAX_ZOOM.
        regional_min_zoom: GLOBAL_MAX_ZOOM,
        regional_max_zoom: GLOBAL_MAX_ZOOM,
        min_zoom: MIN_ZOOM,
        max_zoom: MAX_ZOOM,
        generated_at: Utc::now(),
        artifacts: BTreeMap::new(),
    }
}

fn staging_manifest_path(published_path: &Path) -> PathBuf {
    published_path.with_extension("staging.json")
}

fn promote_staging_manifest(published_path: &Path) -> Result<(), WeatherBakeError> {
    let staging_path = staging_manifest_path(published_path);
    let staging = match read_manifest(&staging_path)? {
        Some(staging) => staging,
        None => return Ok(()),
    };
    write_manifest(published_path, &staging)?;
    std::fs::remove_file(&staging_path).map_err(|source| WeatherBakeError::WriteFile {
        path: staging_path,
        source,
    })
}

fn read_manifest(path: &Path) -> Result<Option<WeatherPmtilesManifest>, WeatherBakeError> {
    match std::fs::read(path) {
        Ok(bytes) => {
            Ok(Some(serde_json::from_slice(&bytes).map_err(|source| {
                WeatherBakeError::Serialize { source }
            })?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(WeatherBakeError::ReadFile {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Debug, Clone)]
enum FieldPathSource<'a> {
    Direct(&'a Path),
    Interpolated {
        before: &'a Path,
        after: &'a Path,
        fraction: f64,
    },
}

fn field_path_source<'a>(
    layer_snapshot: &'a SpatialRunSnapshot,
    timeline_model: WeatherModelId,
    layer_model: WeatherModelId,
    valid_time: &str,
) -> Result<FieldPathSource<'a>, WeatherBakeError> {
    if layer_model == timeline_model {
        let Some(object) = layer_snapshot
            .objects
            .iter()
            .find(|object| object.timestamp == valid_time)
        else {
            return Err(WeatherBakeError::MissingNativeTimestep {
                valid_time: valid_time.to_string(),
            });
        };
        return Ok(FieldPathSource::Direct(&object.local_path));
    }

    let bracket = bracket_objects(&layer_snapshot.objects, valid_time)
        .map_err(WeatherBakeError::Timestamp)?;
    match (bracket.before, bracket.after) {
        (None, None) => Err(WeatherBakeError::MissingNativeTimestep {
            valid_time: valid_time.to_string(),
        }),
        (Some(object), None) | (None, Some(object)) => {
            Ok(FieldPathSource::Direct(&object.local_path))
        }
        (Some(before), Some(after)) if before.timestamp == after.timestamp => {
            Ok(FieldPathSource::Direct(&before.local_path))
        }
        (Some(before), Some(after)) => Ok(FieldPathSource::Interpolated {
            before: &before.local_path,
            after: &after.local_path,
            fraction: bracket.fraction,
        }),
    }
}

fn load_scalar_field(
    source: &FieldPathSource<'_>,
    variable: &str,
) -> Result<RegularLatLonField, WeatherBakeError> {
    match source {
        FieldPathSource::Direct(path) => Ok(SpatialFieldRegridder::from_spatial_file_or_empty(
            path, variable,
        )?),
        FieldPathSource::Interpolated {
            before,
            after,
            fraction,
        } => {
            let left = SpatialFieldRegridder::from_spatial_file_or_empty(before, variable)?;
            let right = SpatialFieldRegridder::from_spatial_file_or_empty(after, variable)?;
            Ok(left.lerp(&right, *fraction))
        }
    }
}

fn bake_timestep(
    plan: &WeatherBakePlan,
    layer_snapshot: &SpatialRunSnapshot,
    timeline_model: WeatherModelId,
    valid_time: &str,
    tile_coords: &[TileCoord],
) -> Result<WeatherPmtilesArtifact, WeatherBakeError> {
    let layer = plan.layer;
    let source = field_path_source(layer_snapshot, timeline_model, plan.model, valid_time)?;
    let tiles: Vec<PmtilesTile> = match layer {
        WeatherBakeLayer::Wind => {
            let (u_name, v_name) = layer.wind_spatial_variables().expect("wind variables");
            let u_field = Arc::new(load_scalar_field(&source, u_name)?);
            let v_field = Arc::new(load_scalar_field(&source, v_name)?);
            let renderer = WindWeatherTileRenderer::new(u_field.as_ref(), v_field.as_ref());
            tile_coords
                .par_iter()
                .map(|&(z, x, y)| {
                    let data = renderer
                        .render_tile_png(z, x, y)
                        .map_err(WeatherBakeError::TileRender)?;
                    Ok(PmtilesTile { z, x, y, data })
                })
                .collect::<Result<Vec<_>, WeatherBakeError>>()?
        }
        _ => {
            let variable = layer.spatial_variable().expect("scalar layer variable");
            let field = Arc::new(load_scalar_field(&source, variable)?);
            let renderer = ScalarWeatherTileRenderer::new(layer, field.as_ref());
            tile_coords
                .par_iter()
                .map(|&(z, x, y)| {
                    let data = renderer
                        .render_tile_png(z, x, y)
                        .map_err(WeatherBakeError::TileRender)?;
                    Ok(PmtilesTile { z, x, y, data })
                })
                .collect::<Result<Vec<_>, WeatherBakeError>>()?
        }
    };

    let output_path = pmtiles_output_path(&plan.output_dir, &layer_snapshot.run_ref, valid_time);
    let metadata = PmtilesMetadata {
        min_zoom: MIN_ZOOM,
        max_zoom: MAX_ZOOM,
        bounds: Some((-180.0, -85.051_128_78, 180.0, 85.051_128_78)),
        json: serde_json::json!({
            "model": plan.model.to_string(),
            "variable": layer.id(),
            "run_ref": layer_snapshot.run_ref,
            "valid_time": valid_time,
            "global_max_zoom": GLOBAL_MAX_ZOOM,
            "regional_min_zoom": GLOBAL_MAX_ZOOM,
            "regional_max_zoom": GLOBAL_MAX_ZOOM,
        })
        .to_string(),
    };
    pmtiles_writer::write_png_pmtiles(&output_path, &metadata, &tiles)?;

    let size_bytes = std::fs::metadata(&output_path)
        .map_err(|source| WeatherBakeError::ReadFile {
            path: output_path.clone(),
            source,
        })?
        .len();
    let sha256 = pmtiles_writer::sha256_file(&output_path)?;

    Ok(WeatherPmtilesArtifact {
        path: output_path.display().to_string(),
        sha256,
        size_bytes,
        tile_count: tiles.len(),
    })
}

fn pmtiles_output_path(output_dir: &Path, run_ref: &str, valid_time: &str) -> PathBuf {
    output_dir
        .join(run_ref)
        .join(format!("{valid_time}.pmtiles"))
}

fn write_manifest(path: &Path, manifest: &WeatherPmtilesManifest) -> Result<(), WeatherBakeError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| WeatherBakeError::WriteFile {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|source| WeatherBakeError::Serialize { source })?;
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, json).map_err(|source| WeatherBakeError::WriteFile {
        path: temp_path.clone(),
        source,
    })?;
    std::fs::rename(&temp_path, path).map_err(|source| WeatherBakeError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn gc_old_runs(
    output_dir: &Path,
    cache_dir: Option<&Path>,
    current_run_ref: &str,
) -> Result<(), WeatherBakeError> {
    gc_run_dirs(output_dir, current_run_ref)?;
    if let Some(cache_dir) = cache_dir {
        gc_run_dirs(cache_dir, current_run_ref)?;
    }
    Ok(())
}

fn gc_run_dirs(base: &Path, current_run_ref: &str) -> Result<(), WeatherBakeError> {
    if !base.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(base).map_err(|source| WeatherBakeError::ReadFile {
        path: base.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| WeatherBakeError::ReadFile {
            path: base.to_path_buf(),
            source,
        })?;
        if !entry
            .file_type()
            .map_err(|source| WeatherBakeError::ReadFile {
                path: entry.path(),
                source,
            })?
            .is_dir()
        {
            continue;
        }
        let name = entry.file_name();
        if name == current_run_ref {
            continue;
        }
        std::fs::remove_dir_all(entry.path()).map_err(|source| WeatherBakeError::WriteFile {
            path: entry.path(),
            source,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        WeatherBakeConfig, WeatherBakeLayer, WeatherBakePlan, WeatherPmtilesArtifact,
        WeatherPmtilesManifest, all_layers_complete, artifact_is_complete, build_bake_plans,
        pending_timeline_times, promote_staging_manifest, read_manifest, resolve_manifest_work,
        staging_manifest_path, write_manifest,
    };
    use crate::application::active_catalog::ActiveSpatialCatalog;
    use crate::domain::WeatherModelId;
    use crate::domain::spatial_snapshot::{SpatialObjectLocal, SpatialRunSnapshot};
    use chrono::Utc;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn pending_timeline_times_preserves_snapshot_order() {
        let objects = [
            SpatialObjectLocal {
                object_key: "a".to_string(),
                timestamp: "2025-06-09T12:00:00Z".to_string(),
                valid_date: "2025-06-09".to_string(),
                local_path: PathBuf::from("a.om"),
            },
            SpatialObjectLocal {
                object_key: "b".to_string(),
                timestamp: "2025-06-09T15:00:00Z".to_string(),
                valid_date: "2025-06-09".to_string(),
                local_path: PathBuf::from("b.om"),
            },
        ];
        let object_refs: Vec<_> = objects.iter().collect();
        let manifest = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "2025060912".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: objects
                .iter()
                .map(|object| object.timestamp.clone())
                .collect(),
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::new(),
        };

        let pending = pending_timeline_times(&object_refs, &manifest);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].timestamp, "2025-06-09T12:00:00Z");
    }

    #[test]
    fn artifact_is_complete_requires_existing_pmtiles_path() {
        assert!(!artifact_is_complete(&WeatherPmtilesArtifact {
            path: String::new(),
            sha256: "abc".to_string(),
            size_bytes: 1,
            tile_count: 1,
        }));
    }

    fn sample_objects() -> Vec<SpatialObjectLocal> {
        vec![
            SpatialObjectLocal {
                object_key: "a".to_string(),
                timestamp: "2025-06-09T12:00:00Z".to_string(),
                valid_date: "2025-06-09".to_string(),
                local_path: PathBuf::from("a.om"),
            },
            SpatialObjectLocal {
                object_key: "b".to_string(),
                timestamp: "2025-06-09T18:00:00Z".to_string(),
                valid_date: "2025-06-09".to_string(),
                local_path: PathBuf::from("b.om"),
            },
        ]
    }

    fn sample_snapshot(
        model: WeatherModelId,
        run_ref: &str,
        objects: &[SpatialObjectLocal],
    ) -> SpatialRunSnapshot {
        SpatialRunSnapshot {
            model,
            reference_time: "2025-06-09T12:00:00Z".to_string(),
            run_ref: run_ref.to_string(),
            objects: objects.to_vec(),
        }
    }

    #[test]
    fn all_layers_complete_false_when_layer_model_catalog_missing() {
        let sync_dir =
            std::env::temp_dir().join(format!("om-all-layers-complete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&sync_dir);
        std::fs::create_dir_all(&sync_dir).expect("temp sync dir");

        let catalog = ActiveSpatialCatalog::new();
        catalog
            .publish(
                &sync_dir,
                Arc::new(sample_snapshot(
                    WeatherModelId::EcmwfIfs,
                    "0600Z",
                    &sample_objects(),
                )),
            )
            .expect("publish ifs snapshot");

        let config = WeatherBakeConfig {
            cache_dir: None,
            timeline_model: WeatherModelId::EcmwfIfs,
            plans: vec![
                WeatherBakePlan {
                    layer: WeatherBakeLayer::Temperature2m,
                    model: WeatherModelId::EcmwfIfs,
                    output_dir: sync_dir.join("temperature_2m"),
                    manifest_path: sync_dir.join("temperature_2m.json"),
                },
                WeatherBakePlan {
                    layer: WeatherBakeLayer::SnowDepth,
                    model: WeatherModelId::EcmwfIfs025,
                    output_dir: sync_dir.join("snow_depth"),
                    manifest_path: sync_dir.join("snow_depth.json"),
                },
            ],
        };

        assert!(
            !all_layers_complete(&config, &catalog).expect("check layers"),
            "snow_depth requires ecmwf_ifs025 catalog"
        );

        let _ = std::fs::remove_dir_all(&sync_dir);
    }

    #[test]
    fn resolve_manifest_work_reuses_published_manifest_for_same_run() {
        let dir = std::env::temp_dir().join(format!(
            "snowbuddy-weather-manifest-same-run-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let manifest_path = dir.join("weather_pmtiles_temperature_2m_manifest.json");
        let objects = sample_objects();
        let object_refs: Vec<_> = objects.iter().collect();
        let published = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "2025060912".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: objects
                .iter()
                .map(|object| object.timestamp.clone())
                .collect(),
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::new(),
        };
        write_manifest(&manifest_path, &published).expect("write published manifest");

        let config = WeatherBakeConfig::default();
        let plan = WeatherBakePlan {
            layer: WeatherBakeLayer::Temperature2m,
            model: WeatherModelId::EcmwfIfs,
            output_dir: dir.join("temperature_2m"),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot(WeatherModelId::EcmwfIfs, "2025060912", &objects);
        let work = resolve_manifest_work(&config, &plan, &snapshot, &object_refs)
            .expect("resolve manifest work");

        assert!(!work.uses_staging);
        assert_eq!(work.path, manifest_path);
        assert_eq!(work.manifest.run_ref, "2025060912");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_manifest_work_stages_new_run_without_touching_published_manifest() {
        let dir = std::env::temp_dir().join(format!(
            "snowbuddy-weather-manifest-staging-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let manifest_path = dir.join("weather_pmtiles_temperature_2m_manifest.json");
        let objects = sample_objects();
        let object_refs: Vec<_> = objects.iter().collect();
        let published = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "2025060912".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: vec!["2025-06-09T12:00:00Z".to_string()],
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::from([(
                "2025-06-09T12:00:00Z".to_string(),
                WeatherPmtilesArtifact {
                    path: dir.join("old.pmtiles").display().to_string(),
                    sha256: "a".repeat(64),
                    size_bytes: 7,
                    tile_count: 1,
                },
            )]),
        };
        write_manifest(&manifest_path, &published).expect("write published manifest");

        let config = WeatherBakeConfig::default();
        let plan = WeatherBakePlan {
            layer: WeatherBakeLayer::Temperature2m,
            model: WeatherModelId::EcmwfIfs,
            output_dir: dir.join("temperature_2m"),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot(WeatherModelId::EcmwfIfs, "2025060918", &objects);
        let work = resolve_manifest_work(&config, &plan, &snapshot, &object_refs)
            .expect("resolve manifest work");

        assert!(work.uses_staging);
        assert_eq!(work.path, staging_manifest_path(&manifest_path));
        assert_eq!(work.manifest.run_ref, "2025060918");
        assert_eq!(
            read_manifest(&manifest_path)
                .expect("read published")
                .expect("published exists")
                .run_ref,
            "2025060912"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn promote_staging_manifest_swaps_only_after_new_run_is_ready() {
        let dir = std::env::temp_dir().join(format!(
            "snowbuddy-weather-manifest-promote-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let manifest_path = dir.join("weather_pmtiles_temperature_2m_manifest.json");
        let published = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "2025060912".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: vec!["2025-06-09T12:00:00Z".to_string()],
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::new(),
        };
        write_manifest(&manifest_path, &published).expect("write published manifest");

        let staging = WeatherPmtilesManifest {
            run_ref: "2025060918".to_string(),
            default_valid_time: "2025-06-09T18:00:00Z".to_string(),
            valid_times: vec![
                "2025-06-09T18:00:00Z".to_string(),
                "2025-06-10T00:00:00Z".to_string(),
            ],
            artifacts: BTreeMap::from([(
                "2025-06-09T18:00:00Z".to_string(),
                WeatherPmtilesArtifact {
                    path: dir.join("new.pmtiles").display().to_string(),
                    sha256: "b".repeat(64),
                    size_bytes: 9,
                    tile_count: 1,
                },
            )]),
            ..published.clone()
        };
        write_manifest(&staging_manifest_path(&manifest_path), &staging)
            .expect("write staging manifest");

        promote_staging_manifest(&manifest_path).expect("promote staging manifest");

        let swapped = read_manifest(&manifest_path)
            .expect("read published")
            .expect("published exists");
        assert_eq!(swapped.run_ref, "2025060918");
        assert_eq!(
            swapped
                .artifacts
                .get("2025-06-09T18:00:00Z")
                .map(|artifact| artifact.path.as_str()),
            Some(dir.join("new.pmtiles").display().to_string().as_str())
        );
        assert!(!staging_manifest_path(&manifest_path).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_manifest_work_purges_stale_published_bake_for_same_run_ref() {
        let dir = std::env::temp_dir().join(format!(
            "snowbuddy-weather-manifest-stale-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let output_dir = dir.join("temperature_2m");
        let run_dir = output_dir.join("0600Z");
        std::fs::create_dir_all(&run_dir).expect("create run dir");
        let stale_tile = run_dir.join("2025-06-09T12:00:00Z.pmtiles");
        std::fs::write(&stale_tile, b"stale").expect("write stale tile");

        let objects = sample_objects();
        let object_refs: Vec<_> = objects.iter().collect();
        let manifest_path = dir.join("weather_pmtiles_temperature_2m_manifest.json");
        let published = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "0600Z".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: vec!["2025-06-09T12:00:00Z".to_string()],
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::from([(
                "2025-06-09T12:00:00Z".to_string(),
                WeatherPmtilesArtifact {
                    path: stale_tile.display().to_string(),
                    sha256: "a".repeat(64),
                    size_bytes: 5,
                    tile_count: 1,
                },
            )]),
        };
        write_manifest(&manifest_path, &published).expect("write published manifest");

        let config = WeatherBakeConfig::default();
        let plan = WeatherBakePlan {
            layer: WeatherBakeLayer::Temperature2m,
            model: WeatherModelId::EcmwfIfs,
            output_dir: output_dir.clone(),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot(WeatherModelId::EcmwfIfs, "0600Z", &objects);
        let work = resolve_manifest_work(&config, &plan, &snapshot, &object_refs)
            .expect("resolve manifest work");

        assert!(!work.uses_staging);
        assert_eq!(work.manifest.valid_times.len(), 2);
        assert!(work.manifest.artifacts.is_empty());
        assert!(!stale_tile.exists());
        let on_disk = read_manifest(&manifest_path)
            .expect("read published")
            .expect("published exists");
        assert_eq!(on_disk.valid_times, work.manifest.valid_times);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_manifest_work_gc_old_run_dirs_when_run_ref_changes() {
        let dir = std::env::temp_dir().join(format!(
            "snowbuddy-weather-manifest-gc-run-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let output_dir = dir.join("temperature_2m");
        let old_run_dir = output_dir.join("0000Z");
        std::fs::create_dir_all(&old_run_dir).expect("create old run dir");
        std::fs::write(old_run_dir.join("old.pmtiles"), b"old").expect("write old tile");

        let objects = sample_objects();
        let object_refs: Vec<_> = objects.iter().collect();
        let manifest_path = dir.join("weather_pmtiles_temperature_2m_manifest.json");
        let published = WeatherPmtilesManifest {
            model: "ecmwf_ifs".to_string(),
            variable: "temperature_2m".to_string(),
            run_ref: "0000Z".to_string(),
            default_valid_time: "2025-06-09T12:00:00Z".to_string(),
            valid_times: vec!["2025-06-09T12:00:00Z".to_string()],
            global_max_zoom: 4,
            regional_min_zoom: 5,
            regional_max_zoom: 5,
            min_zoom: 0,
            max_zoom: 5,
            generated_at: Utc::now(),
            artifacts: BTreeMap::new(),
        };
        write_manifest(&manifest_path, &published).expect("write published manifest");

        let config = WeatherBakeConfig::default();
        let plan = WeatherBakePlan {
            layer: WeatherBakeLayer::Temperature2m,
            model: WeatherModelId::EcmwfIfs,
            output_dir: output_dir.clone(),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot(WeatherModelId::EcmwfIfs, "0600Z", &objects);
        let work = resolve_manifest_work(&config, &plan, &snapshot, &object_refs)
            .expect("resolve manifest work");

        assert!(work.uses_staging);
        assert!(!old_run_dir.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_bake_plans_uses_per_variable_output_and_manifest_paths() {
        use crate::infrastructure::weather_bake_profile::load_weather_bake_profile;

        let profile =
            load_weather_bake_profile(PathBuf::from("config/weather_bake.toml").as_path())
                .expect("default profile");
        let plans = build_bake_plans(
            PathBuf::from("data/processed/weather"),
            PathBuf::from("data/manifests"),
            &profile,
        );
        let temperature = plans
            .iter()
            .find(|plan| plan.layer == WeatherBakeLayer::Temperature2m)
            .expect("temperature plan");
        assert_eq!(
            temperature.manifest_path,
            PathBuf::from("data/manifests/weather_pmtiles_temperature_2m_manifest.json")
        );
        assert_eq!(
            temperature.output_dir,
            PathBuf::from("data/processed/weather/temperature_2m")
        );
        let cloud = plans
            .iter()
            .find(|plan| plan.layer == WeatherBakeLayer::CloudCover)
            .expect("cloud plan");
        assert_eq!(
            cloud.manifest_path,
            PathBuf::from("data/manifests/weather_pmtiles_cloud_cover_manifest.json")
        );
        assert_eq!(plans.len(), 7);
        assert!(
            plans
                .iter()
                .any(|plan| plan.layer == WeatherBakeLayer::SnowDepth
                    && plan.model == WeatherModelId::EcmwfIfs025)
        );
        assert!(
            plans
                .iter()
                .any(|plan| plan.layer == WeatherBakeLayer::Visibility
                    && plan.model == WeatherModelId::EcmwfIfs)
        );
    }

    #[test]
    fn build_bake_plans_honors_profile_model_overrides() {
        use crate::infrastructure::weather_bake_profile::{
            WeatherBakeLayerSpec, WeatherBakeProfile,
        };

        let profile = WeatherBakeProfile {
            timeline_model: WeatherModelId::EcmwfIfs,
            layers: vec![
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::Temperature2m,
                    model: WeatherModelId::EcmwfIfs,
                },
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::SnowDepth,
                    model: WeatherModelId::EcmwfIfs025,
                },
                WeatherBakeLayerSpec {
                    layer: WeatherBakeLayer::Visibility,
                    model: WeatherModelId::EcmwfIfs,
                },
            ],
        };
        let plans = build_bake_plans(
            PathBuf::from("data/processed/weather"),
            PathBuf::from("data/manifests"),
            &profile,
        );
        let temperature = plans
            .iter()
            .find(|plan| plan.layer == WeatherBakeLayer::Temperature2m)
            .expect("temperature plan");
        assert_eq!(temperature.model, WeatherModelId::EcmwfIfs);
        let snow_depth = plans
            .iter()
            .find(|plan| plan.layer == WeatherBakeLayer::SnowDepth)
            .expect("snow_depth plan");
        assert_eq!(snow_depth.model, WeatherModelId::EcmwfIfs025);
        let visibility = plans
            .iter()
            .find(|plan| plan.layer == WeatherBakeLayer::Visibility)
            .expect("visibility plan");
        assert_eq!(visibility.model, WeatherModelId::EcmwfIfs);
    }
}
