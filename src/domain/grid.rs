use std::ops::Range;

use crate::error::GridError;

#[derive(Debug, Clone)]
pub struct SpatialGrid {
    backend: GridBackend,
}

#[derive(Debug, Clone)]
enum GridBackend {
    Regular {
        dimensions: Vec<u64>,
        lat_axis: usize,
        lon_axis: usize,
        bbox: GeoBbox,
    },
    Gaussian(GaussianGrid),
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
    /// Scattered 1D gridpoint indices for Gaussian bilinear: SW, SE, NW, NE.
    pub sample_indices: Vec<u64>,
    pub latitudes: [f64; 2],
    pub longitudes: [f64; 2],
    lat_weight: f64,
    lon_weight: f64,
    lon_weight_upper: f64,
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

/// 1D reduced Gaussian grid (ECMWF O/N or full F notation in `crs_wkt`).
#[derive(Debug, Clone)]
struct GaussianGrid {
    kind: GaussianGridKind,
    latitude_lines: u32,
    nx: u64,
    integral_table: Vec<u64>,
}

#[derive(Debug, Clone, Copy)]
enum GaussianGridKind {
    /// Octahedral reduced Gaussian (O_n): nx grows as 20 + 4y toward the equator.
    Octahedral,
    /// Full Gaussian (F_n): every latitude circle has 4n longitudes.
    Full,
}

impl GaussianGrid {
    fn from_metadata(metadata: &SpatialGridMetadata) -> Result<Self, GridError> {
        if metadata.dimensions.len() != 2 || metadata.dimensions[0] != 1 {
            return Err(GridError::InvalidDimensions {
                dimensions: metadata.dimensions.clone(),
            });
        }
        let nx = metadata.dimensions[1];
        let (kind, latitude_lines) = parse_gaussian_grid(&metadata.crs_wkt, nx)?;
        let expected = total_points(kind, latitude_lines);
        if nx != expected {
            return Err(GridError::GaussianPointCountMismatch {
                expected,
                actual: nx,
            });
        }
        let mut integral_table = Vec::with_capacity(2 * latitude_lines as usize + 1);
        integral_table.push(0);
        for y in 0..2 * latitude_lines {
            let last = *integral_table.last().expect("integral table");
            integral_table.push(last + nx_of_y(kind, y, latitude_lines));
        }
        Ok(Self {
            kind,
            latitude_lines,
            nx,
            integral_table,
        })
    }

    fn find_gridpoint(&self, latitude: f64, longitude: f64) -> Result<u64, GridError> {
        let (x_idx, y_idx) = find_point_xy(self.kind, latitude, longitude, self.latitude_lines);
        Ok(self.gridpoint_at(y_idx, x_idx))
    }

    fn gridpoint_at(&self, y: u32, x: u64) -> u64 {
        let nx = nx_of_y(self.kind, y, self.latitude_lines);
        self.integral(y) + (x % nx)
    }

    fn interpolation_window(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<InterpolationWindow, GridError> {
        let dy = 180.0 / (2.0 * f64::from(self.latitude_lines) + 0.5);
        let y_float = f64::from(self.latitude_lines) - 1.0 - ((latitude - dy / 2.0) / dy);
        let y_lower = y_float.floor() as u32;
        let y_upper = (y_lower + 1).min(2 * self.latitude_lines - 2);
        let lat_weight = if y_lower == y_upper {
            0.0
        } else {
            (y_float - f64::from(y_lower)).clamp(0.0, 1.0)
        };

        let (x_lower_s, x_upper_s, lon_weight) =
            longitude_neighbors(self.kind, longitude, y_lower, self.latitude_lines);
        let (x_lower_n, x_upper_n, lon_weight_upper) =
            longitude_neighbors(self.kind, longitude, y_upper, self.latitude_lines);

        let gp_sw = self.gridpoint_at(y_lower, x_lower_s);
        let gp_se = self.gridpoint_at(y_lower, x_upper_s);
        let gp_nw = self.gridpoint_at(y_upper, x_lower_n);
        let gp_ne = self.gridpoint_at(y_upper, x_upper_n);

        let (lat_s, lon_sw) = self.coordinates(gp_sw)?;
        let (_, lon_se) = self.coordinates(gp_se)?;
        let (lat_n, _) = self.coordinates(gp_nw)?;
        let _ = self.coordinates(gp_ne)?;

        Ok(InterpolationWindow {
            ranges: vec![0..1, 0..1],
            indices: vec![u64::from(y_lower), u64::from(y_upper), x_lower_s, x_upper_s],
            sample_indices: vec![gp_sw, gp_se, gp_nw, gp_ne],
            latitudes: [lat_s, lat_n],
            longitudes: [lon_sw, lon_se],
            lat_weight,
            lon_weight,
            lon_weight_upper,
            lat_axis: 0,
            lon_axis: 1,
        })
    }

    fn coordinates(&self, gridpoint: u64) -> Result<(f64, f64), GridError> {
        let (y, x, nx) = self.position(gridpoint)?;
        let dy = 180.0 / (2.0 * f64::from(self.latitude_lines) + 0.5);
        let lat = f64::from(self.latitude_lines - y - 1) * dy + dy / 2.0;
        let dx = 360.0 / f64::from(nx);
        let mut lon = x as f64 * dx;
        if lon >= 180.0 {
            lon -= 360.0;
        }
        Ok((lat, lon))
    }

    fn integral(&self, y: u32) -> u64 {
        self.integral_table[y as usize]
    }

    fn position(&self, gridpoint: u64) -> Result<(u32, u64, u32), GridError> {
        if gridpoint >= self.nx {
            return Err(GridError::GaussianGridPointOutOfRange {
                gridpoint,
                max: self.nx,
            });
        }
        match self.kind {
            GaussianGridKind::Full => {
                let nx_row = nx_of_y(self.kind, 0, self.latitude_lines);
                let y = (gridpoint / nx_row) as u32;
                let x = gridpoint % nx_row;
                Ok((y, x, nx_row as u32))
            }
            GaussianGridKind::Octahedral => {
                let half_count = self.nx / 2;
                let y = if gridpoint < half_count {
                    ((2.0 * gridpoint as f64 + 81.0).sqrt() - 9.0) / 2.0
                } else {
                    let gridpoint_from_end = self.nx - gridpoint - 1;
                    let y_from_end = ((2.0 * gridpoint_from_end as f64 + 81.0).sqrt() - 9.0) / 2.0;
                    f64::from(2 * self.latitude_lines - 1) - y_from_end
                } as u32;
                let x = gridpoint - self.integral(y);
                let nx = nx_of_y(self.kind, y, self.latitude_lines) as u32;
                Ok((y, x, nx))
            }
        }
    }
}

fn total_points(kind: GaussianGridKind, latitude_lines: u32) -> u64 {
    let n = u64::from(latitude_lines);
    match kind {
        GaussianGridKind::Octahedral => 4 * n * (n + 9),
        GaussianGridKind::Full => 8 * n * n,
    }
}

fn parse_gaussian_grid(crs_wkt: &str, nx: u64) -> Result<(GaussianGridKind, u32), GridError> {
    if let Some(n) = parse_grid_number_from_crs_wkt(crs_wkt, 'O') {
        let expected = total_points(GaussianGridKind::Octahedral, n);
        if nx != expected {
            return Err(GridError::GaussianPointCountMismatch {
                expected,
                actual: nx,
            });
        }
        return Ok((GaussianGridKind::Octahedral, n));
    }
    if let Some(n) = parse_grid_number_from_crs_wkt(crs_wkt, 'F') {
        let expected = total_points(GaussianGridKind::Full, n);
        if nx != expected {
            return Err(GridError::GaussianPointCountMismatch {
                expected,
                actual: nx,
            });
        }
        return Ok((GaussianGridKind::Full, n));
    }
    if let Some(n) = latitude_lines_from_point_count(GaussianGridKind::Octahedral, nx) {
        return Ok((GaussianGridKind::Octahedral, n));
    }
    if let Some(n) = latitude_lines_from_point_count(GaussianGridKind::Full, nx) {
        return Ok((GaussianGridKind::Full, n));
    }
    Err(GridError::UnsupportedGaussianGrid { point_count: nx })
}

fn parse_grid_number_from_crs_wkt(crs_wkt: &str, prefix: char) -> Option<u32> {
    for (idx, ch) in crs_wkt.char_indices() {
        if ch != prefix {
            continue;
        }
        if idx > 0 {
            let prev = crs_wkt[..idx].chars().next_back()?;
            if prev.is_ascii_alphabetic() {
                continue;
            }
        }
        let digits: String = crs_wkt[idx + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if !digits.is_empty() {
            return digits.parse().ok();
        }
    }
    None
}

fn latitude_lines_from_point_count(kind: GaussianGridKind, nx: u64) -> Option<u32> {
    for n in 1u32..=20_000 {
        if total_points(kind, n) == nx {
            return Some(n);
        }
        if total_points(kind, n) > nx {
            break;
        }
    }
    None
}

fn is_gaussian_grid(crs_wkt: &str) -> bool {
    crs_wkt.contains("Reduced Gaussian Grid") || crs_wkt.contains("Gaussian Grid")
}

fn nx_of_y(kind: GaussianGridKind, y: u32, latitude_lines: u32) -> u64 {
    match kind {
        GaussianGridKind::Full => u64::from(4 * latitude_lines),
        GaussianGridKind::Octahedral => {
            if y < latitude_lines {
                u64::from(20 + y * 4)
            } else {
                u64::from((2 * latitude_lines - y - 1) * 4 + 20)
            }
        }
    }
}

fn longitude_neighbors(
    kind: GaussianGridKind,
    longitude: f64,
    y: u32,
    latitude_lines: u32,
) -> (u64, u64, f64) {
    let nx = nx_of_y(kind, y, latitude_lines);
    let dx = 360.0 / nx as f64;
    let lon = normalize_longitude_0_360(longitude);
    let x_float = lon / dx;
    let x_frac = x_float - x_float.floor();
    let x_lower = (x_float.floor() as u64) % nx;
    let x_upper = (x_lower + 1) % nx;
    (x_lower, x_upper, x_frac)
}

fn normalize_longitude_0_360(longitude: f64) -> f64 {
    let mut lon = longitude;
    while lon < 0.0 {
        lon += 360.0;
    }
    while lon >= 360.0 {
        lon -= 360.0;
    }
    lon
}

fn find_point_xy(
    kind: GaussianGridKind,
    latitude: f64,
    longitude: f64,
    latitude_lines: u32,
) -> (u64, u32) {
    let dy = 180.0 / (2.0 * f64::from(latitude_lines) + 0.5);
    let y_float = f64::from(latitude_lines) - 1.0 - ((latitude - dy / 2.0) / dy);
    let y = y_float.floor() as u32;
    let y = y.clamp(0, 2 * latitude_lines - 2);
    let y_upper = y + 1;

    let nx = nx_of_y(kind, y, latitude_lines);
    let nx_upper = nx_of_y(kind, y_upper, latitude_lines);
    let dx = 360.0 / nx as f64;
    let dx_upper = 360.0 / nx_upper as f64;

    let x = (longitude / dx).round() as u64;
    let x_upper = (longitude / dx_upper).round() as u64;

    let point_lat = f64::from(latitude_lines - y - 1) * dy + dy / 2.0;
    let point_lon = x as f64 * dx;
    let point_lat_upper = f64::from(latitude_lines - y_upper - 1) * dy + dy / 2.0;
    let point_lon_upper = x_upper as f64 * dx_upper;

    let distance = (point_lat - latitude).powi(2) + (point_lon - longitude).powi(2);
    let distance_upper =
        (point_lat_upper - latitude).powi(2) + (point_lon_upper - longitude).powi(2);

    if distance < distance_upper {
        ((x + nx) % nx, y)
    } else {
        ((x_upper + nx_upper) % nx_upper, y_upper)
    }
}

impl SpatialGrid {
    pub fn from_metadata(metadata: SpatialGridMetadata) -> Result<Self, GridError> {
        if is_gaussian_grid(&metadata.crs_wkt) {
            return Ok(Self {
                backend: GridBackend::Gaussian(GaussianGrid::from_metadata(&metadata)?),
            });
        }

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
            backend: GridBackend::Regular {
                dimensions: metadata.dimensions,
                lat_axis: 0,
                lon_axis: 1,
                bbox,
            },
        })
    }

    pub fn interpolation_window(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<InterpolationWindow, GridError> {
        match &self.backend {
            GridBackend::Gaussian(grid) => grid.interpolation_window(latitude, longitude),
            GridBackend::Regular {
                dimensions,
                lat_axis,
                lon_axis,
                bbox,
            } => regular_interpolation_window(
                dimensions, *lat_axis, *lon_axis, *bbox, latitude, longitude,
            ),
        }
    }

    pub fn point_window(&self, latitude: f64, longitude: f64) -> Result<PointWindow, GridError> {
        match &self.backend {
            GridBackend::Gaussian(grid) => {
                let gridpoint = grid.find_gridpoint(latitude, longitude)?;
                let (lat, lon) = grid.coordinates(gridpoint)?;
                Ok(PointWindow {
                    ranges: vec![0..1, gridpoint..gridpoint + 1],
                    indices: vec![0, gridpoint],
                    latitude: lat,
                    longitude: lon,
                })
            }
            GridBackend::Regular {
                dimensions,
                lat_axis,
                lon_axis,
                bbox,
            } => regular_point_window(dimensions, *lat_axis, *lon_axis, *bbox, latitude, longitude),
        }
    }

    pub fn sample_field_value(
        &self,
        values: &[f32],
        latitude: f64,
        longitude: f64,
    ) -> Result<f64, GridError> {
        let window = self.interpolation_window(latitude, longitude)?;
        if window.sample_indices.len() == 4 {
            let scattered: Vec<f32> = window
                .sample_indices
                .iter()
                .map(|&index| {
                    values.get(index as usize).copied().ok_or(
                        GridError::GaussianGridPointOutOfRange {
                            gridpoint: index,
                            max: values.len() as u64,
                        },
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            return window.interpolate(&scattered);
        }

        let lon_stride = match &self.backend {
            GridBackend::Regular {
                dimensions,
                lon_axis,
                ..
            } => dimensions[*lon_axis],
            GridBackend::Gaussian(_) => {
                return Err(GridError::InvalidInterpolationWindow {
                    count: values.len(),
                });
            }
        };

        let mut samples = Vec::with_capacity(4);
        let lat_range = &window.ranges[window.lat_axis];
        let lon_range = &window.ranges[window.lon_axis];
        for lat_index in lat_range.start..lat_range.end {
            for lon_index in lon_range.start..lon_range.end {
                let flat_index = lat_index * lon_stride + lon_index;
                let value = values.get(flat_index as usize).copied().ok_or(
                    GridError::InvalidInterpolationWindow {
                        count: values.len(),
                    },
                )?;
                samples.push(value);
            }
        }
        window.interpolate(&samples)
    }

    pub fn label(&self) -> &'static str {
        match &self.backend {
            GridBackend::Gaussian(_) => "coordinates=gaussian,crs_wkt_reduced",
            GridBackend::Regular { .. } => "coordinates=lat lon,crs_wkt_bbox",
        }
    }
}

fn regular_interpolation_window(
    dimensions: &[u64],
    lat_axis: usize,
    lon_axis: usize,
    bbox: GeoBbox,
    latitude: f64,
    longitude: f64,
) -> Result<InterpolationWindow, GridError> {
    let lat_position = latitude_position(dimensions, lat_axis, bbox, latitude);
    let lon_position = longitude_position(dimensions, lon_axis, bbox, longitude);
    let lat_lower = lat_position.floor() as u64;
    let lon_lower = lon_position.floor() as u64;
    let lat_upper = (lat_lower + 1).min(dimensions[lat_axis] - 1);
    let lon_upper = (lon_lower + 1).min(dimensions[lon_axis] - 1);
    let mut ranges = vec![0..0, 0..0];
    ranges[lat_axis] = lat_lower..lat_upper + 1;
    ranges[lon_axis] = lon_lower..lon_upper + 1;
    let lon_weight = lon_position - lon_lower as f64;
    Ok(InterpolationWindow {
        ranges,
        indices: vec![lat_lower, lat_upper, lon_lower, lon_upper],
        sample_indices: Vec::new(),
        latitudes: [
            latitude_at(dimensions, lat_axis, bbox, lat_lower),
            latitude_at(dimensions, lat_axis, bbox, lat_upper),
        ],
        longitudes: [
            longitude_at(dimensions, lon_axis, bbox, lon_lower),
            longitude_at(dimensions, lon_axis, bbox, lon_upper),
        ],
        lat_weight: lat_position - lat_lower as f64,
        lon_weight,
        lon_weight_upper: lon_weight,
        lat_axis,
        lon_axis,
    })
}

fn regular_point_window(
    dimensions: &[u64],
    lat_axis: usize,
    lon_axis: usize,
    bbox: GeoBbox,
    latitude: f64,
    longitude: f64,
) -> Result<PointWindow, GridError> {
    let lat_index = latitude_position(dimensions, lat_axis, bbox, latitude).round() as u64;
    let lon_index = longitude_position(dimensions, lon_axis, bbox, longitude).round() as u64;
    let mut ranges = vec![0..0, 0..0];
    ranges[lat_axis] = lat_index..lat_index + 1;
    ranges[lon_axis] = lon_index..lon_index + 1;
    Ok(PointWindow {
        ranges,
        indices: vec![lat_index, lon_index],
        latitude: latitude_at(dimensions, lat_axis, bbox, lat_index),
        longitude: longitude_at(dimensions, lon_axis, bbox, lon_index),
    })
}

impl InterpolationWindow {
    pub fn interpolate(&self, values: &[f32]) -> Result<f64, GridError> {
        match values.len() {
            4 => {
                if self.sample_indices.len() == 4 {
                    let lower_lat = values[0] as f64 * (1.0 - self.lon_weight)
                        + values[1] as f64 * self.lon_weight;
                    let upper_lat = values[2] as f64 * (1.0 - self.lon_weight_upper)
                        + values[3] as f64 * self.lon_weight_upper;
                    return Ok(lower_lat * (1.0 - self.lat_weight) + upper_lat * self.lat_weight);
                }
                let value_at = |lat_offset: usize, lon_offset: usize| -> f64 {
                    let mut offsets = [0usize; 2];
                    offsets[self.lat_axis] = lat_offset;
                    offsets[self.lon_axis] = lon_offset;
                    values[offsets[0] * 2 + offsets[1]] as f64
                };
                let lower_lat =
                    value_at(0, 0) * (1.0 - self.lon_weight) + value_at(0, 1) * self.lon_weight;
                let upper_lat = value_at(1, 0) * (1.0 - self.lon_weight_upper)
                    + value_at(1, 1) * self.lon_weight_upper;
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

fn latitude_position(dimensions: &[u64], lat_axis: usize, bbox: GeoBbox, latitude: f64) -> f64 {
    let max_index = dimensions[lat_axis].saturating_sub(1);
    let ratio = (latitude - bbox.min_lat) / (bbox.max_lat - bbox.min_lat);
    (ratio * max_index as f64).clamp(0.0, max_index as f64)
}

fn longitude_position(dimensions: &[u64], lon_axis: usize, bbox: GeoBbox, longitude: f64) -> f64 {
    let max_index = dimensions[lon_axis].saturating_sub(1);
    let normalized = normalize_longitude(longitude, bbox.min_lon);
    let ratio = (normalized - bbox.min_lon) / (bbox.max_lon - bbox.min_lon);
    (ratio * max_index as f64).clamp(0.0, max_index as f64)
}

fn latitude_at(dimensions: &[u64], lat_axis: usize, bbox: GeoBbox, index: u64) -> f64 {
    let max_index = dimensions[lat_axis].saturating_sub(1);
    bbox.min_lat + index as f64 * ((bbox.max_lat - bbox.min_lat) / max_index as f64)
}

fn longitude_at(dimensions: &[u64], lon_axis: usize, bbox: GeoBbox, index: u64) -> f64 {
    let max_index = dimensions[lon_axis].saturating_sub(1);
    bbox.min_lon + index as f64 * ((bbox.max_lon - bbox.min_lon) / max_index as f64)
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

    fn sample_o1280_grid() -> SpatialGrid {
        SpatialGrid::from_metadata(SpatialGridMetadata {
            dimensions: vec![1, 6_599_680],
            coordinates: "lat lon".to_string(),
            crs_wkt: "GEOGCRS[\"Reduced Gaussian Grid\",REMARK[\"Reduced Gaussian Grid O1280 (ECMWF)\"],USAGE[BBOX[-90,-180.0,90,180]]]"
                .to_string(),
        })
        .expect("gaussian grid")
    }

    fn sample_f768_grid() -> SpatialGrid {
        SpatialGrid::from_metadata(SpatialGridMetadata {
            dimensions: vec![1, 4_718_592],
            coordinates: "lat lon".to_string(),
            crs_wkt: "GEOGCRS[\"Gaussian Grid\",REMARK[\"Gaussian Grid F768 (NCEP)\"],USAGE[BBOX[-89.912125,-180.0,89.912125,179.88281]]]"
                .to_string(),
        })
        .expect("full gaussian grid")
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

    #[test]
    fn full_gaussian_grid_parses_f_number_from_crs_wkt() {
        let grid = sample_f768_grid();
        let window = grid
            .interpolation_window(31.2304, 121.4737)
            .expect("shanghai window");
        assert_eq!(window.sample_indices.len(), 4);
        assert!(window.sample_indices.iter().all(|&idx| idx < 4_718_592));
    }

    #[test]
    fn octahedral_grid_derives_o_number_from_point_count() {
        let grid = SpatialGrid::from_metadata(SpatialGridMetadata {
            dimensions: vec![1, 4 * 640 * (640 + 9)],
            coordinates: "lat lon".to_string(),
            crs_wkt: "GEOGCRS[\"Reduced Gaussian Grid\",USAGE[BBOX[-90,-180,90,180]]]".to_string(),
        })
        .expect("O640 from point count");
        let window = grid
            .interpolation_window(45.0, 10.0)
            .expect("interpolation window");
        assert_eq!(window.sample_indices.len(), 4);
    }

    #[test]
    fn gaussian_interpolation_window_uses_four_neighboring_gridpoints() {
        let grid = sample_o1280_grid();
        let window = grid
            .interpolation_window(31.2304, 121.4737)
            .expect("shanghai window");
        assert_eq!(window.sample_indices.len(), 4);
        assert!(window.sample_indices.iter().all(|&idx| idx < 6_599_680));
        assert!((0.0..=1.0).contains(&window.lat_weight));
        assert!((0.0..=1.0).contains(&window.lon_weight));
        assert!((0.0..=1.0).contains(&window.lon_weight_upper));
        let value = window
            .interpolate(&[10.0, 20.0, 30.0, 40.0])
            .expect("bilinear value");
        assert!((10.0..=40.0).contains(&value));
    }

    #[test]
    fn gaussian_bilinear_interpolation_weights_corner_values() {
        let grid = sample_o1280_grid();
        let window = grid
            .interpolation_window(45.0, 10.0)
            .expect("interpolation window");
        assert_eq!(window.sample_indices.len(), 4);
        let value = window
            .interpolate(&[10.0, 20.0, 30.0, 40.0])
            .expect("interpolate");
        assert!((10.0..=40.0).contains(&value));
    }

    #[test]
    fn gaussian_grid_point_window_uses_nearest_gridpoint() {
        let grid = sample_o1280_grid();
        let interp = grid
            .interpolation_window(45.0, 10.0)
            .expect("interpolation window");
        let point = grid.point_window(45.0, 10.0).expect("point window");
        assert_eq!(point.ranges[0], 0..1);
        assert_eq!(point.ranges[1].end, point.ranges[1].start + 1);
        assert!(interp.sample_indices.contains(&point.ranges[1].start));
    }
}
