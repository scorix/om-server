use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum OmReaderBackend {
    LocalMmap(PathBuf),
    RangeHttp {
        base_url: String,
    },
}
