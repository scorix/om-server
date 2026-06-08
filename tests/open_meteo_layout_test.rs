use om_server::domain::DataLayout;
use om_server::infrastructure::open_meteo::{OpenMeteoSpatialLayout, OpenMeteoTimeseriesLayout};

#[test]
fn data_layouts_map_to_open_meteo_s3_roots() {
    assert_eq!(DataLayout::Spatial.s3_root(), "data_spatial");
    assert_eq!(DataLayout::Timeseries.s3_root(), "data");
}

#[test]
fn builds_spatial_and_timeseries_object_keys() {
    let spatial = OpenMeteoSpatialLayout::ECMWF_IFS025
        .object_key("0000Z", "2024-02-03T0000")
        .expect("spatial key");
    assert_eq!(
        spatial.0,
        "data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om"
    );

    let timeseries =
        OpenMeteoTimeseriesLayout::ECMWF_IFS025.object_key("temperature_2m", "chunk_1523");
    assert_eq!(
        timeseries.0,
        "data/ecmwf_ifs025/temperature_2m/chunk_1523.om"
    );
}
