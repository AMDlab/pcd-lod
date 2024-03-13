use serde::{Deserialize, Serialize};

use crate::bounding_box::BoundingBox;

#[derive(Serialize, Deserialize)]
pub struct LOD {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub bounds: BoundingBox,
}

impl LOD {
    #[allow(unused)]
    pub fn new(x: i32, y: i32, z: i32, bounds: BoundingBox) -> LOD {
        LOD { x, y, z, bounds }
    }
}
