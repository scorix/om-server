use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use crate::domain::{SpatialGrid, SpatialGridMetadata};
use crate::error::DatasetError;

static CACHE: LazyLock<Mutex<HashMap<GridCacheKey, Arc<SpatialGrid>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GridCacheKey {
    dimensions: Vec<u64>,
    coordinates: String,
    crs_wkt: String,
}

impl GridCacheKey {
    fn from_metadata(metadata: &SpatialGridMetadata) -> Self {
        Self {
            dimensions: metadata.dimensions.clone(),
            coordinates: metadata.coordinates.clone(),
            crs_wkt: metadata.crs_wkt.clone(),
        }
    }
}

pub fn get_or_insert(metadata: SpatialGridMetadata) -> Result<Arc<SpatialGrid>, DatasetError> {
    let key = GridCacheKey::from_metadata(&metadata);
    let mut cache = CACHE.lock().expect("spatial grid cache mutex poisoned");
    if let Some(grid) = cache.get(&key) {
        return Ok(Arc::clone(grid));
    }
    let grid = Arc::new(SpatialGrid::from_metadata(metadata).map_err(DatasetError::Grid)?);
    cache.insert(key, Arc::clone(&grid));
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SpatialGridMetadata;

    #[test]
    fn returns_same_arc_for_identical_metadata() {
        clear_for_tests();
        let metadata = SpatialGridMetadata {
            dimensions: vec![1, 6_599_680],
            coordinates: "lat lon".to_string(),
            crs_wkt: "GEOGCRS[\"Reduced Gaussian Grid\",REMARK[\"Reduced Gaussian Grid O1280 (ECMWF)\"],USAGE[BBOX[-90,-180.0,90,180]]]"
                .to_string(),
        };
        let first = get_or_insert(metadata.clone()).expect("first grid");
        let second = get_or_insert(metadata).expect("cached grid");
        assert!(Arc::ptr_eq(&first, &second));
    }

    fn clear_for_tests() {
        CACHE
            .lock()
            .expect("spatial grid cache mutex poisoned")
            .clear();
    }
}
