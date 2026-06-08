use crate::domain::WeatherElement;

/// `ecmwf_ifs025` spatial manifest (`data_spatial/ecmwf_ifs025/latest.json`, 119 variables).
/// `unused` = exposed via gRPC but not consumed by Snowbuddy yet.
pub(super) const IFS025_SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Cape,           // 对流有效位能 (J/kg), unused
    WeatherElement::CloudCover,     // 总云量 (%)
    WeatherElement::CloudCoverHigh, // 高云量 (%)
    WeatherElement::CloudCoverLow,  // 低云量 (%)
    WeatherElement::CloudCoverMid,  // 中云量 (%)
    WeatherElement::raw("geopotential_height_1000hPa"), // 1000hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_100hPa"), // 100hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_10hPa"), // 10hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_150hPa"), // 150hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_200hPa"), // 200hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_250hPa"), // 250hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_300hPa"), // 300hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_400hPa"), // 400hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_500hPa"), // 500hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_50hPa"), // 50hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_600hPa"), // 600hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_700hPa"), // 700hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_850hPa"), // 850hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_925hPa"), // 925hPa 位势高度 (m), unused
    WeatherElement::raw("ocean_u_current"), // 海流纬向分量 (m/s), unused
    WeatherElement::raw("ocean_v_current"), // 海流经向分量 (m/s), unused
    WeatherElement::Precipitation,  // 降水量 (mm)
    WeatherElement::raw("precipitation_type"), // 降水类型, unused
    WeatherElement::PressureMsl,    // 海平面气压 (hPa), unused
    WeatherElement::raw("relative_humidity_1000hPa"), // 1000hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_100hPa"), // 100hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_10hPa"), // 10hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_150hPa"), // 150hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_200hPa"), // 200hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_250hPa"), // 250hPa 相对湿度 (%), unused
    WeatherElement::RelativeHumidity2m, // 2米相对湿度 (%)
    WeatherElement::raw("relative_humidity_300hPa"), // 300hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_400hPa"), // 400hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_500hPa"), // 500hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_50hPa"), // 50hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_600hPa"), // 600hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_700hPa"), // 700hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_850hPa"), // 850hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_925hPa"), // 925hPa 相对湿度 (%), unused
    WeatherElement::raw("runoff"),  // 径流 (mm), unused
    WeatherElement::raw("sea_ice_thickness"), // 海冰厚度 (m), unused
    WeatherElement::raw("sea_level_height_msl"), // 海平面高度 (m), unused
    WeatherElement::ShortwaveRadiation, // 短波辐射 (W/m²), unused
    WeatherElement::SnowDepth,      // 雪深 (m)
    WeatherElement::raw("snow_depth_water_equivalent"), // 雪深水当量 (mm), unused
    WeatherElement::Snowfall,       // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::SoilMoisture0To7cm, // 土壤湿度 0–7 cm (m³/m³), unused
    WeatherElement::SoilMoisture100To255cm, // 土壤湿度 100–255 cm (m³/m³), unused
    WeatherElement::SoilMoisture28To100cm, // 土壤湿度 28–100 cm (m³/m³), unused
    WeatherElement::SoilMoisture7To28cm, // 土壤湿度 7–28 cm (m³/m³), unused
    WeatherElement::SoilTemperature0To7cm, // 土壤温度 0–7 cm (°C), unused
    WeatherElement::SoilTemperature100To255cm, // 土壤温度 100–255 cm (°C), unused
    WeatherElement::SoilTemperature28To100cm, // 土壤温度 28–100 cm (°C), unused
    WeatherElement::SoilTemperature7To28cm, // 土壤温度 7–28 cm (°C), unused
    WeatherElement::SurfaceTemperature, // 地表/skin 温度 (°C), unused
    WeatherElement::raw("temperature_1000hPa"), // 1000hPa 气温 (°C), unused
    WeatherElement::raw("temperature_100hPa"), // 100hPa 气温 (°C), unused
    WeatherElement::raw("temperature_10hPa"), // 10hPa 气温 (°C), unused
    WeatherElement::raw("temperature_150hPa"), // 150hPa 气温 (°C), unused
    WeatherElement::raw("temperature_200hPa"), // 200hPa 气温 (°C), unused
    WeatherElement::raw("temperature_250hPa"), // 250hPa 气温 (°C), unused
    WeatherElement::Temperature2m,  // 2米气温 (°C)
    WeatherElement::Temperature2mMax, // 2米日最高温 (°C), unused
    WeatherElement::Temperature2mMin, // 2米日最低温 (°C), unused
    WeatherElement::raw("temperature_300hPa"), // 300hPa 气温 (°C), unused
    WeatherElement::raw("temperature_400hPa"), // 400hPa 气温 (°C), unused
    WeatherElement::raw("temperature_500hPa"), // 500hPa 气温 (°C), unused
    WeatherElement::raw("temperature_50hPa"), // 50hPa 气温 (°C), unused
    WeatherElement::raw("temperature_600hPa"), // 600hPa 气温 (°C), unused
    WeatherElement::raw("temperature_700hPa"), // 700hPa 气温 (°C), unused
    WeatherElement::raw("temperature_850hPa"), // 850hPa 气温 (°C), unused
    WeatherElement::raw("temperature_925hPa"), // 925hPa 气温 (°C), unused
    WeatherElement::TotalColumnIntegratedWaterVapour, // 整层可降水量 (kg/m²), unused
    WeatherElement::raw("vertical_velocity_1000hPa"), // 1000hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_100hPa"), // 100hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_10hPa"), // 10hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_150hPa"), // 150hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_200hPa"), // 200hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_250hPa"), // 250hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_300hPa"), // 300hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_400hPa"), // 400hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_500hPa"), // 500hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_50hPa"), // 50hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_600hPa"), // 600hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_700hPa"), // 700hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_850hPa"), // 850hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_925hPa"), // 925hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("wind_u_component_1000hPa"), // 1000hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_100hPa"), // 100hPa 纬向风 (m/s), unused
    WeatherElement::WindUComponent100m, // 100m 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_10hPa"), // 10hPa 纬向风 (m/s), unused
    WeatherElement::WindUComponent10m, // 10m 纬向风 (m/s)
    WeatherElement::raw("wind_u_component_150hPa"), // 150hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_200hPa"), // 200hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_250hPa"), // 250hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_300hPa"), // 300hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_400hPa"), // 400hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_500hPa"), // 500hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_50hPa"), // 50hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_600hPa"), // 600hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_700hPa"), // 700hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_850hPa"), // 850hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_925hPa"), // 925hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_v_component_1000hPa"), // 1000hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_100hPa"), // 100hPa 经向风 (m/s), unused
    WeatherElement::WindVComponent100m, // 100m 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_10hPa"), // 10hPa 经向风 (m/s), unused
    WeatherElement::WindVComponent10m, // 10m 经向风 (m/s)
    WeatherElement::raw("wind_v_component_150hPa"), // 150hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_200hPa"), // 200hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_250hPa"), // 250hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_300hPa"), // 300hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_400hPa"), // 400hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_500hPa"), // 500hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_50hPa"), // 50hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_600hPa"), // 600hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_700hPa"), // 700hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_850hPa"), // 850hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_925hPa"), // 925hPa 经向风 (m/s), unused
];

/// `ecmwf_ifs025` timeseries manifest (`data/ecmwf_ifs025/latest.json`, 114 variables).
pub(super) const IFS025_TIMESERIES_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Cape,           // 对流有效位能 (J/kg), unused
    WeatherElement::CloudCover,     // 总云量 (%)
    WeatherElement::CloudCoverHigh, // 高云量 (%)
    WeatherElement::CloudCoverLow,  // 低云量 (%)
    WeatherElement::CloudCoverMid,  // 中云量 (%)
    WeatherElement::raw("geopotential_height_1000hPa"), // 1000hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_100hPa"), // 100hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_150hPa"), // 150hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_200hPa"), // 200hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_250hPa"), // 250hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_300hPa"), // 300hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_400hPa"), // 400hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_500hPa"), // 500hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_50hPa"), // 50hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_600hPa"), // 600hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_700hPa"), // 700hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_850hPa"), // 850hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_925hPa"), // 925hPa 位势高度 (m), unused
    WeatherElement::raw("ocean_u_current"), // 海流纬向分量 (m/s), unused
    WeatherElement::raw("ocean_v_current"), // 海流经向分量 (m/s), unused
    WeatherElement::Precipitation,  // 降水量 (mm)
    WeatherElement::raw("precipitation_type"), // 降水类型, unused
    WeatherElement::PressureMsl,    // 海平面气压 (hPa), unused
    WeatherElement::raw("relative_humidity_1000hPa"), // 1000hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_100hPa"), // 100hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_150hPa"), // 150hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_200hPa"), // 200hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_250hPa"), // 250hPa 相对湿度 (%), unused
    WeatherElement::RelativeHumidity2m, // 2米相对湿度 (%)
    WeatherElement::raw("relative_humidity_300hPa"), // 300hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_400hPa"), // 400hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_500hPa"), // 500hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_50hPa"), // 50hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_600hPa"), // 600hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_700hPa"), // 700hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_850hPa"), // 850hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_925hPa"), // 925hPa 相对湿度 (%), unused
    WeatherElement::raw("runoff"),  // 径流 (mm), unused
    WeatherElement::raw("sea_ice_thickness"), // 海冰厚度 (m), unused
    WeatherElement::raw("sea_level_height_msl"), // 海平面高度 (m), unused
    WeatherElement::ShortwaveRadiation, // 短波辐射 (W/m²), unused
    WeatherElement::SnowDepth,      // 雪深 (m)
    WeatherElement::raw("snow_depth_water_equivalent"), // 雪深水当量 (mm), unused
    WeatherElement::Snowfall,       // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::SoilMoisture0To7cm, // 土壤湿度 0–7 cm (m³/m³), unused
    WeatherElement::SoilMoisture100To255cm, // 土壤湿度 100–255 cm (m³/m³), unused
    WeatherElement::SoilMoisture28To100cm, // 土壤湿度 28–100 cm (m³/m³), unused
    WeatherElement::SoilMoisture7To28cm, // 土壤湿度 7–28 cm (m³/m³), unused
    WeatherElement::SoilTemperature0To7cm, // 土壤温度 0–7 cm (°C), unused
    WeatherElement::SoilTemperature100To255cm, // 土壤温度 100–255 cm (°C), unused
    WeatherElement::SoilTemperature28To100cm, // 土壤温度 28–100 cm (°C), unused
    WeatherElement::SoilTemperature7To28cm, // 土壤温度 7–28 cm (°C), unused
    WeatherElement::SurfaceTemperature, // 地表/skin 温度 (°C), unused
    WeatherElement::raw("temperature_1000hPa"), // 1000hPa 气温 (°C), unused
    WeatherElement::raw("temperature_100hPa"), // 100hPa 气温 (°C), unused
    WeatherElement::raw("temperature_150hPa"), // 150hPa 气温 (°C), unused
    WeatherElement::raw("temperature_200hPa"), // 200hPa 气温 (°C), unused
    WeatherElement::raw("temperature_250hPa"), // 250hPa 气温 (°C), unused
    WeatherElement::Temperature2m,  // 2米气温 (°C)
    WeatherElement::Temperature2mMax, // 2米日最高温 (°C), unused
    WeatherElement::Temperature2mMin, // 2米日最低温 (°C), unused
    WeatherElement::raw("temperature_300hPa"), // 300hPa 气温 (°C), unused
    WeatherElement::raw("temperature_400hPa"), // 400hPa 气温 (°C), unused
    WeatherElement::raw("temperature_500hPa"), // 500hPa 气温 (°C), unused
    WeatherElement::raw("temperature_50hPa"), // 50hPa 气温 (°C), unused
    WeatherElement::raw("temperature_600hPa"), // 600hPa 气温 (°C), unused
    WeatherElement::raw("temperature_700hPa"), // 700hPa 气温 (°C), unused
    WeatherElement::raw("temperature_850hPa"), // 850hPa 气温 (°C), unused
    WeatherElement::raw("temperature_925hPa"), // 925hPa 气温 (°C), unused
    WeatherElement::TotalColumnIntegratedWaterVapour, // 整层可降水量 (kg/m²), unused
    WeatherElement::raw("vertical_velocity_1000hPa"), // 1000hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_100hPa"), // 100hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_150hPa"), // 150hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_200hPa"), // 200hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_250hPa"), // 250hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_300hPa"), // 300hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_400hPa"), // 400hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_500hPa"), // 500hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_50hPa"), // 50hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_600hPa"), // 600hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_700hPa"), // 700hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_850hPa"), // 850hPa 垂直速度 (Pa/s), unused
    WeatherElement::raw("vertical_velocity_925hPa"), // 925hPa 垂直速度 (Pa/s), unused
    WeatherElement::WindGusts10m,   // 10米阵风 (km/h)
    WeatherElement::raw("wind_u_component_1000hPa"), // 1000hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_100hPa"), // 100hPa 纬向风 (m/s), unused
    WeatherElement::WindUComponent100m, // 100m 纬向风 (m/s), unused
    WeatherElement::WindUComponent10m, // 10m 纬向风 (m/s)
    WeatherElement::raw("wind_u_component_150hPa"), // 150hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_200hPa"), // 200hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_250hPa"), // 250hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_300hPa"), // 300hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_400hPa"), // 400hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_500hPa"), // 500hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_50hPa"), // 50hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_600hPa"), // 600hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_700hPa"), // 700hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_850hPa"), // 850hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_925hPa"), // 925hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_v_component_1000hPa"), // 1000hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_100hPa"), // 100hPa 经向风 (m/s), unused
    WeatherElement::WindVComponent100m, // 100m 经向风 (m/s), unused
    WeatherElement::WindVComponent10m, // 10m 经向风 (m/s)
    WeatherElement::raw("wind_v_component_150hPa"), // 150hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_200hPa"), // 200hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_250hPa"), // 250hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_300hPa"), // 300hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_400hPa"), // 400hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_500hPa"), // 500hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_50hPa"), // 50hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_600hPa"), // 600hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_700hPa"), // 700hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_850hPa"), // 850hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_925hPa"), // 925hPa 经向风 (m/s), unused
];
