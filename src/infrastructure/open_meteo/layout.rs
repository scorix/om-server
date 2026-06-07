use crate::domain::{ObjectKey, WeatherModelId};
use crate::error::{DataSourceError, TimestampParseError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMeteoSpatialLayout {
    model_path: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMeteoTimeseriesLayout {
    model_path: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenMeteoRunLayout {
    model_path: &'static str,
}

macro_rules! model_layouts {
    ($name:ident) => {
        impl $name {
            pub const ECMWF_IFS025: Self = Self {
                model_path: "ecmwf_ifs025",
            };
            pub const GFS025: Self = Self {
                model_path: "ncep_gfs025",
            };
            pub const DWD_ICON: Self = Self {
                model_path: "dwd_icon",
            };

            pub fn for_model(model: WeatherModelId) -> Self {
                Self {
                    model_path: model.as_str(),
                }
            }
        }
    };
}

model_layouts!(OpenMeteoSpatialLayout);
model_layouts!(OpenMeteoTimeseriesLayout);
model_layouts!(OpenMeteoRunLayout);

impl OpenMeteoSpatialLayout {
    pub fn object_key(self, run_ref: &str, timestamp: &str) -> Result<ObjectKey, DataSourceError> {
        let date = RunDate::from_timestamp(timestamp)?;
        Ok(ObjectKey(format!(
            "data_spatial/{}/{:04}/{:02}/{:02}/{run_ref}/{timestamp}.om",
            self.model_path, date.year, date.month, date.day
        )))
    }
}

impl OpenMeteoTimeseriesLayout {
    pub fn object_key(self, variable: &str, chunk: &str) -> ObjectKey {
        ObjectKey(format!("data/{}/{variable}/{chunk}.om", self.model_path))
    }
}

impl OpenMeteoRunLayout {
    pub fn run_prefix(self, year: i32, month: u32, day: u32, run_ref: &str) -> String {
        format!(
            "data_run/{}/{year:04}/{month:02}/{day:02}/{run_ref}/",
            self.model_path
        )
    }

    pub fn object_key_in_prefix(self, run_prefix: &str, variable: &str) -> ObjectKey {
        ObjectKey(format!("{run_prefix}{variable}.om"))
    }

    pub fn object_key(
        self,
        year: i32,
        month: u32,
        day: u32,
        run_ref: &str,
        variable: &str,
    ) -> ObjectKey {
        self.object_key_in_prefix(&self.run_prefix(year, month, day, run_ref), variable)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl RunDate {
    pub fn from_timestamp(timestamp: &str) -> Result<Self, TimestampParseError> {
        let date =
            timestamp
                .split('T')
                .next()
                .ok_or_else(|| TimestampParseError::InvalidFormat {
                    timestamp: timestamp.to_string(),
                })?;
        let mut parts = date.split('-');
        let year = parts
            .next()
            .ok_or_else(|| TimestampParseError::MissingYear {
                timestamp: timestamp.to_string(),
            })?
            .parse::<i32>()
            .map_err(|source| TimestampParseError::ParseYear {
                timestamp: timestamp.to_string(),
                source,
            })?;
        let month = parts
            .next()
            .ok_or_else(|| TimestampParseError::MissingMonth {
                timestamp: timestamp.to_string(),
            })?
            .parse::<u32>()
            .map_err(|source| TimestampParseError::ParseMonth {
                timestamp: timestamp.to_string(),
                source,
            })?;
        let day = parts
            .next()
            .ok_or_else(|| TimestampParseError::MissingDay {
                timestamp: timestamp.to_string(),
            })?
            .parse::<u32>()
            .map_err(|source| TimestampParseError::ParseDay {
                timestamp: timestamp.to_string(),
                source,
            })?;
        Ok(Self { year, month, day })
    }
}
