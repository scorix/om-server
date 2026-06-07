mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use omfiles::reader::OmFileReader;
use om_server::application::spatial::SpatialService;
use om_server::domain::{ObjectKey, SourceRegistry};
use om_server::r#gen::GetSpatialMetaRequest;
use om_server::infrastructure::{OmDatasetReader, RangeHttpBackend, S3OmFetcher};
use om_server::infrastructure::http::HttpClient;

struct MapClient {
    files: Mutex<HashMap<String, Vec<u8>>>,
}

impl HttpClient for MapClient {
    fn get_bytes(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        self.files
            .lock()
            .expect("lock")
            .get(url)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing fixture for {url}"))
    }

    fn download_to(&self, url: &str, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.get_bytes(url)?)?;
        Ok(())
    }

    fn get_range(&self, url: &str, offset: u64, count: u64) -> anyhow::Result<Vec<u8>> {
        let bytes = self.get_bytes(url)?;
        let start = offset as usize;
        let end = start.saturating_add(count as usize).min(bytes.len());
        Ok(bytes[start..end].to_vec())
    }

    fn probe_content_length(&self, url: &str) -> anyhow::Result<u64> {
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
    let backend = RangeHttpBackend::with_client(url, client).expect("range backend");
    let reader = OmFileReader::new(Arc::new(backend)).expect("reader");
    let meta = OmDatasetReader::read_meta_from_reader(reader, Default::default()).expect("meta");
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
    let fetcher = S3OmFetcher::with_client("https://example.test", temp.path(), client);
    let service = SpatialService::new(SourceRegistry::with_defaults(), fetcher, true);
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
