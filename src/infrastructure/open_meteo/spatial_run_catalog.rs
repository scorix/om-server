use crate::domain::{SpatialObjectRef, SpatialRun, SpatialRunCatalog, WeatherModelId};
use crate::error::OpenMeteoError;

use super::run_manifest::RunManifest;
use super::s3_catalog::OpenMeteoS3Catalog;

pub struct OpenMeteoSpatialRunCatalog {
    s3: OpenMeteoS3Catalog,
}

impl OpenMeteoSpatialRunCatalog {
    pub fn new(s3: OpenMeteoS3Catalog) -> Self {
        Self { s3 }
    }
}

impl SpatialRunCatalog for OpenMeteoSpatialRunCatalog {
    fn resolve_spatial_run(&self, model: WeatherModelId) -> Result<SpatialRun, OpenMeteoError> {
        RunManifestLoader::new(&self.s3)
            .load(model)?
            .resolve(model, &self.s3)
    }
}

struct RunManifestLoader<'a> {
    s3: &'a OpenMeteoS3Catalog,
}

impl<'a> RunManifestLoader<'a> {
    fn new(s3: &'a OpenMeteoS3Catalog) -> Self {
        Self { s3 }
    }

    fn load(self, model: WeatherModelId) -> Result<LoadedRunManifest, OpenMeteoError> {
        let url = format!(
            "{}/data_run/{}/latest.json",
            self.s3.base_url().trim_end_matches('/'),
            model.as_str()
        );
        let body = self.s3.fetch_text(&url)?;
        let manifest: RunManifest = serde_json::from_str(&body)
            .map_err(|source| OpenMeteoError::ParseRunManifest { url, source })?;
        Ok(LoadedRunManifest { manifest })
    }
}

struct LoadedRunManifest {
    manifest: RunManifest,
}

impl LoadedRunManifest {
    fn resolve(
        self,
        model: WeatherModelId,
        s3: &OpenMeteoS3Catalog,
    ) -> Result<SpatialRun, OpenMeteoError> {
        SpatialRunResolver::new(self.manifest).resolve(model, s3)
    }
}

struct SpatialRunResolver {
    manifest: RunManifest,
}

impl SpatialRunResolver {
    fn new(manifest: RunManifest) -> Self {
        Self { manifest }
    }

    fn resolve(
        self,
        model: WeatherModelId,
        s3: &OpenMeteoS3Catalog,
    ) -> Result<SpatialRun, OpenMeteoError> {
        let run_prefix = self.manifest.spatial_run_prefix(model)?;
        let allowed = self.manifest.spatial_timestamps()?;
        let objects = s3
            .list_spatial_objects(&run_prefix)?
            .into_iter()
            .filter(|object| allowed.contains(&object.timestamp))
            .collect::<Vec<SpatialObjectRef>>();
        if objects.is_empty() {
            return Err(OpenMeteoError::NoSpatialObjects {
                prefix: run_prefix.clone(),
            });
        }
        Ok(SpatialRun {
            reference_time: self.manifest.reference_time,
            run_ref: OpenMeteoS3Catalog::run_ref_from_prefix(&run_prefix),
            run_prefix,
            objects,
        })
    }
}
