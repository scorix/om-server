use std::path::Path;
use std::sync::Arc;

use crate::domain::SpatialGrid;
use crate::error::DatasetError;
use crate::infrastructure::omfiles_dataset_reader::open_local;

pub fn read_flat_variable(
    local_path: &Path,
    variable_name: &str,
) -> Result<(Arc<SpatialGrid>, Vec<f32>), DatasetError> {
    let file = open_local(local_path)?;
    let session = file.session();
    let Some(dimensions) = session.variables().first_dimensions(&[variable_name]) else {
        return Err(DatasetError::VariableNotFound {
            variable: variable_name.to_string(),
        });
    };
    let grid = session.grid(dimensions)?;
    let values = session.with_variable(variable_name, |variable| {
        let array = variable
            .expect_array()
            .map_err(|source| DatasetError::NotArray {
                variable: variable_name.to_string(),
                source,
            })?;
        let ranges: Vec<std::ops::Range<u64>> = dimensions.iter().map(|size| 0..*size).collect();
        let slice = array
            .read::<f32>(&ranges)
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
    })?;
    Ok((grid, values))
}
