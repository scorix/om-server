use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[derive(Debug, Default)]
pub struct EcmwfIfs025Source;

const SPATIAL_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Temperature2m,
    WeatherElement::RelativeHumidity2m,
    WeatherElement::Precipitation,
    WeatherElement::SnowDepth,
    WeatherElement::CloudCover,
    WeatherElement::CloudCoverLow,
    WeatherElement::CloudCoverMid,
    WeatherElement::CloudCoverHigh,
    WeatherElement::SurfaceTemperature,
    WeatherElement::ShortwaveRadiation,
    WeatherElement::Cape,
];

const TIMESERIES_ELEMENTS: &[WeatherElement] = &[
    WeatherElement::Temperature2m,
    WeatherElement::RelativeHumidity2m,
    WeatherElement::Precipitation,
    WeatherElement::SnowDepth,
    WeatherElement::WindGusts10m,
    WeatherElement::CloudCover,
    WeatherElement::CloudCoverLow,
    WeatherElement::CloudCoverMid,
    WeatherElement::CloudCoverHigh,
    WeatherElement::SurfaceTemperature,
    WeatherElement::ShortwaveRadiation,
    WeatherElement::Cape,
];

impl WeatherDataSource for EcmwfIfs025Source {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs025
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
        super::OpenMeteoSpatialLayout::ECMWF_IFS025.object_key(run_ref, timestamp)
    }

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        Ok(super::OpenMeteoTimeseriesLayout::ECMWF_IFS025.object_key(variable, chunk))
    }

    fn run_object_key(&self, run_prefix: &str, variable: &str) -> ObjectKey {
        super::OpenMeteoRunLayout::ECMWF_IFS025.object_key_in_prefix(run_prefix, variable)
    }
}

#[cfg(test)]
mod tests {
    use super::EcmwfIfs025Source;
    use crate::domain::{DataLayout, WeatherDataSource, WeatherElement};

    #[test]
    fn builds_spatial_object_key() {
        let source = EcmwfIfs025Source;
        let key = source
            .spatial_object_key("0000Z", "2024-02-03T0000")
            .expect("object key");
        assert_eq!(
            key.0,
            "data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om"
        );
    }

    #[test]
    fn maps_temperature_variable() {
        let source = EcmwfIfs025Source;
        assert_eq!(
            source.variable_name(DataLayout::Spatial, WeatherElement::Temperature2m),
            Some("temperature_2m")
        );
    }

    #[test]
    fn builds_timeseries_object_key() {
        let source = EcmwfIfs025Source;
        let key = source
            .timeseries_object_key("temperature_2m", "chunk_1523")
            .expect("object key");
        assert_eq!(
            key.0,
            "data/ecmwf_ifs025/temperature_2m/chunk_1523.om"
        );
    }

    #[test]
    fn builds_run_object_key() {
        let source = EcmwfIfs025Source;
        let key = source.run_object_key(
            "data_run/ecmwf_ifs025/2026/06/07/0000Z/",
            "temperature_2m",
        );
        assert_eq!(
            key.0,
            "data_run/ecmwf_ifs025/2026/06/07/0000Z/temperature_2m.om"
        );
    }
}
