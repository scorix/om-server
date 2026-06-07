use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};

use crate::domain::{ObjectKey, OmFetcher};

use super::http::{HttpClient, UreqHttpClient};

#[derive(Debug, Clone)]
pub struct S3OmFetcher<C = UreqHttpClient> {
    base_url: String,
    sync_dir: PathBuf,
    client: Arc<C>,
}

impl S3OmFetcher<UreqHttpClient> {
    pub fn new(base_url: impl Into<String>, sync_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_url: base_url.into(),
            sync_dir: sync_dir.into(),
            client: Arc::new(UreqHttpClient),
        }
    }
}

impl<C> S3OmFetcher<C>
where
    C: HttpClient + 'static,
{
    pub fn with_client(
        base_url: impl Into<String>,
        sync_dir: impl Into<PathBuf>,
        client: C,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            sync_dir: sync_dir.into(),
            client: Arc::new(client),
        }
    }

    fn object_url(&self, object_key: &ObjectKey) -> String {
        format!(
            "{}/{}",
            self.base_url.trim_end_matches('/'),
            object_key.0.trim_start_matches('/')
        )
    }
}

impl<C> OmFetcher for S3OmFetcher<C>
where
    C: HttpClient + Send + Sync + 'static,
{
    fn sync_object(&self, object_key: &ObjectKey) -> Result<()> {
        let dest = self.synced_path(object_key);
        if dest.exists() {
            return Ok(());
        }
        let url = self.object_url(object_key);
        self.client
            .download_to(&url, &dest)
            .with_context(|| format!("sync {} to {}", url, dest.display()))
    }

    fn synced_path(&self, object_key: &ObjectKey) -> PathBuf {
        self.sync_dir.join(&object_key.0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MapClient {
        files: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl HttpClient for MapClient {
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
            self.files
                .lock()
                .expect("lock")
                .get(url)
                .cloned()
                .with_context(|| format!("missing fixture for {url}"))
        }

        fn download_to(&self, url: &str, path: &Path) -> Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, self.get_bytes(url)?)?;
            Ok(())
        }

        fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>> {
            let bytes = self.get_bytes(url)?;
            let start = offset as usize;
            let end = start.saturating_add(count as usize).min(bytes.len());
            Ok(bytes[start..end].to_vec())
        }

        fn probe_content_length(&self, url: &str) -> Result<u64> {
            Ok(self.get_bytes(url)?.len() as u64)
        }
    }

    #[test]
    fn syncs_object_into_om_sync_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let client = MapClient::default();
        client.files.lock().expect("lock").insert(
            "https://example.test/data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om"
                .to_string(),
            b"fixture".to_vec(),
        );
        let fetcher = S3OmFetcher::with_client("https://example.test", temp.path(), client);
        let key = ObjectKey(
            "data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om".to_string(),
        );
        fetcher.sync_object(&key).expect("sync");
        let synced = fetcher.synced_path(&key);
        assert!(synced.exists());
        assert_eq!(std::fs::read(synced).expect("read"), b"fixture");
        temp.close().expect("cleanup tempdir");
    }
}
