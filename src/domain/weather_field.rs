use rayon::prelude::*;

use crate::domain::SpatialGrid;
use crate::domain::spatial_sample::sample_scalar_field;
use crate::error::{DatasetError, GridError};

/// Regular 0.25° lat/lon grid (1440 × 721) for Mercator tile sampling.
pub const REGULAR_LON_COUNT: u32 = 1440;
pub const REGULAR_LAT_COUNT: u32 = 721;
const WEB_MERCATOR_MAX_LAT: f64 = 85.051_128_78;

#[derive(Debug, Clone)]
pub struct RegularLatLonField {
    width: u32,
    height: u32,
    /// Row-major [lat_index][lon_index], lat_index 0 = north.
    values: Vec<f32>,
}

impl RegularLatLonField {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            values: vec![f32::NAN; (width * height) as usize],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn sample(&self, latitude: f64, longitude: f64) -> Option<f32> {
        if !latitude.is_finite() || !longitude.is_finite() {
            return None;
        }
        let lat = latitude.clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT);
        let lon = normalize_longitude(longitude);
        let row_f = (90.0 - lat) / 180.0 * f64::from(self.height - 1);
        let col_f = (lon + 180.0) / 360.0 * f64::from(self.width - 1);
        let row0 = row_f.floor() as u32;
        let col0 = col_f.floor() as u32;
        let row1 = (row0 + 1).min(self.height - 1);
        let col1 = (col0 + 1).min(self.width - 1);
        let lat_w = row_f - f64::from(row0);
        let lon_w = col_f - f64::from(col0);

        let v00 = self.get(col0, row0)?;
        let v10 = self.get(col1, row0)?;
        let v01 = self.get(col0, row1)?;
        let v11 = self.get(col1, row1)?;

        let lower = v00 * (1.0 - lon_w) as f32 + v10 * lon_w as f32;
        let upper = v01 * (1.0 - lon_w) as f32 + v11 * lon_w as f32;
        Some(lower * (1.0 - lat_w) as f32 + upper * lat_w as f32)
    }

    fn get(&self, col: u32, row: u32) -> Option<f32> {
        let value = self.values[(row * self.width + col) as usize];
        value.is_finite().then_some(value)
    }

    pub fn set(&mut self, col: u32, row: u32, value: Option<f32>) {
        self.values[(row * self.width + col) as usize] = value.unwrap_or(f32::NAN);
    }
}

pub struct SpatialFieldRegridder;

impl SpatialFieldRegridder {
    pub fn regrid(
        grid: &SpatialGrid,
        source_values: &[f32],
        width: u32,
        height: u32,
    ) -> Result<RegularLatLonField, GridError> {
        let mut field = RegularLatLonField::new(width, height);
        let rows: Vec<u32> = (0..height).collect();
        let regridded: Vec<(u32, Vec<f32>)> = rows
            .par_iter()
            .map(|&row| {
                let lat = 90.0 - (f64::from(row) + 0.5) * 180.0 / f64::from(height);
                let mut row_values = Vec::with_capacity(width as usize);
                for col in 0..width {
                    let lon = -180.0 + (f64::from(col) + 0.5) * 360.0 / f64::from(width);
                    let value = sample_scalar_field(grid, source_values, lat, lon)
                        .ok()
                        .filter(|value| value.is_finite())
                        .map(|value| value as f32);
                    row_values.push(value.unwrap_or(f32::NAN));
                }
                (row, row_values)
            })
            .collect();

        for (row, row_values) in regridded {
            for (col, value) in row_values.into_iter().enumerate() {
                field.values[(row * width + col as u32) as usize] = value;
            }
        }
        Ok(field)
    }

    pub fn from_spatial_file(
        local_path: &std::path::Path,
        variable_name: &str,
    ) -> Result<RegularLatLonField, DatasetError> {
        let (grid, values) = crate::infrastructure::spatial_field_loader::read_flat_variable(
            local_path,
            variable_name,
        )?;
        Self::regrid(&grid, &values, REGULAR_LON_COUNT, REGULAR_LAT_COUNT)
            .map_err(DatasetError::Grid)
    }

    /// Like [`Self::from_spatial_file`], but returns an all-NaN field when the variable is absent.
    ///
    /// Some timesteps (e.g. analysis at T+0) omit cumulative fields such as
    /// `snowfall_water_equivalent`; tile renderers treat those samples as transparent.
    pub fn from_spatial_file_or_empty(
        local_path: &std::path::Path,
        variable_name: &str,
    ) -> Result<RegularLatLonField, DatasetError> {
        match Self::from_spatial_file(local_path, variable_name) {
            Ok(field) => Ok(field),
            Err(DatasetError::VariableNotFound { .. }) => Ok(RegularLatLonField::new(
                REGULAR_LON_COUNT,
                REGULAR_LAT_COUNT,
            )),
            Err(err) => Err(err),
        }
    }
}

fn normalize_longitude(longitude: f64) -> f64 {
    let mut lon = longitude;
    while lon < -180.0 {
        lon += 360.0;
    }
    while lon >= 180.0 {
        lon -= 360.0;
    }
    lon
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SpatialGridMetadata;

    #[test]
    fn regrid_regular_fixture_produces_finite_center_value() {
        let metadata = SpatialGridMetadata {
            dimensions: vec![2, 5],
            coordinates: "lat lon".to_string(),
            crs_wkt: "GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433],AXIS[\"Lat\",NORTH],AXIS[\"Lon\",EAST]] BBOX[-90,-180,90,180]]"
                .to_string(),
        };
        let grid = SpatialGrid::from_metadata(metadata).expect("grid");
        let source = vec![0.0, 5.0, 2.0, 3.0, 2.0, 5.0, 6.0, 2.0, 8.0, 3.0];
        let field = SpatialFieldRegridder::regrid(&grid, &source, 36, 18).expect("regrid");
        let sample = field
            .sample(0.0, 0.0)
            .expect("sample equator/prime meridian");
        assert!(sample.is_finite());
    }
}
