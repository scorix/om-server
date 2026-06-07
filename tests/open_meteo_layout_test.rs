use om_server::domain::DataLayout;
use om_server::infrastructure::open_meteo::{
    OpenMeteoRunLayout, OpenMeteoSpatialLayout, OpenMeteoTimeseriesLayout,
};

#[test]
fn data_layouts_map_to_open_meteo_s3_roots() {
    assert_eq!(DataLayout::Spatial.s3_root(), "data_spatial");
    assert_eq!(DataLayout::Timeseries.s3_root(), "data");
    assert_eq!(DataLayout::Run.s3_root(), "data_run");
}

#[test]
fn builds_all_layout_object_keys() {
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

    let run = OpenMeteoRunLayout::ECMWF_IFS025.object_key_in_prefix(
        "data_run/ecmwf_ifs025/2026/06/07/0000Z/",
        "temperature_2m",
    );
    assert_eq!(
        run.0,
        "data_run/ecmwf_ifs025/2026/06/07/0000Z/temperature_2m.om"
    );
}
