use crate::domain::WeatherElement;

/// `dwd_icon` spatial manifest (`data_spatial/dwd_icon/latest.json`, 123 variables).
/// `unused` = exposed via gRPC but not consumed by Snowbuddy yet.
pub(super) const DWD_SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Cape,                               // 对流有效位能 (J/kg)
    WeatherElement::CloudCover,                         // 总云量 (%)
    WeatherElement::CloudCoverHigh,                     // 高云量 (%)
    WeatherElement::CloudCoverLow,                      // 低云量 (%)
    WeatherElement::CloudCoverMid,                      // 中云量 (%)
    WeatherElement::raw("convective_cloud_base"),       // 对流云底高度 (m), unused
    WeatherElement::raw("convective_cloud_top"),        // 对流云顶高度 (m), unused
    WeatherElement::raw("diffuse_radiation"),           // 散射太阳辐射 (W/m²), unused
    WeatherElement::DirectRadiation,                    // 直射太阳辐射 (W/m²), unused
    WeatherElement::FreezingLevelHeight,                // 冻结层高度 (m)
    WeatherElement::raw("geopotential_height_1000hPa"), // 1000hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_100hPa"),  // 100hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_150hPa"),  // 150hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_200hPa"),  // 200hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_250hPa"),  // 250hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_300hPa"),  // 300hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_30hPa"),   // 30hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_400hPa"),  // 400hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_500hPa"),  // 500hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_50hPa"),   // 50hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_600hPa"),  // 600hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_700hPa"),  // 700hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_70hPa"),   // 70hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_800hPa"),  // 800hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_850hPa"),  // 850hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_900hPa"),  // 900hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_925hPa"),  // 925hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_950hPa"),  // 950hPa 位势高度 (m), unused
    WeatherElement::raw("latent_heat_flux"),            // 潜热通量 (W/m²), unused
    WeatherElement::Precipitation,                      // 降水量 (mm)
    WeatherElement::PressureMsl,                        // 海平面气压 (hPa), unused
    WeatherElement::Rain,                               // 降雨量 (mm)
    WeatherElement::raw("relative_humidity_1000hPa"),   // 1000hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_100hPa"),    // 100hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_150hPa"),    // 150hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_200hPa"),    // 200hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_250hPa"),    // 250hPa 相对湿度 (%), unused
    WeatherElement::RelativeHumidity2m,                 // 2米相对湿度 (%)
    WeatherElement::raw("relative_humidity_300hPa"),    // 300hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_30hPa"),     // 30hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_400hPa"),    // 400hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_500hPa"),    // 500hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_50hPa"),     // 50hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_600hPa"),    // 600hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_700hPa"),    // 700hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_70hPa"),     // 70hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_800hPa"),    // 800hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_850hPa"),    // 850hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_900hPa"),    // 900hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_925hPa"),    // 925hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_950hPa"),    // 950hPa 相对湿度 (%), unused
    WeatherElement::raw("sensible_heat_flux"),          // 感热通量 (W/m²), unused
    WeatherElement::Showers,                            // 阵雨量 (mm), unused
    WeatherElement::SnowDepth,                          // 雪深 (m)
    WeatherElement::Snowfall, // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::raw("soil_moisture_0_to_1cm"), // 土壤湿度 0_to_1cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_1_to_3cm"), // 土壤湿度 1_to_3cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_27_to_81cm"), // 土壤湿度 27_to_81cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_3_to_9cm"), // 土壤湿度 3_to_9cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_9_to_27cm"), // 土壤湿度 9_to_27cm (m³/m³), unused
    WeatherElement::raw("soil_temperature_0cm"), // 土壤温度 0cm (°C), unused
    WeatherElement::raw("soil_temperature_18cm"), // 土壤温度 18cm (°C), unused
    WeatherElement::raw("soil_temperature_54cm"), // 土壤温度 54cm (°C), unused
    WeatherElement::raw("soil_temperature_6cm"), // 土壤温度 6cm (°C), unused
    WeatherElement::raw("temperature_1000hPa"), // 1000hPa 气温 (°C), unused
    WeatherElement::raw("temperature_100hPa"), // 100hPa 气温 (°C), unused
    WeatherElement::raw("temperature_150hPa"), // 150hPa 气温 (°C), unused
    WeatherElement::raw("temperature_200hPa"), // 200hPa 气温 (°C), unused
    WeatherElement::raw("temperature_250hPa"), // 250hPa 气温 (°C), unused
    WeatherElement::Temperature2m, // 2米气温 (°C)
    WeatherElement::raw("temperature_300hPa"), // 300hPa 气温 (°C), unused
    WeatherElement::raw("temperature_30hPa"), // 30hPa 气温 (°C), unused
    WeatherElement::raw("temperature_400hPa"), // 400hPa 气温 (°C), unused
    WeatherElement::raw("temperature_500hPa"), // 500hPa 气温 (°C), unused
    WeatherElement::raw("temperature_50hPa"), // 50hPa 气温 (°C), unused
    WeatherElement::raw("temperature_600hPa"), // 600hPa 气温 (°C), unused
    WeatherElement::raw("temperature_700hPa"), // 700hPa 气温 (°C), unused
    WeatherElement::raw("temperature_70hPa"), // 70hPa 气温 (°C), unused
    WeatherElement::raw("temperature_800hPa"), // 800hPa 气温 (°C), unused
    WeatherElement::raw("temperature_850hPa"), // 850hPa 气温 (°C), unused
    WeatherElement::raw("temperature_900hPa"), // 900hPa 气温 (°C), unused
    WeatherElement::raw("temperature_925hPa"), // 925hPa 气温 (°C), unused
    WeatherElement::raw("temperature_950hPa"), // 950hPa 气温 (°C), unused
    WeatherElement::WeatherCode, // 天气代码 (WMO)
    WeatherElement::WindGusts10m, // 10米阵风 (km/h)
    WeatherElement::raw("wind_u_component_1000hPa"), // 1000hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_100hPa"), // 100hPa 纬向风 (m/s), unused
    WeatherElement::WindUComponent10m, // 10米纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_150hPa"), // 150hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_200hPa"), // 200hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_250hPa"), // 250hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_300hPa"), // 300hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_30hPa"), // 30hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_400hPa"), // 400hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_500hPa"), // 500hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_50hPa"), // 50hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_600hPa"), // 600hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_700hPa"), // 700hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_70hPa"), // 70hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_800hPa"), // 800hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_850hPa"), // 850hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_900hPa"), // 900hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_925hPa"), // 925hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_950hPa"), // 950hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_v_component_1000hPa"), // 1000hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_100hPa"), // 100hPa 经向风 (m/s), unused
    WeatherElement::WindVComponent10m, // 10米经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_150hPa"), // 150hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_200hPa"), // 200hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_250hPa"), // 250hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_300hPa"), // 300hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_30hPa"), // 30hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_400hPa"), // 400hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_500hPa"), // 500hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_50hPa"), // 50hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_600hPa"), // 600hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_700hPa"), // 700hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_70hPa"), // 70hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_800hPa"), // 800hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_850hPa"), // 850hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_900hPa"), // 900hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_925hPa"), // 925hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_950hPa"), // 950hPa 经向风 (m/s), unused
];

/// `dwd_icon` timeseries manifest (`data/dwd_icon/latest.json`, 134 variables).
/// `unused` = exposed via gRPC but not consumed by Snowbuddy yet.
pub(super) const DWD_TIMESERIES_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Cape,                               // 对流有效位能 (J/kg)
    WeatherElement::CloudCover,                         // 总云量 (%)
    WeatherElement::CloudCoverHigh,                     // 高云量 (%)
    WeatherElement::CloudCoverLow,                      // 低云量 (%)
    WeatherElement::CloudCoverMid,                      // 中云量 (%)
    WeatherElement::raw("convective_cloud_base"),       // 对流云底高度 (m), unused
    WeatherElement::raw("convective_cloud_top"),        // 对流云顶高度 (m), unused
    WeatherElement::raw("diffuse_radiation"),           // 散射太阳辐射 (W/m²), unused
    WeatherElement::DirectRadiation,                    // 直射太阳辐射 (W/m²), unused
    WeatherElement::FreezingLevelHeight,                // 冻结层高度 (m)
    WeatherElement::raw("geopotential_height_1000hPa"), // 1000hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_100hPa"),  // 100hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_150hPa"),  // 150hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_200hPa"),  // 200hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_250hPa"),  // 250hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_300hPa"),  // 300hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_30hPa"),   // 30hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_400hPa"),  // 400hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_500hPa"),  // 500hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_50hPa"),   // 50hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_600hPa"),  // 600hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_700hPa"),  // 700hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_70hPa"),   // 70hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_800hPa"),  // 800hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_850hPa"),  // 850hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_900hPa"),  // 900hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_925hPa"),  // 925hPa 位势高度 (m), unused
    WeatherElement::raw("geopotential_height_950hPa"),  // 950hPa 位势高度 (m), unused
    WeatherElement::raw("latent_heat_flux"),            // 潜热通量 (W/m²), unused
    WeatherElement::Precipitation,                      // 降水量 (mm)
    WeatherElement::PressureMsl,                        // 海平面气压 (hPa), unused
    WeatherElement::Rain,                               // 降雨量 (mm)
    WeatherElement::raw("relative_humidity_1000hPa"),   // 1000hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_100hPa"),    // 100hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_150hPa"),    // 150hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_200hPa"),    // 200hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_250hPa"),    // 250hPa 相对湿度 (%), unused
    WeatherElement::RelativeHumidity2m,                 // 2米相对湿度 (%)
    WeatherElement::raw("relative_humidity_300hPa"),    // 300hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_30hPa"),     // 30hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_400hPa"),    // 400hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_500hPa"),    // 500hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_50hPa"),     // 50hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_600hPa"),    // 600hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_700hPa"),    // 700hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_70hPa"),     // 70hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_800hPa"),    // 800hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_850hPa"),    // 850hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_900hPa"),    // 900hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_925hPa"),    // 925hPa 相对湿度 (%), unused
    WeatherElement::raw("relative_humidity_950hPa"),    // 950hPa 相对湿度 (%), unused
    WeatherElement::raw("sensible_heat_flux"),          // 感热通量 (W/m²), unused
    WeatherElement::Showers,                            // 阵雨量 (mm), unused
    WeatherElement::SnowDepth,                          // 雪深 (m)
    WeatherElement::raw("snowfall_convective_water_equivalent"), // snowfall convective water equivalent, unused
    WeatherElement::Snowfall, // 降雪 (cm; S3: snowfall_water_equivalent mm)
    WeatherElement::raw("soil_moisture_0_to_1cm"), // 土壤湿度 0_to_1cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_1_to_3cm"), // 土壤湿度 1_to_3cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_27_to_81cm"), // 土壤湿度 27_to_81cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_3_to_9cm"), // 土壤湿度 3_to_9cm (m³/m³), unused
    WeatherElement::raw("soil_moisture_9_to_27cm"), // 土壤湿度 9_to_27cm (m³/m³), unused
    WeatherElement::raw("soil_temperature_0cm"), // 土壤温度 0cm (°C), unused
    WeatherElement::raw("soil_temperature_18cm"), // 土壤温度 18cm (°C), unused
    WeatherElement::raw("soil_temperature_54cm"), // 土壤温度 54cm (°C), unused
    WeatherElement::raw("soil_temperature_6cm"), // 土壤温度 6cm (°C), unused
    WeatherElement::raw("static"), // 静态场, unused
    WeatherElement::raw("temperature_1000hPa"), // 1000hPa 气温 (°C), unused
    WeatherElement::raw("temperature_100hPa"), // 100hPa 气温 (°C), unused
    WeatherElement::raw("temperature_120m"), // 120m 气温 (°C), unused
    WeatherElement::raw("temperature_150hPa"), // 150hPa 气温 (°C), unused
    WeatherElement::raw("temperature_180m"), // 180m 气温 (°C), unused
    WeatherElement::raw("temperature_200hPa"), // 200hPa 气温 (°C), unused
    WeatherElement::raw("temperature_250hPa"), // 250hPa 气温 (°C), unused
    WeatherElement::Temperature2m, // 2米气温 (°C)
    WeatherElement::raw("temperature_300hPa"), // 300hPa 气温 (°C), unused
    WeatherElement::raw("temperature_30hPa"), // 30hPa 气温 (°C), unused
    WeatherElement::raw("temperature_400hPa"), // 400hPa 气温 (°C), unused
    WeatherElement::raw("temperature_500hPa"), // 500hPa 气温 (°C), unused
    WeatherElement::raw("temperature_50hPa"), // 50hPa 气温 (°C), unused
    WeatherElement::raw("temperature_600hPa"), // 600hPa 气温 (°C), unused
    WeatherElement::raw("temperature_700hPa"), // 700hPa 气温 (°C), unused
    WeatherElement::raw("temperature_70hPa"), // 70hPa 气温 (°C), unused
    WeatherElement::raw("temperature_800hPa"), // 800hPa 气温 (°C), unused
    WeatherElement::raw("temperature_80m"), // 80m 气温 (°C), unused
    WeatherElement::raw("temperature_850hPa"), // 850hPa 气温 (°C), unused
    WeatherElement::raw("temperature_900hPa"), // 900hPa 气温 (°C), unused
    WeatherElement::raw("temperature_925hPa"), // 925hPa 气温 (°C), unused
    WeatherElement::raw("temperature_950hPa"), // 950hPa 气温 (°C), unused
    WeatherElement::WeatherCode, // 天气代码 (WMO)
    WeatherElement::WindGusts10m, // 10米阵风 (km/h)
    WeatherElement::raw("wind_u_component_1000hPa"), // 1000hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_100hPa"), // 100hPa 纬向风 (m/s), unused
    WeatherElement::WindUComponent10m, // 10米纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_120m"), // 120m 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_150hPa"), // 150hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_180m"), // 180m 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_200hPa"), // 200hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_250hPa"), // 250hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_300hPa"), // 300hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_30hPa"), // 30hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_400hPa"), // 400hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_500hPa"), // 500hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_50hPa"), // 50hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_600hPa"), // 600hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_700hPa"), // 700hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_70hPa"), // 70hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_800hPa"), // 800hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_80m"), // 80m 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_850hPa"), // 850hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_900hPa"), // 900hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_925hPa"), // 925hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_u_component_950hPa"), // 950hPa 纬向风 (m/s), unused
    WeatherElement::raw("wind_v_component_1000hPa"), // 1000hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_100hPa"), // 100hPa 经向风 (m/s), unused
    WeatherElement::WindVComponent10m, // 10米经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_120m"), // 120m 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_150hPa"), // 150hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_180m"), // 180m 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_200hPa"), // 200hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_250hPa"), // 250hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_300hPa"), // 300hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_30hPa"), // 30hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_400hPa"), // 400hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_500hPa"), // 500hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_50hPa"), // 50hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_600hPa"), // 600hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_700hPa"), // 700hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_70hPa"), // 70hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_800hPa"), // 800hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_80m"), // 80m 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_850hPa"), // 850hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_900hPa"), // 900hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_925hPa"), // 925hPa 经向风 (m/s), unused
    WeatherElement::raw("wind_v_component_950hPa"), // 950hPa 经向风 (m/s), unused
];
