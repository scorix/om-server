use crate::domain::WeatherElement;

/// `ecmwf_ifs` spatial manifest (`data_spatial/ecmwf_ifs/latest.json`, 35 variables).
/// `unused` = exposed via gRPC but not consumed by Snowbuddy yet.
pub(super) const IFS_SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::BoundaryLayerHeight,    // 边界层高度 (m), unused
    WeatherElement::Cape,                   // 对流有效位能 (J/kg), unused
    WeatherElement::CloudCover,             // 总云量 (%)
    WeatherElement::CloudCoverHigh,         // 高云量 (%), unused
    WeatherElement::CloudCoverLow,          // 低云量 (%), unused
    WeatherElement::CloudCoverMid,          // 中云量 (%), unused
    WeatherElement::ConvectiveInhibition,   // 对流抑制 (J/kg), unused
    WeatherElement::DewPoint2m,             // 2米露点温度 (°C)
    WeatherElement::DirectRadiation,        // 直射太阳辐射 (W/m²), unused
    WeatherElement::Precipitation,          // 降水量 (mm)
    WeatherElement::PressureMsl,            // 海平面气压 (hPa), unused
    WeatherElement::ShortwaveRadiation,     // 短波辐射 (W/m²), unused
    WeatherElement::Showers,                // 阵雨量 (mm), unused
    WeatherElement::Snowfall,               // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::SoilMoisture0To7cm,     // 土壤湿度 0–7 cm (m³/m³), unused
    WeatherElement::SoilMoisture7To28cm,    // 土壤湿度 7–28 cm (m³/m³), unused
    WeatherElement::SoilMoisture28To100cm,  // 土壤湿度 28–100 cm (m³/m³), unused
    WeatherElement::SoilMoisture100To255cm, // 土壤湿度 100–255 cm (m³/m³), unused
    WeatherElement::SoilTemperature0To7cm,  // 土壤温度 0–7 cm (°C), unused
    WeatherElement::SoilTemperature7To28cm, // 土壤温度 7–28 cm (°C), unused
    WeatherElement::SoilTemperature28To100cm, // 土壤温度 28–100 cm (°C), unused
    WeatherElement::SoilTemperature100To255cm, // 土壤温度 100–255 cm (°C), unused
    WeatherElement::SurfaceTemperature,     // 地表/skin 温度 (°C), unused
    WeatherElement::Temperature2m,          // 2米气温 (°C)
    WeatherElement::Temperature2mMax,       // 2米日最高温 (°C), unused
    WeatherElement::Temperature2mMin,       // 2米日最低温 (°C), unused
    WeatherElement::TotalColumnIntegratedWaterVapour, // 整层可降水量 (kg/m²), unused
    WeatherElement::Visibility,             // 能见度 (m)
    WeatherElement::WindGusts10m,           // 10米阵风 (km/h)
    WeatherElement::WindUComponent100m,     // 100米纬向风 (m/s), unused
    WeatherElement::WindUComponent10m,      // 10米纬向风 (m/s)
    WeatherElement::WindUComponent200m,     // 200米纬向风 (m/s), unused
    WeatherElement::WindVComponent100m,     // 100米经向风 (m/s), unused
    WeatherElement::WindVComponent10m,      // 10米经向风 (m/s)
    WeatherElement::WindVComponent200m,     // 200米经向风 (m/s), unused
];

/// `ecmwf_ifs` timeseries manifest (`data/ecmwf_ifs/latest.json`, 36 variables).
pub(super) const IFS_TIMESERIES_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::BoundaryLayerHeight,    // 边界层高度 (m), unused
    WeatherElement::Cape,                   // 对流有效位能 (J/kg), unused
    WeatherElement::CloudCover,             // 总云量 (%)
    WeatherElement::CloudCoverHigh,         // 高云量 (%), unused
    WeatherElement::CloudCoverLow,          // 低云量 (%), unused
    WeatherElement::CloudCoverMid,          // 中云量 (%), unused
    WeatherElement::ConvectiveInhibition,   // 对流抑制 (J/kg), unused
    WeatherElement::DewPoint2m,             // 2米露点温度 (°C)
    WeatherElement::DirectRadiation,        // 直射太阳辐射 (W/m²), unused
    WeatherElement::Precipitation,          // 降水量 (mm)
    WeatherElement::PressureMsl,            // 海平面气压 (hPa), unused
    WeatherElement::RoughnessLength,        // 地表粗糙度长度 (m), unused
    WeatherElement::ShortwaveRadiation,     // 短波辐射 (W/m²), unused
    WeatherElement::Showers,                // 阵雨量 (mm), unused
    WeatherElement::Snowfall,               // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::SoilMoisture0To7cm,     // 土壤湿度 0–7 cm (m³/m³), unused
    WeatherElement::SoilMoisture7To28cm,    // 土壤湿度 7–28 cm (m³/m³), unused
    WeatherElement::SoilMoisture28To100cm,  // 土壤湿度 28–100 cm (m³/m³), unused
    WeatherElement::SoilMoisture100To255cm, // 土壤湿度 100–255 cm (m³/m³), unused
    WeatherElement::SoilTemperature0To7cm,  // 土壤温度 0–7 cm (°C), unused
    WeatherElement::SoilTemperature7To28cm, // 土壤温度 7–28 cm (°C), unused
    WeatherElement::SoilTemperature28To100cm, // 土壤温度 28–100 cm (°C), unused
    WeatherElement::SoilTemperature100To255cm, // 土壤温度 100–255 cm (°C), unused
    WeatherElement::SurfaceTemperature,     // 地表/skin 温度 (°C), unused
    WeatherElement::Temperature2m,          // 2米气温 (°C)
    WeatherElement::Temperature2mMax,       // 2米日最高温 (°C), unused
    WeatherElement::Temperature2mMin,       // 2米日最低温 (°C), unused
    WeatherElement::TotalColumnIntegratedWaterVapour, // 整层可降水量 (kg/m²), unused
    WeatherElement::Visibility,             // 能见度 (m)
    WeatherElement::WindGusts10m,           // 10米阵风 (km/h)
    WeatherElement::WindUComponent100m,     // 100米纬向风 (m/s), unused
    WeatherElement::WindUComponent10m,      // 10米纬向风 (m/s)
    WeatherElement::WindUComponent200m,     // 200米纬向风 (m/s), unused
    WeatherElement::WindVComponent100m,     // 100米经向风 (m/s), unused
    WeatherElement::WindVComponent10m,      // 10米经向风 (m/s)
    WeatherElement::WindVComponent200m,     // 200米经向风 (m/s), unused
];
