use anyhow::{Context, Result, bail};

use crate::domain::{
    DataLayout, ObjectKey, WeatherDataSource, WeatherModelId,
};

#[derive(Debug, Default)]
pub struct EcmwfIfs025SpatialSource;

impl WeatherDataSource for EcmwfIfs025SpatialSource {
    fn model_id(&self) -> WeatherModelId {
        WeatherModelId::EcmwfIfs025
    }

    fn supported_layouts(&self) -> &'static [DataLayout] {
        &[DataLayout::Spatial, DataLayout::Timeseries]
    }

    fn spatial_object_key(&self, run_ref: &str, timestamp: &str) -> Result<ObjectKey> {
        let (year, month, day) = parse_spatial_timestamp(timestamp)?;
        Ok(ObjectKey(format!(
            "data_spatial/ecmwf_ifs025/{year:04}/{month:02}/{day:02}/{run_ref}/{timestamp}.om"
        )))
    }

    fn timeseries_object_key(&self, variable: &str, chunk: &str) -> Result<ObjectKey> {
        let _ = (variable, chunk);
        bail!("data/ timeseries layout is not implemented yet")
    }
}

fn parse_spatial_timestamp(timestamp: &str) -> Result<(i32, u32, u32)> {
    let date = timestamp
        .split('T')
        .next()
        .with_context(|| format!("parse spatial timestamp {timestamp}"))?;
    let mut parts = date.split('-');
    let year = parts
        .next()
        .with_context(|| format!("missing year in {timestamp}"))?
        .parse::<i32>()?;
    let month = parts
        .next()
        .with_context(|| format!("missing month in {timestamp}"))?
        .parse::<u32>()?;
    let day = parts
        .next()
        .with_context(|| format!("missing day in {timestamp}"))?
        .parse::<u32>()?;
    Ok((year, month, day))
}

#[cfg(test)]
mod tests {
    use super::EcmwfIfs025SpatialSource;
    use crate::domain::WeatherDataSource;

    #[test]
    fn builds_spatial_object_key() {
        let source = EcmwfIfs025SpatialSource;
        let key = source
            .spatial_object_key("0000Z", "2024-02-03T0000")
            .expect("object key");
        assert_eq!(
            key.0,
            "data_spatial/ecmwf_ifs025/2024/02/03/0000Z/2024-02-03T0000.om"
        );
    }
}
