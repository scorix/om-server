use std::ops::Range;

use crate::error::GridError;

#[derive(Debug, Clone)]
pub struct SpatialGrid {
    dimensions: Vec<u64>,
    lat_axis: usize,
    lon_axis: usize,
    bbox: GeoBbox,
}

#[derive(Debug, Clone)]
pub struct SpatialGridMetadata {
    pub dimensions: Vec<u64>,
    pub coordinates: String,
    pub crs_wkt: String,
}

#[derive(Debug, Clone)]
pub struct InterpolationWindow {
    pub ranges: Vec<Range<u64>>,
    pub indices: Vec<u64>,
    pub latitudes: [f64; 2],
    pub longitudes: [f64; 2],
    lat_weight: f64,
    lon_weight: f64,
    lat_axis: usize,
    lon_axis: usize,
}

#[derive(Debug, Clone)]
pub struct PointWindow {
    pub ranges: Vec<Range<u64>>,
    pub indices: Vec<u64>,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Clone, Copy)]
struct GeoBbox {
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
}

impl SpatialGrid {
    pub fn from_metadata(metadata: SpatialGridMetadata) -> Result<Self, GridError> {
        if metadata.dimensions.len() != 2 {
            return Err(GridError::InvalidDimensions {
                dimensions: metadata.dimensions,
            });
        }
        let axes = metadata.coordinates.split_whitespace().collect::<Vec<_>>();
        if axes != ["lat", "lon"] {
            return Err(GridError::UnsupportedCoordinates {
                coordinates: metadata.coordinates,
            });
        }
        let bbox = parse_wkt_bbox(&metadata.crs_wkt)?;
        Ok(Self {
            dimensions: metadata.dimensions,
            lat_axis: 0,
            lon_axis: 1,
            bbox,
        })
    }

    pub fn interpolation_window(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<InterpolationWindow, GridError> {
        let lat_position = self.latitude_position(latitude);
        let lon_position = self.longitude_position(longitude);
        let lat_lower = lat_position.floor() as u64;
        let lon_lower = lon_position.floor() as u64;
        let lat_upper = (lat_lower + 1).min(self.dimensions[self.lat_axis] - 1);
        let lon_upper = (lon_lower + 1).min(self.dimensions[self.lon_axis] - 1);
        let mut ranges = vec![0..0, 0..0];
        ranges[self.lat_axis] = lat_lower..lat_upper + 1;
        ranges[self.lon_axis] = lon_lower..lon_upper + 1;
        Ok(InterpolationWindow {
            ranges,
            indices: vec![lat_lower, lat_upper, lon_lower, lon_upper],
            latitudes: [self.latitude_at(lat_lower), self.latitude_at(lat_upper)],
            longitudes: [self.longitude_at(lon_lower), self.longitude_at(lon_upper)],
            lat_weight: lat_position - lat_lower as f64,
            lon_weight: lon_position - lon_lower as f64,
            lat_axis: self.lat_axis,
            lon_axis: self.lon_axis,
        })
    }

    pub fn point_window(&self, latitude: f64, longitude: f64) -> Result<PointWindow, GridError> {
        let lat_index = self.latitude_position(latitude).round() as u64;
        let lon_index = self.longitude_position(longitude).round() as u64;
        let mut ranges = vec![0..0, 0..0];
        ranges[self.lat_axis] = lat_index..lat_index + 1;
        ranges[self.lon_axis] = lon_index..lon_index + 1;
        Ok(PointWindow {
            ranges,
            indices: vec![lat_index, lon_index],
            latitude: self.latitude_at(lat_index),
            longitude: self.longitude_at(lon_index),
        })
    }

    pub fn label(&self) -> &'static str {
        "coordinates=lat lon,crs_wkt_bbox"
    }

    fn latitude_position(&self, latitude: f64) -> f64 {
        let max_index = self.dimensions[self.lat_axis].saturating_sub(1);
        let ratio = (latitude - self.bbox.min_lat) / (self.bbox.max_lat - self.bbox.min_lat);
        (ratio * max_index as f64).clamp(0.0, max_index as f64)
    }

    fn longitude_position(&self, longitude: f64) -> f64 {
        let max_index = self.dimensions[self.lon_axis].saturating_sub(1);
        let normalized = normalize_longitude(longitude, self.bbox.min_lon);
        let ratio = (normalized - self.bbox.min_lon) / (self.bbox.max_lon - self.bbox.min_lon);
        (ratio * max_index as f64).clamp(0.0, max_index as f64)
    }

    fn latitude_at(&self, index: u64) -> f64 {
        let max_index = self.dimensions[self.lat_axis].saturating_sub(1);
        self.bbox.min_lat
            + index as f64 * ((self.bbox.max_lat - self.bbox.min_lat) / max_index as f64)
    }

    fn longitude_at(&self, index: u64) -> f64 {
        let max_index = self.dimensions[self.lon_axis].saturating_sub(1);
        self.bbox.min_lon
            + index as f64 * ((self.bbox.max_lon - self.bbox.min_lon) / max_index as f64)
    }
}

impl InterpolationWindow {
    pub fn interpolate(&self, values: &[f32]) -> Result<f64, GridError> {
        match values.len() {
            4 => {
                let value_at = |lat_offset: usize, lon_offset: usize| -> f64 {
                    let mut offsets = [0usize; 2];
                    offsets[self.lat_axis] = lat_offset;
                    offsets[self.lon_axis] = lon_offset;
                    values[offsets[0] * 2 + offsets[1]] as f64
                };
                let lower_lat =
                    value_at(0, 0) * (1.0 - self.lon_weight) + value_at(0, 1) * self.lon_weight;
                let upper_lat =
                    value_at(1, 0) * (1.0 - self.lon_weight) + value_at(1, 1) * self.lon_weight;
                Ok(lower_lat * (1.0 - self.lat_weight) + upper_lat * self.lat_weight)
            }
            2 if self.indices[0] == self.indices[1] => {
                Ok(values[0] as f64 * (1.0 - self.lon_weight) + values[1] as f64 * self.lon_weight)
            }
            2 if self.indices[2] == self.indices[3] => {
                Ok(values[0] as f64 * (1.0 - self.lat_weight) + values[1] as f64 * self.lat_weight)
            }
            1 => Ok(values[0] as f64),
            count => Err(GridError::InvalidInterpolationWindow { count }),
        }
    }
}

impl PointWindow {
    pub fn value(&self, values: &[f32]) -> Result<f64, GridError> {
        if values.len() != 1 {
            return Err(GridError::InvalidPointWindow {
                count: values.len(),
            });
        }
        Ok(values[0] as f64)
    }
}

fn parse_wkt_bbox(value: &str) -> Result<GeoBbox, GridError> {
    let start = value.find("BBOX[").ok_or(GridError::MissingBbox)? + "BBOX[".len();
    let end = value[start..]
        .find(']')
        .map(|end| start + end)
        .ok_or(GridError::UnterminatedBbox)?;
    let values = value[start..end]
        .split(',')
        .map(|part| part.trim().parse::<f64>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| GridError::ParseBboxValues { source })?;
    if values.len() != 4 {
        return Err(GridError::InvalidBboxValueCount {
            count: values.len(),
        });
    }
    Ok(GeoBbox {
        min_lat: values[0],
        min_lon: values[1],
        max_lat: values[2],
        max_lon: values[3],
    })
}

fn normalize_longitude(longitude: f64, min_longitude: f64) -> f64 {
    let mut normalized = longitude;
    while normalized < min_longitude {
        normalized += 360.0;
    }
    while normalized >= min_longitude + 360.0 {
        normalized -= 360.0;
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grid() -> SpatialGrid {
        SpatialGrid::from_metadata(SpatialGridMetadata {
            dimensions: vec![3, 3],
            coordinates: "lat lon".to_string(),
            crs_wkt: "BBOX[-90,-180,90,180]".to_string(),
        })
        .expect("grid")
    }

    #[test]
    fn interpolation_window_uses_four_neighboring_cells() {
        let grid = sample_grid();
        let window = grid.interpolation_window(0.0, 0.0).expect("window");
        assert_eq!(window.ranges[0], 1..3);
        assert_eq!(window.ranges[1], 1..3);
    }

    #[test]
    fn bilinear_interpolation_weights_corner_values() {
        let grid = sample_grid();
        let window = grid.interpolation_window(45.0, 90.0).expect("window");
        assert!((window.lat_weight - 0.5).abs() < 1e-9);
        assert!((window.lon_weight - 0.5).abs() < 1e-9);
        let value = window
            .interpolate(&[10.0, 20.0, 30.0, 40.0])
            .expect("interpolate");
        assert!((value - 25.0).abs() < 1e-9);
    }
}
