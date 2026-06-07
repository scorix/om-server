use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::domain::WeatherModelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialObjectLocal {
    pub object_key: String,
    pub timestamp: String,
    pub valid_date: String,
    pub local_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialRunSnapshot {
    pub model: WeatherModelId,
    pub reference_time: String,
    pub run_ref: String,
    pub objects: Vec<SpatialObjectLocal>,
}

impl SpatialRunSnapshot {
    pub fn grouped_by_date(&self) -> BTreeMap<String, Vec<&SpatialObjectLocal>> {
        let mut grouped: BTreeMap<String, Vec<&SpatialObjectLocal>> = BTreeMap::new();
        for object in &self.objects {
            grouped
                .entry(object.valid_date.clone())
                .or_default()
                .push(object);
        }
        grouped
    }
}
