use crate::domain::{SourceRegistry, WeatherDataSource};

use super::{dwd, ecmwf, gfs};

#[derive(Debug, Default, Clone, Copy)]
pub struct OpenMeteoSources;

impl OpenMeteoSources {
    pub fn registry(self) -> SourceRegistry {
        SourceRegistry::new(self.adapters())
    }

    pub fn adapters(self) -> Vec<Box<dyn WeatherDataSource>> {
        vec![
            Box::new(ecmwf::EcmwfIfs025Source),
            Box::new(gfs::Gfs025Source),
            Box::new(dwd::DwdIconSource),
        ]
    }
}
