mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use om_server::application::active_catalog::ActiveSpatialCatalog;
use om_server::application::spatial::SpatialPointReader;
use om_server::application::spatial::SpatialService;
use om_server::domain::{
    DataLayout, ObjectFetcher, ObjectKey, SpatialObjectLocal, SpatialRunSnapshot, WeatherBakeLayer,
    WeatherDataSource, WeatherElement, WeatherModelId,
};
use om_server::error::{DataSourceError, HttpError};
use om_server::r#gen::GetSpatialMetaRequest;
use om_server::infrastructure::http::HttpClient;
use om_server::infrastructure::weather_bake_profile::{WeatherBakeLayerSpec, WeatherBakeProfile};
use om_server::infrastructure::{
    HttpRangeReader, OmfilesDatasetReader, S3ObjectFetcher, open_meteo,
};
use omfiles::reader::OmFileReader;

struct SingleElementSource;

struct TwoElementSource;

impl WeatherDataSource for TwoElementSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs025
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => &[WeatherElement::Temperature2m, WeatherElement::Precipitation],
            DataLayout::Timeseries => &[],
        }
    }

    fn variable_name(&self, layout: DataLayout, element: WeatherElement) -> Option<&'static str> {
        match (layout, element) {
            (DataLayout::Spatial, WeatherElement::Temperature2m) => Some("temperature_2m"),
            (DataLayout::Spatial, WeatherElement::Precipitation) => Some("precipitation"),
            _ => None,
        }
    }

    fn spatial_object_key(
        &self,
        _run_ref: &str,
        _timestamp: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        unimplemented!()
    }

    fn timeseries_object_key(
        &self,
        _variable: &str,
        _chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        unimplemented!()
    }
}

impl WeatherDataSource for SingleElementSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs025
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial]
    }

    fn supported_elements(&self, layout: DataLayout) -> &'static [WeatherElement] {
        match layout {
            DataLayout::Spatial => &[WeatherElement::Temperature2m],
            DataLayout::Timeseries => &[],
        }
    }

    fn variable_name(&self, layout: DataLayout, element: WeatherElement) -> Option<&'static str> {
        match (layout, element) {
            (DataLayout::Spatial, WeatherElement::Temperature2m) => Some("temperature_2m"),
            _ => None,
        }
    }

    fn spatial_object_key(
        &self,
        _run_ref: &str,
        _timestamp: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        unimplemented!()
    }

    fn timeseries_object_key(
        &self,
        _variable: &str,
        _chunk: &str,
    ) -> Result<ObjectKey, DataSourceError> {
        unimplemented!()
    }
}

struct MapClient {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl HttpClient for MapClient {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpError> {
        self.files
            .lock()
            .expect("lock")
            .get(url)
            .cloned()
            .ok_or_else(|| HttpError::MissingFixture {
                url: url.to_string(),
            })
    }

    fn download_to(&self, url: &str, path: &std::path::Path) -> Result<(), HttpError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| HttpError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(path, self.get_bytes(url)?).map_err(|source| HttpError::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>, HttpError> {
        let bytes = self.get_bytes(url)?;
        let start = offset as usize;
        let end = start.saturating_add(count as usize).min(bytes.len());
        Ok(bytes[start..end].to_vec())
    }

    fn probe_content_length(&self, url: &str) -> Result<u64, HttpError> {
        Ok(self.get_bytes(url)?.len() as u64)
    }
}

#[test]
fn range_http_backend_reads_fixture_metadata() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let url = "https://example.test/data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om";
    let client = MapClient {
        files: Mutex::new(HashMap::from([(url.to_string(), bytes)])),
    };
    let backend = HttpRangeReader::with_client(url, client).expect("range backend");
    let reader = OmFileReader::new(Arc::new(backend)).expect("reader");
    let meta =
        OmfilesDatasetReader::read_meta_from_reader(reader, Default::default()).expect("meta");
    assert_eq!(meta.variables.len(), 3);
    assert_eq!(meta.variables[0].name, "temperature_2m");
}

#[test]
fn spatial_service_returns_synced_metadata() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let url = "https://example.test/data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om";
    let client = MapClient {
        files: Mutex::new(HashMap::from([(url.to_string(), bytes)])),
    };
    let temp = tempfile::tempdir().expect("tempdir");
    let fetcher = S3ObjectFetcher::with_client("https://example.test", temp.path(), client);
    let key =
        ObjectKey("data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om".to_string());
    fetcher.sync_object(&key).expect("sync");
    let local_path = fetcher.synced_path(&key);
    let catalog = Arc::new(ActiveSpatialCatalog::new());
    catalog
        .publish(
            temp.path(),
            Arc::new(SpatialRunSnapshot {
                model: WeatherModelId::EcmwfIfs025,
                reference_time: "2024-02-03T0000Z".to_string(),
                run_ref: "0000Z".to_string(),
                objects: vec![SpatialObjectLocal {
                    object_key: key.0.clone(),
                    timestamp: "2024-02-03T0000".to_string(),
                    valid_date: "2024-02-03".to_string(),
                    local_path: local_path.clone(),
                }],
            }),
        )
        .expect("publish snapshot");
    let service = SpatialService::new(
        open_meteo::OpenMeteoSources.registry(),
        fetcher,
        OmfilesDatasetReader,
        catalog,
        WeatherBakeProfile {
            timeline_model: WeatherModelId::EcmwfIfs025,
            layers: vec![WeatherBakeLayerSpec {
                layer: WeatherBakeLayer::Temperature2m,
                model: WeatherModelId::EcmwfIfs025,
            }],
        },
        temp.path().join("manifests"),
    );
    let response = service
        .get_spatial_meta(GetSpatialMetaRequest {
            model: "ecmwf_ifs025".to_string(),
            run_ref: "0000Z".to_string(),
            timestamp: "2024-02-03T0000".to_string(),
        })
        .expect("spatial meta");
    assert_eq!(response.model, "ecmwf_ifs025");
    assert_eq!(response.variables.len(), 3);
    assert!(std::path::Path::new(&response.local_path).exists());
    let key = ObjectKey(response.object_key.clone());
    assert_eq!(
        service.synced_path_for(&key),
        std::path::PathBuf::from(response.local_path)
    );
    temp.close().expect("cleanup tempdir");
}

#[test]
fn read_at_skips_variables_missing_from_timestep_file() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("sample.om");
    std::fs::write(&path, bytes).expect("write fixture");

    let values =
        SpatialPointReader::read_at(&TwoElementSource, &path, 47.0, 11.0).expect("read point");

    assert_eq!(values.len(), 1);
    assert_eq!(values[0].element, "temperature_2m");
}

#[test]
fn read_supported_spatial_point_reads_every_listed_element() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("sample.om");
    std::fs::write(&path, bytes).expect("write fixture");
    let source = SingleElementSource;
    let elements = SpatialPointReader::element_names(&source);
    let values = SpatialPointReader::read_at(&source, &path, 47.0, 11.0).expect("read point");
    assert_eq!(values.len(), elements.len());
    assert_eq!(values[0].element, "temperature_2m");
    assert!(values[0].value.is_finite());
}

#[test]
fn ecmwf_ifs_spatial_catalog_lists_all_spatial_elements() {
    let registry = open_meteo::OpenMeteoSources.registry();
    let source = registry
        .get(WeatherModelId::EcmwfIfs)
        .expect("ecmwf_ifs source");
    let elements = SpatialPointReader::element_names(source);
    assert_eq!(elements.len(), 35);
    assert!(elements.contains(&"visibility".to_string()));
    assert!(elements.contains(&"dew_point_2m".to_string()));
}

#[test]
fn ecmwf_ifs025_spatial_catalog_lists_all_spatial_elements() {
    let registry = open_meteo::OpenMeteoSources.registry();
    let source = registry
        .get(WeatherModelId::EcmwfIfs025)
        .expect("ecmwf_ifs025 source");
    let elements = SpatialPointReader::element_names(source);
    assert_eq!(elements.len(), 119);
    assert!(elements.contains(&"temperature_2m".to_string()));
    assert!(elements.contains(&"relative_humidity_2m".to_string()));
    assert!(elements.contains(&"geopotential_height_850hPa".to_string()));
    assert!(elements.contains(&"snowfall".to_string()));
}

#[test]
fn dwd_icon_spatial_catalog_lists_all_spatial_elements() {
    let registry = open_meteo::OpenMeteoSources.registry();
    let source = registry
        .get(WeatherModelId::DwdIcon)
        .expect("dwd_icon source");
    let elements = SpatialPointReader::element_names(source);
    assert_eq!(elements.len(), 123);
    assert!(elements.contains(&"temperature_2m".to_string()));
    assert!(elements.contains(&"weather_code".to_string()));
    assert!(elements.contains(&"snowfall".to_string()));
}

#[test]
fn gfs025_spatial_catalog_lists_all_spatial_elements() {
    let registry = open_meteo::OpenMeteoSources.registry();
    let source = registry
        .get(WeatherModelId::Gfs025)
        .expect("ncep_gfs025 source");
    let elements = SpatialPointReader::element_names(source);
    assert_eq!(elements.len(), 316);
    assert!(elements.contains(&"visibility".to_string()));
    assert!(elements.contains(&"lifted_index".to_string()));
    assert!(elements.contains(&"cloud_cover_500hPa".to_string()));
}
