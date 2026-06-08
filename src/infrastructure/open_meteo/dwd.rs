use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[path = "dwd_icon_catalog.rs"]
mod dwd_icon_catalog;

use dwd_icon_catalog::{DWD_SPATIAL_ELEMENTS, DWD_TIMESERIES_ELEMENTS};

#[derive(Debug, Default)]
pub struct DwdIconSource;

impl WeatherDataSource for DwdIconSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::DwdIcon
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => DWD_SPATIAL_ELEMENTS,
            DataLayout::Timeseries => DWD_TIMESERIES_ELEMENTS,
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

    #[test]
    fn dwd_icon_spatial_catalog_lists_all_manifest_variables() {
        let source = DwdIconSource;
        let elements = source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 123);
        assert!(elements.contains(&"temperature_2m".to_string()));
        assert!(elements.contains(&"snowfall".to_string()));
        assert!(elements.contains(&"weather_code".to_string()));
    }

    #[test]
    fn dwd_icon_timeseries_catalog_lists_all_manifest_variables() {
        let source = DwdIconSource;
        let elements = source
            .supported_elements(DataLayout::Timeseries)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 134);
        assert!(elements.contains(&"static".to_string()));
    }
}
