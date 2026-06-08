use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[path = "gfs025_catalog.rs"]
mod gfs025_catalog;

use gfs025_catalog::{GFS025_SPATIAL_ELEMENTS, GFS025_TIMESERIES_ELEMENTS};

#[derive(Debug, Default)]
pub struct Gfs025Source;

impl WeatherDataSource for Gfs025Source {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::Gfs025
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => GFS025_SPATIAL_ELEMENTS,
            DataLayout::Timeseries => GFS025_TIMESERIES_ELEMENTS,
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
        super::OpenMeteoSpatialLayout::GFS025.object_key(run_ref, timestamp)
    }

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        Ok(super::OpenMeteoTimeseriesLayout::GFS025.object_key(variable, chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::Gfs025Source;
    use crate::domain::{DataLayout, WeatherDataSource, WeatherElement};

    #[test]
    fn builds_spatial_object_key() {
        let source = Gfs025Source;
        let key = source
            .spatial_object_key("0000Z", "2024-02-03T0000")
            .expect("object key");
        assert_eq!(
            key.0,
            "data_spatial/ncep_gfs025/2024/02/03/0000Z/2024-02-03T0000.om"
        );
    }

    #[test]
    fn maps_gfs_fallback_variable() {
        let source = Gfs025Source;
        assert_eq!(
            source.variable_name(DataLayout::Spatial, WeatherElement::Visibility),
            Some("visibility")
        );
    }

    #[test]
    fn gfs025_spatial_catalog_lists_all_manifest_variables() {
        let source = Gfs025Source;
        let elements = source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 316);
        assert!(elements.contains(&"visibility".to_string()));
        assert!(elements.contains(&"geopotential_height_500hPa".to_string()));
        assert!(elements.contains(&"cloud_cover_850hPa".to_string()));
    }

    #[test]
    fn gfs025_timeseries_catalog_lists_all_manifest_variables() {
        let source = Gfs025Source;
        let elements = source
            .supported_elements(DataLayout::Timeseries)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 317);
        assert!(elements.contains(&"static".to_string()));
    }
}
