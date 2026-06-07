use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};

pub trait HttpClient: Send + Sync {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>>;

    fn download_to(&self, url: &str, path: &Path) -> Result<()>;

    fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>>;

    fn probe_content_length(&self, url: &str) -> Result<u64>;
}

#[derive(Debug, Default)]
pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let mut response = ureq::get(url)
            .call()
            .with_context(|| format!("GET {url}"))?;
        response
            .body_mut()
            .read_to_string()
            .with_context(|| format!("read response body {url}"))
            .map(|body| body.into_bytes())
    }

    fn download_to(&self, url: &str, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let bytes = self.get_bytes(url)?;
        let temp_path = path.with_extension("partial");
        {
            let mut file = fs::File::create(&temp_path)
                .with_context(|| format!("create {}", temp_path.display()))?;
            file.write_all(&bytes)
                .with_context(|| format!("write {}", temp_path.display()))?;
            file.sync_all()
                .with_context(|| format!("sync {}", temp_path.display()))?;
        }
        fs::rename(&temp_path, path)
            .with_context(|| format!("rename {} -> {}", temp_path.display(), path.display()))?;
        Ok(())
    }

    fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let end = offset.saturating_add(count).saturating_sub(1);
        let mut response = ureq::get(url)
            .header("Range", format!("bytes={offset}-{end}"))
            .call()
            .with_context(|| format!("GET range {url} bytes={offset}-{end}"))?;
        if response.status().as_u16() != 206 {
            anyhow::bail!("server did not return partial content for range request");
        }
        let reader = response.body_mut().as_reader();
        let mut body = Vec::with_capacity(count as usize);
        reader
            .take(count)
            .read_to_end(&mut body)
            .with_context(|| format!("read range body {url}"))?;
        Ok(body)
    }

    fn probe_content_length(&self, url: &str) -> Result<u64> {
        let response = ureq::get(url)
            .header("Range", "bytes=0-0")
            .call()
            .with_context(|| format!("GET range probe {url}"))?;
        if response.status().as_u16() != 206 {
            anyhow::bail!("server did not return partial content for range probe");
        }
        let content_range = response
            .headers()
            .get("content-range")
            .with_context(|| format!("range probe response missing Content-Range for {url}"))?
            .to_str()
            .context("parse Content-Range header")?;
        parse_content_range_size(content_range)
    }
}

pub fn probe_range_size(client: &impl HttpClient, url: &str) -> Result<u64> {
    client.probe_content_length(url)
}

pub fn fetch_range(client: &impl HttpClient, url: &str, offset: u64, count: u64) -> Result<Vec<u8>> {
    client.get_range(url, offset, count)
}

fn parse_content_range_size(value: &str) -> Result<u64> {
    value
        .split('/')
        .nth(1)
        .with_context(|| format!("parse Content-Range size from {value}"))?
        .trim()
        .parse::<u64>()
        .with_context(|| format!("parse Content-Range total from {value}"))
}

#[cfg(test)]
mod tests {
    use super::parse_content_range_size;

    #[test]
    fn parses_content_range_total() {
        assert_eq!(
            parse_content_range_size("bytes 0-0/12345").expect("size"),
            12345
        );
    }
}
