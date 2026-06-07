use std::collections::BTreeMap;

use crate::domain::WeatherModelId;
use crate::domain::spatial_snapshot::SpatialRunSnapshot;
use crate::error::OpenMeteoError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialObjectRef {
    pub object_key: String,
    pub timestamp: String,
    pub valid_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialRun {
    pub reference_time: String,
    pub run_prefix: String,
    pub run_ref: String,
    pub objects: Vec<SpatialObjectRef>,
}

impl SpatialRun {
    pub fn grouped_by_date(&self) -> BTreeMap<String, Vec<SpatialObjectRef>> {
        let mut grouped: BTreeMap<String, Vec<SpatialObjectRef>> = BTreeMap::new();
        for object in &self.objects {
            grouped
                .entry(object.valid_date.clone())
                .or_default()
                .push(object.clone());
        }
        grouped
    }

    pub fn planned_objects(&self, forecast_days: usize) -> Vec<SpatialObjectRef> {
        self.grouped_by_date()
            .into_iter()
            .take(forecast_days)
            .flat_map(|(_, day_objects)| day_objects)
            .collect()
    }

    pub fn matches_snapshot(&self, snapshot: &SpatialRunSnapshot) -> bool {
        !self.reference_time.is_empty() && self.reference_time == snapshot.reference_time
    }
}

pub trait SpatialRunCatalog: Send + Sync {
    fn resolve_spatial_run(&self, model: WeatherModelId) -> Result<SpatialRun, OpenMeteoError>;
}
