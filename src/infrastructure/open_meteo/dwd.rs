use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[derive(Debug, Default)]
pub struct DwdIconSource;

const SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Temperature2m,
    WeatherElement::RelativeHumidity2m,
    WeatherElement::Precipitation,
    WeatherElement::Rain,
    WeatherElement::Snowfall,
    WeatherElement::SnowDepth,
    WeatherElement::FreezingLevelHeight,
    WeatherElement::WeatherCode,
    WeatherElement::WindGusts10m,
    WeatherElement::CloudCover,
    WeatherElement::CloudCoverLow,
    WeatherElement::CloudCoverMid,
    WeatherElement::CloudCoverHigh,
    WeatherElement::Cape,
];

const TIMESERIES_ELEMENTS: &[WeatherElement] = SPATIAL_ELEMENTS;

impl WeatherDataSource for DwdIconSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::DwdIcon
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries, DataLayout::Run]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => SPATIAL_ELEMENTS,
            DataLayout::Timeseries | DataLayout::Run => TIMESERIES_ELEMENTS,
        }
    }

    fn variable_name(&self, layout: DataLayout, element: WeatherElement) -> Option<&'static str> {
        self.supported_elements(layout)
            .contains(&element)
            .then(|| element.open_meteo_s3_variable())
            .flatten()
    }

    fn spatial_object_key(
        &self,
        run_ref: &str,
        timestamp: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        super::OpenMeteoSpatialLayout::DWD_ICON.object_key(run_ref, timestamp)
    }

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        Ok(super::OpenMeteoTimeseriesLayout::DWD_ICON.object_key(variable, chunk))
    }

    fn run_object_key(&self, run_prefix: &str, variable: &str) -> ObjectKey {
        super::OpenMeteoRunLayout::DWD_ICON.object_key_in_prefix(run_prefix, variable)
    }
}

#[cfg(test)]
mod tests {
    use super::DwdIconSource;
    use crate::domain::{DataLayout, WeatherDataSource, WeatherElement};

    #[test]
    fn builds_spatial_object_key() {
        let source = DwdIconSource;
        let key = source
            .spatial_object_key("0000Z", "2024-02-03T0000")
            .expect("object key");
        assert_eq!(
            key.0,
            "data_spatial/dwd_icon/2024/02/03/0000Z/2024-02-03T0000.om"
        );
    }

    #[test]
    fn maps_dwd_fallback_variable() {
        let source = DwdIconSource;
        assert_eq!(
            source.variable_name(DataLayout::Spatial, WeatherElement::WeatherCode),
            Some("weather_code")
        );
    }
}
