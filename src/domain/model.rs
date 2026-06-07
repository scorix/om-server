use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherModelId {
    EcmwfIfs025,
}

impl WeatherModelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EcmwfIfs025 => "ecmwf_ifs025",
        }
    }

    pub fn all() -> &'static [Self] {
        &[Self::EcmwfIfs025]
    }
}

impl Display for WeatherModelId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WeatherModelId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ecmwf_ifs025" => Ok(Self::EcmwfIfs025),
            other => anyhow::bail!("unsupported weather model {other}"),
        }
    }
}
