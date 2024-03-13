use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::bounding_box::BoundingBox;

pub type Coordinates = HashMap<i32, HashMap<String, BoundingBox>>;

/// Meta representation of the lod
#[derive(Serialize, Deserialize)]
pub struct Meta {
    pub lod: i32,
    pub bounds: BoundingBox,
    pub coordinates: Coordinates,
}
