mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use om_server::application::spatial::SpatialService;
use om_server::domain::ObjectKey;
use om_server::error::HttpError;
use om_server::r#gen::GetSpatialMetaRequest;
use om_server::infrastructure::http::HttpClient;
use om_server::infrastructure::{
    HttpRangeReader, OmfilesDatasetReader, S3ObjectFetcher, open_meteo,
};
use omfiles::reader::OmFileReader;

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
    assert_eq!(meta.variables.len(), 1);
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
    let service = SpatialService::new(
        open_meteo::source_registry(),
        fetcher,
        OmfilesDatasetReader,
        true,
    );
    let response = service
        .get_spatial_meta(GetSpatialMetaRequest {
            model: "ecmwf_ifs025".to_string(),
            run_ref: "0000Z".to_string(),
            timestamp: "2024-02-03T0000".to_string(),
        })
        .expect("spatial meta");
    assert_eq!(response.model, "ecmwf_ifs025");
    assert_eq!(response.variables.len(), 1);
    assert!(std::path::Path::new(&response.local_path).exists());
    let key = ObjectKey(response.object_key.clone());
    assert_eq!(
        service.synced_path_for(&key),
        std::path::PathBuf::from(response.local_path)
    );
    temp.close().expect("cleanup tempdir");
}
