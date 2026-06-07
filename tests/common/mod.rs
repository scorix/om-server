use ndarray::{ArrayD, ShapeError};
use omfiles::{OmCompressionType, OmFilesError, traits::OmFileWriterBackend, writer::OmFileWriter};

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error(transparent)]
    Shape(#[from] ShapeError),

    #[error(transparent)]
    Om(#[from] OmFilesError),
}

#[derive(Default)]
struct VecWriter {
    data: Vec<u8>,
}

impl OmFileWriterBackend for &mut VecWriter {
    fn write(&mut self, data: &[u8]) -> Result<(), OmFilesError> {
        self.data.extend_from_slice(data);
        Ok(())
    }

    fn synchronize(&self) -> Result<(), OmFilesError> {
        Ok(())
    }
}

pub fn write_sample_spatial_om() -> Result<Vec<u8>, FixtureError> {
    let data: Vec<f32> = vec![0.0, 5.0, 2.0, 3.0, 2.0, 5.0, 6.0, 2.0, 8.0, 3.0];
    let shape = vec![2, 5];
    let chunks = vec![2, 5];
    let array = ArrayD::from_shape_vec(
        shape
            .iter()
            .map(|value| *value as usize)
            .collect::<Vec<_>>(),
        data,
    )?;

    let mut backend = VecWriter::default();
    {
        let mut file_writer = OmFileWriter::new(&mut backend, 8);
        let mut writer = file_writer.prepare_array::<f32>(
            shape,
            chunks,
            OmCompressionType::PforDelta2dInt16,
            1.0,
            0.0,
        )?;
        writer.write_data(array.view(), None, None)?;
        let variable_meta = writer.finalize();
        let coordinates = file_writer.write_scalar("lat lon".to_string(), "coordinates", &[])?;
        let crs_wkt = file_writer.write_scalar(
            "GEOGCRS[\"WGS 84\",DATUM[\"World Geodetic System 1984\",ELLIPSOID[\"WGS 84\",6378137,298.257223563]],PRIMEM[\"Greenwich\",0],UNIT[\"degree\",0.0174532925199433],AXIS[\"Lat\",NORTH],AXIS[\"Lon\",EAST]] BBOX[-90,-180,90,180]]"
                .to_string(),
            "crs_wkt",
            &[],
        )?;
        let variable =
            file_writer.write_array(variable_meta, "temperature_2m", &[coordinates, crs_wkt])?;
        file_writer.write_trailer(variable)?;
    }

    Ok(backend.data)
}
