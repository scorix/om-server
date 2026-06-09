use crate::domain::SpatialGrid;
use crate::error::GridError;

/// Bilinear sample from a flat spatial variable array using grid metadata.
pub fn sample_scalar_field(
    grid: &SpatialGrid,
    values: &[f32],
    latitude: f64,
    longitude: f64,
) -> Result<f64, GridError> {
    grid.sample_field_value(values, latitude, longitude)
}
