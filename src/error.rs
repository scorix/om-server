use std::io;
use std::num::ParseFloatError;
use std::num::ParseIntError;
use std::path::PathBuf;

use omfiles::OmFilesError;

#[derive(Debug, thiserror::Error)]
pub enum GridError {
    #[error("expected 2D spatial variable, got dimensions {dimensions:?}")]
    InvalidDimensions { dimensions: Vec<u64> },

    #[error("unsupported coordinates metadata {coordinates}")]
    UnsupportedCoordinates { coordinates: String },

    #[error("crs_wkt missing BBOX")]
    MissingBbox,

    #[error("crs_wkt BBOX is unterminated")]
    UnterminatedBbox,

    #[error("parse crs_wkt BBOX values")]
    ParseBboxValues {
        #[source]
        source: ParseFloatError,
    },

    #[error("expected 4 crs_wkt BBOX values, got {count}")]
    InvalidBboxValueCount { count: usize },

    #[error("expected 2x2 interpolation window, got {count} values")]
    InvalidInterpolationWindow { count: usize },

    #[error("expected 1x1 point window, got {count} values")]
    InvalidPointWindow { count: usize },

    #[error("gaussian grid point count mismatch: expected {expected}, got {actual}")]
    GaussianPointCountMismatch { expected: u64, actual: u64 },

    #[error("unsupported gaussian grid with {point_count} points")]
    UnsupportedGaussianGrid { point_count: u64 },

    #[error("gaussian grid point {gridpoint} out of range [0, {max})")]
    GaussianGridPointOutOfRange { gridpoint: u64, max: u64 },
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported weather model {value}")]
pub struct ModelParseError {
    pub value: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DataSourceError {
    #[error(transparent)]
    Timestamp(#[from] TimestampParseError),
}

#[derive(Debug, thiserror::Error)]
pub enum TimestampParseError {
    #[error("parse spatial timestamp {timestamp}")]
    InvalidFormat { timestamp: String },

    #[error("missing year in {timestamp}")]
    MissingYear { timestamp: String },

    #[error("missing month in {timestamp}")]
    MissingMonth { timestamp: String },

    #[error("missing day in {timestamp}")]
    MissingDay { timestamp: String },

    #[error("parse year in {timestamp}")]
    ParseYear {
        timestamp: String,
        #[source]
        source: ParseIntError,
    },

    #[error("parse month in {timestamp}")]
    ParseMonth {
        timestamp: String,
        #[source]
        source: ParseIntError,
    },

    #[error("parse day in {timestamp}")]
    ParseDay {
        timestamp: String,
        #[source]
        source: ParseIntError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum TileRenderError {
    #[error("weather tile rendering is not implemented")]
    NotImplemented,

    #[error("encode weather tile PNG")]
    EncodePng {
        #[source]
        source: image::ImageError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum WeatherBakeError {
    #[error("tile cache directory is required for weather bake")]
    MissingCacheDir,

    #[error(transparent)]
    ActiveCatalog(#[from] ActiveCatalogError),

    #[error(transparent)]
    Dataset(#[from] DatasetError),

    #[error(transparent)]
    TileRender(#[from] TileRenderError),

    #[error(transparent)]
    Timestamp(#[from] TimestampParseError),

    #[error("read file {path}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("write file {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("serialize weather manifest")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },

    #[error("write PMTiles {path}: {message}")]
    PmtilesWrite { path: PathBuf, message: String },

    #[error("read weather bake config {path}")]
    ReadConfig {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("parse weather bake config {path}")]
    ParseConfig {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("weather bake config {path}: no layers configured")]
    EmptyLayers { path: PathBuf },

    #[error("weather bake config {path}: unknown variable {variable}")]
    UnknownVariable { path: PathBuf, variable: String },

    #[error("weather bake config {path}: layer {variable} is missing required `model`")]
    MissingLayerModel { path: PathBuf, variable: String },

    #[error("weather bake config {path}: duplicate variable {variable}")]
    DuplicateVariable { path: PathBuf, variable: String },

    #[error("weather bake config {path}: unknown model {model} for variable {variable}")]
    UnknownModel {
        path: PathBuf,
        variable: String,
        model: String,
    },

    #[error("no native timesteps available to bake {valid_time}")]
    MissingNativeTimestep { valid_time: String },
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("GET {url}")]
    Request {
        url: String,
        #[source]
        source: ureq::Error,
    },

    #[error("read response body for {url}")]
    ReadBody {
        url: String,
        #[source]
        source: io::Error,
    },

    #[error("GET range {url} bytes={start}-{end}")]
    RangeRequest {
        url: String,
        start: u64,
        end: u64,
        #[source]
        source: ureq::Error,
    },

    #[error("server returned status {status} instead of 206 for range request to {url}")]
    NotPartialContent { url: String, status: u16 },

    #[error("read range body for {url}")]
    ReadRangeBody {
        url: String,
        #[source]
        source: io::Error,
    },

    #[error("range probe response missing Content-Range for {url}")]
    MissingContentRange { url: String },

    #[error("invalid Content-Range header {value}")]
    InvalidContentRangeHeader { value: String },

    #[error("parse Content-Range total from {value}")]
    ParseContentRangeTotal {
        value: String,
        #[source]
        source: ParseIntError,
    },

    #[error("create directory {path}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("create file {path}")]
    CreateFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("write file {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("sync file {path}")]
    SyncFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("rename {from} to {to}")]
    Rename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("missing fixture for {url}")]
    MissingFixture { url: String },
}

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("sync {url} to {path}")]
    Download {
        url: String,
        path: PathBuf,
        #[source]
        source: Box<HttpError>,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("non-UTF8 path {path}")]
    NonUtf8Path { path: PathBuf },

    #[error("open om file {path}")]
    OpenFile {
        path: PathBuf,
        #[source]
        source: OmFilesError,
    },

    #[error("variable {variable} not found")]
    VariableNotFound { variable: String },

    #[error("variable {variable} is not an array")]
    NotArray {
        variable: String,
        #[source]
        source: OmFilesError,
    },

    #[error("OM file missing {field} metadata")]
    MissingMetadata { field: &'static str },

    #[error("expected array variable metadata")]
    ExpectedArray {
        #[source]
        source: OmFilesError,
    },

    #[error("read contiguous values for variable {variable}")]
    NonContiguousValues { variable: String },

    #[error(transparent)]
    Grid(#[from] GridError),

    #[error(transparent)]
    Http(#[from] HttpError),

    #[error("read om variable {variable}")]
    ReadVariable {
        variable: String,
        #[source]
        source: OmFilesError,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum OpenMeteoError {
    #[error("GET {url}")]
    FetchRequest {
        url: String,
        #[source]
        source: ureq::Error,
    },

    #[error("read response body for {url}")]
    ReadFetchResponse {
        url: String,
        #[source]
        source: io::Error,
    },

    #[error("parse run manifest at {url}")]
    ParseRunManifest {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("invalid run manifest reference_time {reference_time}")]
    InvalidManifestReferenceTime { reference_time: String },

    #[error("invalid run manifest valid_time {valid_time}")]
    InvalidManifestValidTime { valid_time: String },

    #[error("GET {url}")]
    ListRequest {
        url: String,
        #[source]
        source: ureq::Error,
    },

    #[error("read S3 list response for {url}")]
    ReadListResponse {
        url: String,
        #[source]
        source: io::Error,
    },

    #[error(transparent)]
    Timestamp(#[from] TimestampParseError),

    #[error("no S3 prefixes under {prefix}")]
    MissingS3Prefix { prefix: String },

    #[error("invalid spatial object key {object_key}")]
    InvalidSpatialObjectKey { object_key: String },

    #[error("no spatial objects under {prefix}")]
    NoSpatialObjects { prefix: String },

    #[error("invalid run object key {object_key}")]
    InvalidRunObjectKey { object_key: String },

    #[error("no run variables under {prefix}")]
    NoRunVariables { prefix: String },

    #[error("invalid timeseries object key {object_key}")]
    InvalidTimeseriesObjectKey { object_key: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ActiveCatalogError {
    #[error("serialize active manifest for {model}")]
    SerializeManifest {
        model: crate::domain::WeatherModelId,
        #[source]
        source: serde_json::Error,
    },

    #[error("write active manifest at {path}")]
    WriteManifest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("read active manifest at {path}")]
    ReadManifest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("parse active manifest at {path}")]
    ParseManifest {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SyncWorkerError {
    #[error("unknown model {model}")]
    UnknownModel {
        model: crate::domain::WeatherModelId,
    },

    #[error("spatial run for {model} has no objects")]
    EmptyRun {
        model: crate::domain::WeatherModelId,
    },

    #[error(transparent)]
    DataSource(#[from] DataSourceError),

    #[error(transparent)]
    Sync(#[from] SyncError),

    #[error(transparent)]
    OpenMeteo(#[from] OpenMeteoError),

    #[error(transparent)]
    ActiveCatalog(#[from] ActiveCatalogError),
}

#[derive(Debug, thiserror::Error)]
pub enum SpatialServiceError {
    #[error("unsupported model {model}")]
    UnsupportedModel {
        model: String,
        #[source]
        source: ModelParseError,
    },

    #[error("unknown model {model}")]
    UnknownModel { model: String },

    #[error("no active spatial run for model {model}")]
    NotReady { model: String },

    #[error("object {object_key} is not synced at {path}")]
    NotSynced { object_key: String, path: PathBuf },

    #[error(transparent)]
    DataSource(#[from] DataSourceError),

    #[error(transparent)]
    Sync(#[from] SyncError),

    #[error(transparent)]
    Dataset(#[from] DatasetError),

    #[error(transparent)]
    OpenMeteo(#[from] OpenMeteoError),

    #[error("spatial point series returned no samples")]
    EmptyPointSeries,

    #[error("weather bake profile has no layers")]
    EmptyBlendProfile,

    #[error("unknown weather tile variable {variable}")]
    UnknownWeatherVariable { variable: String },

    #[error("weather manifest not found at {path}")]
    WeatherManifestNotFound { path: PathBuf },

    #[error("read weather manifest at {path}")]
    ReadWeatherManifest {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum MainError {
    #[error("parse grpc bind address {address}")]
    ParseAddress {
        address: String,
        #[source]
        source: std::net::AddrParseError,
    },

    #[error("invalid sync model {model}")]
    InvalidSyncModel {
        model: String,
        #[source]
        source: ModelParseError,
    },

    #[error(transparent)]
    ActiveCatalog(#[from] ActiveCatalogError),

    #[error("failed to build gRPC reflection service")]
    Reflection {
        #[source]
        source: tonic_reflection::server::Error,
    },

    #[error(transparent)]
    Serve(#[from] tonic::transport::Error),

    #[error(transparent)]
    WeatherBake(#[from] WeatherBakeError),
}
