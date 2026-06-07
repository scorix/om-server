use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[derive(Debug, Default)]
pub struct DwdIconSource;

const SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Temperature2m,
    WeatherElement::RelativeHumidity2m,
    WeatherElement::Precipitation,
    WeatherElement::Rain,
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
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => SPATIAL_ELEMENTS,
            DataLayout::Timeseries => TIMESERIES_ELEMENTS,
        }
    }

    fn variable_name(&self, layout: DataLayout, element: WeatherElement) -> Option<&'static str> {
        self.supported_elements(layout)
            .contains(&element)
            .then(|| super::standard_variable_name(element))
            .flatten()
    }

    fn spatial_object_key(
        &self,
        run_ref: &str,
        timestamp: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        super::spatial_object_key("dwd_icon", run_ref, timestamp)
    }

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        let _ = (variable, chunk);
        Err(DataSourceError::TimeseriesNotImplemented)
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
