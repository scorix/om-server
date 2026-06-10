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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_z0_to_z4_has_341_tiles() {
        assert_eq!(global_tile_coords(0, 4).len(), 341);
    }
}
