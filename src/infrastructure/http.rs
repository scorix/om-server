use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use crate::error::HttpError;

pub trait HttpClient: Send + Sync {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpError>;

    fn download_to(&self, url: &str, path: &Path) -> Result<(), HttpError>;

    fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>, HttpError>;

    fn probe_content_length(&self, url: &str) -> Result<u64, HttpError>;
}

#[derive(Debug, Default)]
pub struct UreqHttpClient;

impl HttpClient for UreqHttpClient {
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, HttpError> {
        let mut response = ureq::get(url).call().map_err(|source| HttpError::Request {
            url: url.to_string(),
            source,
        })?;
        let mut body = Vec::new();
        response
            .body_mut()
            .as_reader()
            .read_to_end(&mut body)
            .map_err(|source| HttpError::ReadBody {
                url: url.to_string(),
                source,
            })?;
        Ok(body)
    }

    fn download_to(&self, url: &str, path: &Path) -> Result<(), HttpError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| HttpError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut response = ureq::get(url).call().map_err(|source| HttpError::Request {
            url: url.to_string(),
            source,
        })?;
        let temp_path = path.with_extension("partial");
        {
            let mut file = File::create(&temp_path).map_err(|source| HttpError::CreateFile {
                path: temp_path.clone(),
                source,
            })?;
            io::copy(&mut response.body_mut().as_reader(), &mut file).map_err(|source| {
                HttpError::WriteFile {
                    path: temp_path.clone(),
                    source,
                }
            })?;
            file.sync_all().map_err(|source| HttpError::SyncFile {
                path: temp_path.clone(),
                source,
            })?;
        }
        fs::rename(&temp_path, path).map_err(|source| HttpError::Rename {
            from: temp_path,
            to: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    fn get_range(&self, url: &str, offset: u64, count: u64) -> Result<Vec<u8>, HttpError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let end = offset.saturating_add(count).saturating_sub(1);
        let mut response = ureq::get(url)
            .header("Range", format!("bytes={offset}-{end}"))
            .call()
            .map_err(|source| HttpError::RangeRequest {
                url: url.to_string(),
                start: offset,
                end,
                source,
            })?;
        if response.status().as_u16() != 206 {
            return Err(HttpError::NotPartialContent {
                url: url.to_string(),
                status: response.status().as_u16(),
            });
        }
        let reader = response.body_mut().as_reader();
        let mut body = Vec::with_capacity(count as usize);
        reader
            .take(count)
            .read_to_end(&mut body)
            .map_err(|source| HttpError::ReadRangeBody {
                url: url.to_string(),
                source,
            })?;
        Ok(body)
    }

    fn probe_content_length(&self, url: &str) -> Result<u64, HttpError> {
        let response = ureq::get(url)
            .header("Range", "bytes=0-0")
            .call()
            .map_err(|source| HttpError::RangeRequest {
                url: url.to_string(),
                start: 0,
                end: 0,
                source,
            })?;
        if response.status().as_u16() != 206 {
            return Err(HttpError::NotPartialContent {
                url: url.to_string(),
                status: response.status().as_u16(),
            });
        }
        let content_range = response
            .headers()
            .get("content-range")
            .ok_or_else(|| HttpError::MissingContentRange {
                url: url.to_string(),
            })?
            .to_str()
            .map_err(|_| HttpError::InvalidContentRangeHeader {
                value: "<invalid utf-8>".to_string(),
            })?;
        parse_content_range_size(content_range)
    }
}

pub fn probe_range_size(client: &impl HttpClient, url: &str) -> Result<u64, HttpError> {
    client.probe_content_length(url)
}

pub fn fetch_range(
    client: &impl HttpClient,
    url: &str,
    offset: u64,
    count: u64,
) -> Result<Vec<u8>, HttpError> {
    client.get_range(url, offset, count)
}

fn parse_content_range_size(value: &str) -> Result<u64, HttpError> {
    let total = value
        .split('/')
        .nth(1)
        .ok_or_else(|| HttpError::InvalidContentRangeHeader {
            value: value.to_string(),
        })?
        .trim()
        .parse::<u64>()
        .map_err(|source| HttpError::ParseContentRangeTotal {
            value: value.to_string(),
            source,
        })?;
    Ok(total)
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
