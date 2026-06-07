use crate::domain::{DataLayout, SpatialObjectRef, SpatialRun, WeatherModelId};
use crate::error::OpenMeteoError;

const DEFAULT_OPEN_METEO_S3_BASE_URL: &str = "https://openmeteo.s3.amazonaws.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeseriesChunkRef {
    pub object_key: String,
    pub chunk: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunVariableRef {
    pub object_key: String,
    pub variable: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRunArchive {
    pub run_prefix: String,
    pub run_ref: String,
    pub variables: Vec<RunVariableRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenMeteoS3Catalog {
    base_url: String,
}

impl OpenMeteoS3Catalog {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub fn with_default_base_url() -> Self {
        Self::new(DEFAULT_OPEN_METEO_S3_BASE_URL)
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn load_latest_spatial_run(
        &self,
        model: WeatherModelId,
    ) -> Result<SpatialRun, OpenMeteoError> {
        let run_prefix = self.latest_dated_run_prefix(DataLayout::Spatial, model)?;
        let objects = self.list_spatial_objects(&run_prefix)?;
        if objects.is_empty() {
            return Err(OpenMeteoError::NoSpatialObjects {
                prefix: run_prefix.clone(),
            });
        }
        Ok(SpatialRun {
            reference_time: String::new(),
            run_prefix: run_prefix.clone(),
            run_ref: Self::run_ref_from_prefix(&run_prefix),
            objects,
        })
    }

    pub fn load_latest_run_archive(
        &self,
        model: WeatherModelId,
    ) -> Result<ModelRunArchive, OpenMeteoError> {
        let run_prefix = self.latest_dated_run_prefix(DataLayout::Run, model)?;
        let mut variables = Vec::new();
        for key in self.list_object_keys(&run_prefix)? {
            if !key.ends_with(".om") {
                continue;
            }
            let variable = key
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".om"))
                .ok_or_else(|| OpenMeteoError::InvalidRunObjectKey {
                    object_key: key.clone(),
                })?
                .to_string();
            variables.push(RunVariableRef {
                object_key: key,
                variable,
            });
        }
        variables.sort_by(|left, right| left.variable.cmp(&right.variable));
        if variables.is_empty() {
            return Err(OpenMeteoError::NoRunVariables {
                prefix: run_prefix.clone(),
            });
        }
        Ok(ModelRunArchive {
            run_ref: Self::run_ref_from_prefix(&run_prefix),
            run_prefix,
            variables,
        })
    }

    pub fn list_timeseries_variables(
        &self,
        model: WeatherModelId,
    ) -> Result<Vec<String>, OpenMeteoError> {
        self.list_published_variables(model.as_str())
    }

    pub fn list_timeseries_chunks(
        &self,
        model: WeatherModelId,
        variable: &str,
    ) -> Result<Vec<TimeseriesChunkRef>, OpenMeteoError> {
        let prefix = format!("data/{}/{variable}/", model.as_str());
        let mut chunks = Vec::new();
        for key in self.list_object_keys(&prefix)? {
            if !key.ends_with(".om") {
                continue;
            }
            let chunk = key
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".om"))
                .ok_or_else(|| OpenMeteoError::InvalidTimeseriesObjectKey {
                    object_key: key.clone(),
                })?
                .to_string();
            chunks.push(TimeseriesChunkRef {
                object_key: key,
                chunk,
            });
        }
        chunks.sort_by(|left, right| left.chunk.cmp(&right.chunk));
        Ok(chunks)
    }

    pub fn list_published_variables(
        &self,
        model_path: &str,
    ) -> Result<Vec<String>, OpenMeteoError> {
        let prefix = format!("data/{model_path}/");
        let body = self.list_s3(&prefix, Some("/"), None)?;
        Ok(self
            .extract_xml_values(&body, "Prefix")
            .into_iter()
            .filter_map(|value| {
                value
                    .strip_prefix(&prefix)
                    .and_then(|name| name.strip_suffix('/'))
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
            })
            .collect())
    }

    fn latest_dated_run_prefix(
        &self,
        layout: DataLayout,
        model: WeatherModelId,
    ) -> Result<String, OpenMeteoError> {
        let root_prefix = format!("{}/{}/", layout.s3_root(), model.as_str());
        let year_prefix = self.latest_common_prefix(&root_prefix)?;
        let month_prefix = self.latest_common_prefix(&year_prefix)?;
        let day_prefix = self.latest_common_prefix(&month_prefix)?;
        self.latest_common_prefix(&day_prefix)
    }

    pub(crate) fn list_spatial_objects(
        &self,
        run_prefix: &str,
    ) -> Result<Vec<SpatialObjectRef>, OpenMeteoError> {
        let mut objects = Vec::new();
        for key in self.list_object_keys(run_prefix)? {
            if !key.ends_with(".om") || key.contains("_model-level") {
                continue;
            }
            let timestamp = key
                .rsplit('/')
                .next()
                .and_then(|name| name.strip_suffix(".om"))
                .ok_or_else(|| OpenMeteoError::InvalidSpatialObjectKey {
                    object_key: key.clone(),
                })?
                .to_string();
            let valid_date = timestamp
                .split('T')
                .next()
                .ok_or_else(|| OpenMeteoError::InvalidSpatialObjectKey {
                    object_key: key.clone(),
                })?
                .to_string();
            objects.push(SpatialObjectRef {
                object_key: key,
                timestamp,
                valid_date,
            });
        }
        objects.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        Ok(objects)
    }

    pub(crate) fn run_ref_from_prefix(run_prefix: &str) -> String {
        run_prefix
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("0000Z")
            .to_string()
    }

    fn latest_common_prefix(&self, prefix: &str) -> Result<String, OpenMeteoError> {
        self.list_common_prefixes(prefix)?
            .into_iter()
            .max()
            .ok_or_else(|| OpenMeteoError::MissingS3Prefix {
                prefix: prefix.to_string(),
            })
    }

    fn list_common_prefixes(&self, prefix: &str) -> Result<Vec<String>, OpenMeteoError> {
        let body = self.list_s3(prefix, Some("/"), None)?;
        Ok(self
            .extract_xml_values(&body, "Prefix")
            .into_iter()
            .filter(|value| value != prefix)
            .collect())
    }

    fn list_object_keys(&self, prefix: &str) -> Result<Vec<String>, OpenMeteoError> {
        let mut keys = Vec::new();
        let mut continuation_token = None;
        loop {
            let body = self.list_s3(prefix, None, continuation_token.as_deref())?;
            keys.extend(self.extract_xml_values(&body, "Key"));
            continuation_token = self
                .extract_xml_values(&body, "NextContinuationToken")
                .into_iter()
                .next();
            if continuation_token.is_none() {
                break;
            }
        }
        Ok(keys)
    }

    pub(crate) fn fetch_text(&self, url: &str) -> Result<String, OpenMeteoError> {
        use std::io::Read;

        let mut response =
            ureq::get(url)
                .call()
                .map_err(|source| OpenMeteoError::FetchRequest {
                    url: url.to_string(),
                    source,
                })?;
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|source| OpenMeteoError::ReadFetchResponse {
                url: url.to_string(),
                source,
            })?;
        Ok(body)
    }

    fn list_s3(
        &self,
        prefix: &str,
        delimiter: Option<&str>,
        continuation_token: Option<&str>,
    ) -> Result<String, OpenMeteoError> {
        use std::io::Read;

        let mut url = format!(
            "{}/?list-type=2&prefix={}",
            self.base_url.trim_end_matches('/'),
            Self::url_encode(prefix)
        );
        if let Some(delimiter) = delimiter {
            url.push_str("&delimiter=");
            url.push_str(&Self::url_encode(delimiter));
        }
        if let Some(token) = continuation_token {
            url.push_str("&continuation-token=");
            url.push_str(&Self::url_encode(token));
        }
        let mut response =
            ureq::get(&url)
                .call()
                .map_err(|source| OpenMeteoError::ListRequest {
                    url: url.clone(),
                    source,
                })?;
        let mut body = String::new();
        response
            .body_mut()
            .as_reader()
            .read_to_string(&mut body)
            .map_err(|source| OpenMeteoError::ReadListResponse { url, source })?;
        Ok(body)
    }

    fn extract_xml_values(&self, body: &str, tag: &str) -> Vec<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let mut values = Vec::new();
        let mut rest = body;
        while let Some(start) = rest.find(&open) {
            let value_start = start + open.len();
            let Some(end) = rest[value_start..].find(&close) else {
                break;
            };
            values.push(Self::xml_unescape(&rest[value_start..value_start + end]));
            rest = &rest[value_start + end + close.len()..];
        }
        values
    }

    fn xml_unescape(value: &str) -> String {
        value
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }

    fn url_encode(value: &str) -> String {
        value
            .bytes()
            .flat_map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    vec![byte as char]
                }
                _ => format!("%{byte:02X}").chars().collect(),
            })
            .collect()
    }
}

impl Default for OpenMeteoS3Catalog {
    fn default() -> Self {
        Self::with_default_base_url()
    }
}
