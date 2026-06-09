mod common;

use om_server::domain::WeatherBakeLayer;
use om_server::domain::weather_field::SpatialFieldRegridder;
use om_server::infrastructure::tile_index::global_tile_coords;
use om_server::infrastructure::weather_tile_renderer::ScalarWeatherTileRenderer;

#[test]
fn renders_global_z4_tile_png_from_fixture() {
    let bytes = common::write_sample_spatial_om().expect("fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("sample.om");
    std::fs::write(&path, bytes).expect("write fixture");

    let field = SpatialFieldRegridder::from_spatial_file(&path, "temperature_2m").expect("regrid");
    let renderer = ScalarWeatherTileRenderer::new(WeatherBakeLayer::Temperature2m, &field);
    let png = renderer.render_tile_png(4, 8, 8).expect("render tile");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(png.len() > 100);
}

#[test]
fn global_tile_index_z0_to_z4_count() {
    assert_eq!(global_tile_coords(0, 4).len(), 341);
}
