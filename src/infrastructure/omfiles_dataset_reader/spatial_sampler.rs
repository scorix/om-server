use omfiles::traits::OmFileReaderBackend;

use crate::domain::InterpolationWindow;
use crate::error::DatasetError;

use super::spatial_file::OmSpatialFileSession;

pub struct SpatialSampler<'a, B> {
    file: &'a OmSpatialFileSession<'a, B>,
    window: InterpolationWindow,
}

impl<'a, B: OmFileReaderBackend> SpatialSampler<'a, B> {
    pub fn prepare(
        file: &'a OmSpatialFileSession<'a, B>,
        variable_names: &[&str],
        latitude: f64,
        longitude: f64,
    ) -> Result<Option<Self>, DatasetError> {
        let Some(dimensions) = file.variables().first_dimensions(variable_names) else {
            return Ok(None);
        };
        let grid = file.grid(dimensions)?;
        let window = grid
            .interpolation_window(latitude, longitude)
            .map_err(DatasetError::Grid)?;
        Ok(Some(Self { file, window }))
    }

    pub fn read(&self, variable_name: &str) -> Result<f64, DatasetError> {
        if self.window.sample_indices.len() == 4 {
            let scattered = self.read_scattered(variable_name)?;
            return self
                .window
                .interpolate(&scattered)
                .map_err(DatasetError::Grid);
        }
        let values = self.read_contiguous(variable_name)?;
        self.window.interpolate(&values).map_err(DatasetError::Grid)
    }

    pub fn read_many(&self, variable_names: &[&str]) -> Result<Vec<Option<f64>>, DatasetError> {
        let mut values = Vec::with_capacity(variable_names.len());
        for &variable_name in variable_names {
            let value = match self.read(variable_name) {
                Ok(value) => Some(value),
                Err(DatasetError::VariableNotFound { .. }) => None,
                Err(error) => return Err(error),
            };
            values.push(value);
        }
        Ok(values)
    }

    fn read_contiguous(&self, variable_name: &str) -> Result<Vec<f32>, DatasetError> {
        self.file.with_variable(variable_name, |variable| {
            let slice = variable
                .expect_array()
                .map_err(|source| DatasetError::NotArray {
                    variable: variable_name.to_string(),
                    source,
                })?
                .read::<f32>(&self.window.ranges)
                .map_err(|source| DatasetError::ReadVariable {
                    variable: variable_name.to_string(),
                    source,
                })?;
            slice
                .as_slice()
                .map(|values| values.to_vec())
                .ok_or_else(|| DatasetError::NonContiguousValues {
                    variable: variable_name.to_string(),
                })
        })
    }

    fn read_scattered(&self, variable_name: &str) -> Result<Vec<f32>, DatasetError> {
        self.file.with_variable(variable_name, |variable| {
            let array = variable
                .expect_array()
                .map_err(|source| DatasetError::NotArray {
                    variable: variable_name.to_string(),
                    source,
                })?;
            let mut values = Vec::with_capacity(self.window.sample_indices.len());
            for &index in &self.window.sample_indices {
                let slice = array
                    .read::<f32>(&[0..1, index..index + 1])
                    .map_err(|source| DatasetError::ReadVariable {
                        variable: variable_name.to_string(),
                        source,
                    })?;
                let value = slice
                    .as_slice()
                    .ok_or_else(|| DatasetError::NonContiguousValues {
                        variable: variable_name.to_string(),
                    })?[0];
                values.push(value);
            }
            Ok(values)
        })
    }
}
