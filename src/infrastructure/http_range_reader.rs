use std::sync::Arc;

use omfiles::OmFilesError;
use omfiles::traits::OmFileReaderBackend;

use crate::error::HttpError;

use super::http::{HttpClient, UreqHttpClient, fetch_range, probe_range_size};

#[derive(Debug, Clone)]
pub struct HttpRangeReader<C = UreqHttpClient> {
    url: String,
    size: usize,
    client: Arc<C>,
}

impl HttpRangeReader<UreqHttpClient> {
    pub fn new(url: impl Into<String>) -> Result<Self, HttpError> {
        let url = url.into();
        let size = probe_range_size(&UreqHttpClient, &url)? as usize;
        Ok(Self {
            url,
            size,
            client: Arc::new(UreqHttpClient),
        })
    }
}

impl<C> HttpRangeReader<C>
where
    C: HttpClient + 'static,
{
    pub fn with_client(url: impl Into<String>, client: C) -> Result<Self, HttpError> {
        let url = url.into();
        let size = probe_range_size(&client, &url)? as usize;
        Ok(Self {
            url,
            size,
            client: Arc::new(client),
        })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl<C> OmFileReaderBackend for HttpRangeReader<C>
where
    C: HttpClient + Send + Sync + 'static,
{
    type Bytes<'a> = Vec<u8>;

    fn count(&self) -> usize {
        self.size
    }

    fn prefetch_data(&self, _offset: usize, _count: usize) {}

    fn get_bytes(&self, offset: u64, count: u64) -> Result<Self::Bytes<'_>, OmFilesError> {
        fetch_range(self.client.as_ref(), &self.url, offset, count)
            .map_err(|error| OmFilesError::GenericError(error.to_string()))
    }
}
