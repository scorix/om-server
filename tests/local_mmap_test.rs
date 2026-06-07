mod common;

use std::path::Path;

use om_server::domain::OmReaderBackend;
use om_server::infrastructure::OmDatasetReader;

#[test]
fn reads_local_mmap_fixture() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("sample.om");
    std::fs::write(&path, bytes).expect("write om file");

    let meta = OmDatasetReader::read_meta(OmReaderBackend::LocalMmap(path.clone()))
        .expect("read meta");
    assert_eq!(meta.local_path, path);
    assert_eq!(meta.variables.len(), 1);
    assert_eq!(meta.variables[0].name, "temperature_2m");
    assert_eq!(meta.variables[0].data_type, "float_array");
    assert!(!meta.variables[0].dimensions.is_empty());
    temp.close().expect("cleanup tempdir");
}

#[test]
fn local_fixture_path_is_regular_file() {
    let bytes = common::write_sample_spatial_om().expect("write fixture");
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("sample.om");
    std::fs::write(&path, bytes).expect("write om file");
    assert!(Path::new(&path).is_file());
    temp.close().expect("cleanup tempdir");
}
