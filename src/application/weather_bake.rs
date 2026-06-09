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
use crate::domain::weather_field::SpatialFieldRegridder;
use crate::error::WeatherBakeError;
use crate::infrastructure::pmtiles_writer::{self, PmtilesMetadata, PmtilesTile};
use crate::infrastructure::resort_coverage;
use crate::infrastructure::tile_index::{self, TileCoord};
use crate::infrastructure::weather_tile_renderer::{
    ScalarWeatherTileRenderer, WindWeatherTileRenderer,
};

pub const GLOBAL_MAX_ZOOM: u8 = 4;
pub const REGIONAL_MIN_ZOOM: u8 = 5;
pub const REGIONAL_MAX_ZOOM: u8 = 5;
pub const MIN_ZOOM: u8 = 0;
pub const MAX_ZOOM: u8 = REGIONAL_MAX_ZOOM;
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
    pub output_dir: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WeatherBakeConfig {
    pub sqlite_path: PathBuf,
    pub cache_dir: Option<PathBuf>,
    pub model: WeatherModelId,
    pub complete_only: bool,
    pub plans: Vec<WeatherBakePlan>,
}

impl WeatherBakeConfig {
    pub fn primary_plan(&self) -> Option<&WeatherBakePlan> {
        self.plans.first()
    }
}

impl Default for WeatherBakeConfig {
    fn default() -> Self {
        let layer = WeatherBakeLayer::Temperature2m;
        Self {
            sqlite_path: PathBuf::from("data/processed/opensnowmap/snowbuddy_osm.sqlite"),
            cache_dir: Some(PathBuf::from("data/cache/weather-pmtiles")),
            model: WeatherModelId::EcmwfIfs,
            complete_only: true,
            plans: vec![WeatherBakePlan {
                layer,
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
        run_ref: String,
        valid_time: String,
        completed: usize,
        total: usize,
    },
    RunComplete {
        variable: String,
        run_ref: String,
    },
}

impl WeatherBakeUseCase {
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

        let snapshot = match catalog.get(config.model) {
            Some(snapshot) => snapshot,
            None => {
                return Ok(BakeTickResult::Idle {
                    reason: "active spatial snapshot unavailable",
                });
            }
        };
        let snapshot = snapshot.as_ref();

        let objects: Vec<_> = snapshot.objects.iter().collect();
        if objects.is_empty() {
            return Ok(BakeTickResult::Idle {
                reason: "no synced timesteps",
            });
        }

        for plan in &config.plans {
            if let tick @ BakeTickResult::Progress { .. } =
                self.bake_tick_plan(config, plan, snapshot, &objects, tile_coords)?
            {
                return Ok(tick);
            }
        }

        if all_layers_complete(config, snapshot, &objects)? {
            return Ok(BakeTickResult::RunComplete {
                variable: config
                    .plans
                    .iter()
                    .map(|plan| plan.layer.id())
                    .collect::<Vec<_>>()
                    .join(","),
                run_ref: snapshot.run_ref.clone(),
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
        objects: &[&SpatialObjectLocal],
        tile_coords: &[TileCoord],
    ) -> Result<BakeTickResult, WeatherBakeError> {
        let variable = plan.layer.id().to_string();
        let mut work = resolve_manifest_work(config, plan, snapshot, objects)?;
        let pending = pending_objects(objects, &work.manifest);
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
            config,
            plan,
            &object.local_path,
            &snapshot.run_ref,
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

        if pending_objects(objects, &work.manifest).is_empty() && work.uses_staging {
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
            run_ref: snapshot.run_ref.clone(),
            valid_time: object.timestamp.clone(),
            completed,
            total: objects.len(),
        })
    }
}

pub fn load_weather_tile_coords(
    config: &WeatherBakeConfig,
) -> Result<Vec<TileCoord>, WeatherBakeError> {
    let coverages =
        resort_coverage::load_resort_coverages(&config.sqlite_path, config.complete_only)?;
    Ok(tile_index::build_weather_tile_index(
        GLOBAL_MAX_ZOOM,
        REGIONAL_MIN_ZOOM,
        REGIONAL_MAX_ZOOM,
        &coverages,
    ))
}

pub fn build_bake_plans(
    output_dir: PathBuf,
    manifest_dir: PathBuf,
    model: WeatherModelId,
    variables: Option<Vec<WeatherBakeLayer>>,
) -> Vec<WeatherBakePlan> {
    let layers = variables.unwrap_or_else(|| WeatherBakeLayer::available_for_model(model));
    layers
        .into_iter()
        .map(|layer| WeatherBakePlan {
            layer,
            output_dir: output_dir.join(layer.id()),
            manifest_path: manifest_dir
                .join(format!("weather_pmtiles_{}_manifest.json", layer.id())),
        })
        .collect()
}

fn all_layers_complete(
    config: &WeatherBakeConfig,
    snapshot: &SpatialRunSnapshot,
    objects: &[&SpatialObjectLocal],
) -> Result<bool, WeatherBakeError> {
    for plan in &config.plans {
        let work = resolve_manifest_work(config, plan, snapshot, objects)?;
        if !pending_objects(objects, &work.manifest).is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn pending_objects<'a>(
    objects: &[&'a SpatialObjectLocal],
    manifest: &WeatherPmtilesManifest,
) -> Vec<&'a SpatialObjectLocal> {
    let completed: HashSet<_> = manifest
        .artifacts
        .iter()
        .filter(|(_, artifact)| artifact_is_complete(artifact))
        .map(|(valid_time, _)| valid_time.as_str())
        .collect();
    objects
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
    objects: &[&SpatialObjectLocal],
) -> Result<ManifestWork, WeatherBakeError> {
    let published_path = &plan.manifest_path;
    let published = read_manifest(published_path)?;
    let has_published = published.is_some();
    if let Some(existing) = published
        && existing.run_ref == snapshot.run_ref
    {
        return Ok(ManifestWork {
            path: published_path.clone(),
            manifest: existing,
            uses_staging: false,
        });
    }

    let staging_path = staging_manifest_path(published_path);
    if let Some(staging) = read_manifest(&staging_path)?
        && staging.run_ref == snapshot.run_ref
    {
        return Ok(ManifestWork {
            path: staging_path,
            manifest: staging,
            uses_staging: true,
        });
    }

    if staging_path.exists() {
        std::fs::remove_file(&staging_path).map_err(|source| WeatherBakeError::WriteFile {
            path: staging_path.clone(),
            source,
        })?;
    }

    let uses_staging = has_published;
    Ok(ManifestWork {
        path: if uses_staging {
            staging_path
        } else {
            published_path.clone()
        },
        manifest: init_manifest(config, plan, snapshot, objects),
        uses_staging,
    })
}

fn init_manifest(
    config: &WeatherBakeConfig,
    plan: &WeatherBakePlan,
    snapshot: &SpatialRunSnapshot,
    objects: &[&SpatialObjectLocal],
) -> WeatherPmtilesManifest {
    WeatherPmtilesManifest {
        model: config.model.to_string(),
        variable: plan.layer.id().to_string(),
        run_ref: snapshot.run_ref.clone(),
        default_valid_time: objects
            .first()
            .map(|object| object.timestamp.clone())
            .unwrap_or_default(),
        valid_times: objects
            .iter()
            .map(|object| object.timestamp.clone())
            .collect(),
        global_max_zoom: GLOBAL_MAX_ZOOM,
        regional_min_zoom: REGIONAL_MIN_ZOOM,
        regional_max_zoom: REGIONAL_MAX_ZOOM,
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

fn bake_timestep(
    config: &WeatherBakeConfig,
    plan: &WeatherBakePlan,
    local_path: &Path,
    run_ref: &str,
    valid_time: &str,
    tile_coords: &[TileCoord],
) -> Result<WeatherPmtilesArtifact, WeatherBakeError> {
    let layer = plan.layer;
    let tiles: Vec<PmtilesTile> = match layer {
        WeatherBakeLayer::Wind => {
            let (u_name, v_name) = layer.wind_spatial_variables().expect("wind variables");
            let u_field = Arc::new(SpatialFieldRegridder::from_spatial_file_or_empty(
                local_path, u_name,
            )?);
            let v_field = Arc::new(SpatialFieldRegridder::from_spatial_file_or_empty(
                local_path, v_name,
            )?);
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
            let field = Arc::new(SpatialFieldRegridder::from_spatial_file_or_empty(
                local_path, variable,
            )?);
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

    let output_path = pmtiles_output_path(&plan.output_dir, run_ref, valid_time);
    let metadata = PmtilesMetadata {
        min_zoom: MIN_ZOOM,
        max_zoom: MAX_ZOOM,
        bounds: Some((-180.0, -85.051_128_78, 180.0, 85.051_128_78)),
        json: serde_json::json!({
            "model": config.model.to_string(),
            "variable": layer.id(),
            "run_ref": run_ref,
            "valid_time": valid_time,
            "global_max_zoom": GLOBAL_MAX_ZOOM,
            "regional_min_zoom": REGIONAL_MIN_ZOOM,
            "regional_max_zoom": REGIONAL_MAX_ZOOM,
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
        WeatherPmtilesManifest, artifact_is_complete, build_bake_plans, pending_objects,
        promote_staging_manifest, read_manifest, resolve_manifest_work, staging_manifest_path,
        write_manifest,
    };
    use crate::domain::WeatherModelId;
    use crate::domain::spatial_snapshot::{SpatialObjectLocal, SpatialRunSnapshot};
    use chrono::Utc;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn pending_objects_preserves_snapshot_order_for_priority_bake() {
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

        let pending = pending_objects(&object_refs, &manifest);
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

    fn sample_snapshot(run_ref: &str, objects: &[SpatialObjectLocal]) -> SpatialRunSnapshot {
        SpatialRunSnapshot {
            model: WeatherModelId::EcmwfIfs,
            reference_time: "2025-06-09T12:00:00Z".to_string(),
            run_ref: run_ref.to_string(),
            objects: objects.to_vec(),
        }
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
            output_dir: dir.join("temperature_2m"),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot("2025060912", &objects);
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
            output_dir: dir.join("temperature_2m"),
            manifest_path: manifest_path.clone(),
        };
        let snapshot = sample_snapshot("2025060918", &objects);
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
            swapped.artifacts.get("2025-06-09T18:00:00Z").map(|artifact| artifact.path.as_str()),
            Some(dir.join("new.pmtiles").display().to_string().as_str())
        );
        assert!(!staging_manifest_path(&manifest_path).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_bake_plans_uses_per_variable_output_and_manifest_paths() {
        let plans = build_bake_plans(
            PathBuf::from("data/processed/weather"),
            PathBuf::from("data/manifests"),
            WeatherModelId::EcmwfIfs,
            None,
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
    }
}
