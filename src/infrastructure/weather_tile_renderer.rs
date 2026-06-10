use std::f64::consts::PI;

use image::{ImageBuffer, Rgba};

use crate::domain::WeatherBakeLayer;
use crate::domain::weather_colormap::{encode_value_gray, wind_particle_rgba};
use crate::domain::weather_field::RegularLatLonField;
use crate::domain::{TileRequest, WeatherTileRenderer};
use crate::error::TileRenderError;

pub const TILE_SIZE: u32 = 256;
const WEB_MERCATOR_MAX_LAT: f64 = 85.051_128_78;

pub struct ScalarWeatherTileRenderer<'a> {
    layer: WeatherBakeLayer,
    field: &'a RegularLatLonField,
}

impl<'a> ScalarWeatherTileRenderer<'a> {
    pub fn new(layer: WeatherBakeLayer, field: &'a RegularLatLonField) -> Self {
        Self { layer, field }
    }

    fn sample_rgba(&self, lat: f64, lon: f64) -> [u8; 4] {
        let Some(range) = self.layer.value_range() else {
            return [0, 0, 0, 0];
        };
        encode_value_gray(self.field.sample(lat, lon), range)
    }

    pub fn render_tile_png(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, TileRenderError> {
        render_rgba_tile(|lat, lon| self.sample_rgba(lat, lon), z, x, y)
    }
}

pub struct WindWeatherTileRenderer<'a> {
    u_field: &'a RegularLatLonField,
    v_field: &'a RegularLatLonField,
}

impl<'a> WindWeatherTileRenderer<'a> {
    pub fn new(u_field: &'a RegularLatLonField, v_field: &'a RegularLatLonField) -> Self {
        Self { u_field, v_field }
    }

    pub fn render_tile_png(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, TileRenderError> {
        render_rgba_tile(
            |lat, lon| {
                let u = self.u_field.sample(lat, lon);
                let v = self.v_field.sample(lat, lon);
                match (u, v) {
                    (Some(u), Some(v)) => wind_particle_rgba(u, v),
                    _ => [0, 0, 0, 0],
                }
            },
            z,
            x,
            y,
        )
    }
}

impl WeatherTileRenderer for ScalarWeatherTileRenderer<'_> {
    fn render(
        &self,
        _dataset: &crate::domain::DatasetMeta,
        request: &TileRequest,
    ) -> Result<Vec<u8>, TileRenderError> {
        self.render_tile_png(request.z, request.x, request.y)
    }
}

fn render_rgba_tile(
    sample: impl Fn(f64, f64) -> [u8; 4],
    z: u8,
    x: u32,
    y: u32,
) -> Result<Vec<u8>, TileRenderError> {
    let bounds = tile_bounds(z, x, y);
    let mut image = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(TILE_SIZE, TILE_SIZE);
    for pixel_y in 0..TILE_SIZE {
        // Pixel rows are evenly spaced in web-mercator Y (not in latitude). Tile boundaries are
        // already mercator-correct, but interpolating latitude linearly inside the tile stretches
        // the field vertically (worst at low zoom / high latitude). Clients sample these tiles as
        // standard web-mercator rasters, so the data must be baked the same way.
        let lat = lat_at_pixel(z, y, pixel_y);
        for pixel_x in 0..TILE_SIZE {
            let lon = lng_at_pixel(bounds, pixel_x);
            image.put_pixel(pixel_x, pixel_y, Rgba(sample(lat, lon)));
        }
    }
    let mut bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut bytes);
    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|source| TileRenderError::EncodePng { source })?;
    Ok(bytes)
}

#[derive(Debug, Clone, Copy)]
pub struct TileBounds {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

pub fn tile_bounds(z: u8, x: u32, y: u32) -> TileBounds {
    let scale = 2f64.powi(i32::from(z));
    let west = x as f64 / scale * 360.0 - 180.0;
    let east = (x + 1) as f64 / scale * 360.0 - 180.0;
    let north = lat_from_tile_y(y as f64 / scale);
    let south = lat_from_tile_y((y + 1) as f64 / scale);
    TileBounds {
        west,
        south,
        east,
        north,
    }
}

fn lng_at_pixel(bounds: TileBounds, pixel_x: u32) -> f64 {
    let t = (f64::from(pixel_x) + 0.5) / f64::from(TILE_SIZE);
    bounds.west + (bounds.east - bounds.west) * t
}

fn lat_at_pixel(z: u8, y: u32, pixel_y: u32) -> f64 {
    let scale = 2f64.powi(i32::from(z));
    let global_y = (f64::from(y) + (f64::from(pixel_y) + 0.5) / f64::from(TILE_SIZE)) / scale;
    lat_from_tile_y(global_y)
}

fn lat_from_tile_y(y: f64) -> f64 {
    let n = PI * (1.0 - 2.0 * y);
    let lat = (n.sinh()).atan().to_degrees();
    lat.clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_rows_are_mercator_spaced_not_latitude_spaced() {
        // z0 single world tile. Pixel rows must be evenly spaced in web-mercator Y, which means the
        // latitude step shrinks toward the poles. If rows were latitude-linear the steps would all
        // be equal, stretching the field vertically when clients sample it as a mercator raster.
        let equator_step = (lat_at_pixel(0, 0, 128) - lat_at_pixel(0, 0, 127)).abs();
        let pole_step = (lat_at_pixel(0, 0, 1) - lat_at_pixel(0, 0, 0)).abs();
        assert!(
            equator_step > pole_step * 2.0,
            "expected mercator spacing (equator_step={equator_step}, pole_step={pole_step})"
        );
    }

    #[test]
    fn tile_pixels_align_with_neighbour_tile() {
        // The last pixel row of a tile and the first pixel row of the tile directly below it should
        // be one pixel apart in the global mercator grid, i.e. continuous across the seam.
        let z = 3u8;
        let bottom_of_top = lat_at_pixel(z, 2, TILE_SIZE - 1);
        let top_of_bottom = lat_at_pixel(z, 3, 0);
        let one_pixel =
            (lat_at_pixel(z, 2, TILE_SIZE - 1) - lat_at_pixel(z, 2, TILE_SIZE - 2)).abs();
        assert!((bottom_of_top - top_of_bottom).abs() < one_pixel * 1.5);
    }

    #[test]
    fn tile_center_pixels_match_tile_bounds() {
        // The pixel-center latitudes at the tile edges must stay inside the mercator tile bounds.
        let z = 4u8;
        let y = 5u32;
        let bounds = tile_bounds(z, 0, y);
        let first = lat_at_pixel(z, y, 0);
        let last = lat_at_pixel(z, y, TILE_SIZE - 1);
        assert!(first <= bounds.north && first > bounds.south);
        assert!(last >= bounds.south && last < bounds.north);
    }
}
