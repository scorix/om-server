use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GeoBoundingBox {
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}
