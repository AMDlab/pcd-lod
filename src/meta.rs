use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::bounding_box::BoundingBox;

pub type Coordinates = HashMap<u32, HashMap<String, BoundingBox>>;

/// Meta representation of the processed lod data
#[derive(Serialize, Deserialize)]
pub struct Meta {
    pub lod: u32,
    pub bounds: BoundingBox,
    pub coordinates: Coordinates,
}
