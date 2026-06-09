use std::collections::BTreeSet;

use crate::domain::GeoBoundingBox;

pub type TileCoord = (u8, u32, u32);

pub fn global_tile_coords(min_zoom: u8, max_zoom: u8) -> Vec<TileCoord> {
    let mut coords = Vec::new();
    for z in min_zoom..=max_zoom {
        let count = 1u32 << z;
        for x in 0..count {
            for y in 0..count {
                coords.push((z, x, y));
            }
        }
    }
    coords
}

pub fn regional_tile_coords(
    coverages: &[GeoBoundingBox],
    min_zoom: u8,
    max_zoom: u8,
) -> Vec<TileCoord> {
    let mut coords = BTreeSet::new();
    for bbox in coverages {
        for z in min_zoom..=max_zoom {
            for (x, y) in tile_coordinates_for_bounds(z, bbox) {
                coords.insert((z, x, y));
            }
        }
    }
    coords.into_iter().collect()
}

pub fn build_weather_tile_index(
    global_max_zoom: u8,
    regional_min_zoom: u8,
    regional_max_zoom: u8,
    coverages: &[GeoBoundingBox],
) -> Vec<TileCoord> {
    let mut coords: BTreeSet<TileCoord> =
        global_tile_coords(0, global_max_zoom).into_iter().collect();
    for coord in regional_tile_coords(coverages, regional_min_zoom, regional_max_zoom) {
        coords.insert(coord);
    }
    coords.into_iter().collect()
}

fn tile_coordinates_for_bounds(z: u8, bbox: &GeoBoundingBox) -> Vec<(u32, u32)> {
    let tiles = 1u32 << z;
    let min_x = tile_x_for_lon(bbox.west.min(bbox.east), tiles);
    let max_x = tile_x_for_lon(bbox.west.max(bbox.east), tiles);
    let min_y = tile_y_for_lat(bbox.north.max(bbox.south), tiles);
    let max_y = tile_y_for_lat(bbox.north.min(bbox.south), tiles);
    let mut coordinates = Vec::new();
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            coordinates.push((x, y));
        }
    }
    coordinates
}

fn tile_x_for_lon(lon: f64, tiles: u32) -> u32 {
    let world_x = ((lon + 180.0) / 360.0).clamp(0.0, 1.0 - f64::EPSILON);
    (world_x * f64::from(tiles)).floor() as u32
}

fn tile_y_for_lat(lat: f64, tiles: u32) -> u32 {
    let lat = lat.clamp(-85.051_128_78, 85.051_128_78).to_radians();
    let world_y =
        ((1.0 - ((lat.tan() + (1.0 / lat.cos())).ln() / PI)) / 2.0).clamp(0.0, 1.0 - f64::EPSILON);
    (world_y * f64::from(tiles)).floor() as u32
}

const PI: f64 = std::f64::consts::PI;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_z0_to_z4_has_341_tiles() {
        assert_eq!(global_tile_coords(0, 4).len(), 341);
    }
}
