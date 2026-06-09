use std::f64::consts::PI;

use image::{ImageBuffer, Rgba};

use crate::domain::WeatherBakeLayer;
use crate::domain::weather_colormap::{
    cloud_cover_rgba, snow_depth_rgba, snowfall_rgba, temperature_rgba, wind_particle_rgba,
};
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
        let Some(value) = self.field.sample(lat, lon) else {
            return [0, 0, 0, 0];
        };
        match self.layer {
            WeatherBakeLayer::Temperature2m => temperature_rgba(value),
            WeatherBakeLayer::CloudCover => cloud_cover_rgba(value),
            WeatherBakeLayer::Snowfall => snowfall_rgba(value),
            WeatherBakeLayer::SnowDepth => snow_depth_rgba(value),
            WeatherBakeLayer::Wind => [0, 0, 0, 0],
        }
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
        for pixel_x in 0..TILE_SIZE {
            let lon = lng_at_pixel(bounds, pixel_x);
            let lat = lat_at_pixel(bounds, pixel_y);
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
    west: f64,
    south: f64,
    east: f64,
    north: f64,
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

fn lat_at_pixel(bounds: TileBounds, pixel_y: u32) -> f64 {
    let t = (f64::from(pixel_y) + 0.5) / f64::from(TILE_SIZE);
    bounds.north + (bounds.south - bounds.north) * t
}

fn lat_from_tile_y(y: f64) -> f64 {
    let n = PI * (1.0 - 2.0 * y);
    let lat = (n.sinh()).atan().to_degrees();
    lat.clamp(-WEB_MERCATOR_MAX_LAT, WEB_MERCATOR_MAX_LAT)
}
