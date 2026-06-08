use crate::domain::{DataLayout, ObjectKey, WeatherDataSource, WeatherElement, WeatherModelId};
use crate::error::DataSourceError;

#[path = "ecmwf_ifs_catalog.rs"]
mod ecmwf_ifs_catalog;

#[path = "ecmwf_ifs025_catalog.rs"]
mod ecmwf_ifs025_catalog;

use ecmwf_ifs_catalog::{IFS_SPATIAL_ELEMENTS, IFS_TIMESERIES_ELEMENTS};
use ecmwf_ifs025_catalog::{IFS025_SPATIAL_ELEMENTS, IFS025_TIMESERIES_ELEMENTS};

#[derive(Debug, Default)]
pub struct EcmwfIfsSource;

#[derive(Debug, Default)]
pub struct EcmwfIfs025Source;

impl WeatherDataSource for EcmwfIfsSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => IFS_SPATIAL_ELEMENTS,
            DataLayout::Timeseries => IFS_TIMESERIES_ELEMENTS,
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
        super::OpenMeteoSpatialLayout::for_model(WeatherModelId::EcmwfIfs)
            .object_key(run_ref, timestamp)
    }

    fn timeseries_object_key(
        &self,
        variable: &str,
        chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        Ok(
            super::OpenMeteoTimeseriesLayout::for_model(WeatherModelId::EcmwfIfs)
                .object_key(variable, chunk),
        )
    }
}

impl WeatherDataSource for EcmwfIfs025Source {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs025
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => IFS025_SPATIAL_ELEMENTS,
            DataLayout::Timeseries => IFS025_TIMESERIES_ELEMENTS,
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
}

#[cfg(test)]
mod tests {
    use super::{EcmwfIfs025Source, EcmwfIfsSource};
    use crate::domain::{DataLayout, WeatherDataSource, WeatherElement};

    #[test]
    fn ecmwf_ifs_spatial_catalog_lists_all_manifest_variables() {
        let source = EcmwfIfsSource;
        let elements = source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 35);
        assert_eq!(
            elements,
            vec![
                "boundary_layer_height",
                "cape",
                "cloud_cover",
                "cloud_cover_high",
                "cloud_cover_low",
                "cloud_cover_mid",
                "convective_inhibition",
                "dew_point_2m",
                "direct_radiation",
                "precipitation",
                "pressure_msl",
                "shortwave_radiation",
                "showers",
                "snowfall",
                "soil_moisture_0_to_7cm",
                "soil_moisture_7_to_28cm",
                "soil_moisture_28_to_100cm",
                "soil_moisture_100_to_255cm",
                "soil_temperature_0_to_7cm",
                "soil_temperature_7_to_28cm",
                "soil_temperature_28_to_100cm",
                "soil_temperature_100_to_255cm",
                "surface_temperature",
                "temperature_2m",
                "temperature_2m_max",
                "temperature_2m_min",
                "total_column_integrated_water_vapour",
                "visibility",
                "wind_gusts_10m",
                "wind_u_component_100m",
                "wind_u_component_10m",
                "wind_u_component_200m",
                "wind_v_component_100m",
                "wind_v_component_10m",
                "wind_v_component_200m",
            ]
        );
    }

    #[test]
    fn ecmwf_ifs_timeseries_catalog_lists_all_manifest_variables() {
        let source = EcmwfIfsSource;
        let elements = source
            .supported_elements(DataLayout::Timeseries)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 36);
        assert!(elements.contains(&"roughness_length".to_string()));
        assert!(elements.contains(&"visibility".to_string()));
    }

    #[test]
    fn ecmwf_ifs025_spatial_catalog_lists_all_manifest_variables() {
        let source = EcmwfIfs025Source;
        let elements = source
            .supported_elements(DataLayout::Spatial)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 119);
        assert!(elements.contains(&"temperature_2m".to_string()));
        assert!(elements.contains(&"geopotential_height_500hPa".to_string()));
        assert!(elements.contains(&"snowfall".to_string()));
    }

    #[test]
    fn ecmwf_ifs025_timeseries_catalog_lists_all_manifest_variables() {
        let source = EcmwfIfs025Source;
        let elements = source
            .supported_elements(DataLayout::Timeseries)
            .iter()
            .map(|element| element.as_str().to_string())
            .collect::<Vec<_>>();
        assert_eq!(elements.len(), 114);
        assert!(elements.contains(&"wind_gusts_10m".to_string()));
        assert!(!elements.contains(&"geopotential_height_10hPa".to_string()));
    }

    #[test]
    fn builds_ecmwf_ifs_spatial_object_key() {
        let source = EcmwfIfsSource;
        let key = source
            .spatial_object_key("0600Z", "2026-06-08T0600")
            .expect("object key");
        assert_eq!(
            key.0,
            "data_spatial/ecmwf_ifs/2026/06/08/0600Z/2026-06-08T0600.om"
        );
    }

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
        assert_eq!(key.0, "data/ecmwf_ifs025/temperature_2m/chunk_1523.om");
    }
}
