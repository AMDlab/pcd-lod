use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::prelude::BoundingBox;

/// bounding boxes for each unit in octree of LOD
pub type Coordinates = HashMap<u32, HashMap<String, BoundingBox>>;

/// Meta representation of the processed lod data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Meta {
    version: String,
    pub lod: u32,
    pub bounds: BoundingBox,
    pub coordinates: Coordinates,
}

impl Meta {
    pub fn new(lod: u32, bounds: BoundingBox, coordinates: Coordinates) -> Meta {
        Meta {
            version: env!("CARGO_PKG_VERSION").to_string(),
            lod,
            bounds,
            coordinates,
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn lod(&self) -> u32 {
        self.lod
    }

    pub fn bounds(&self) -> &BoundingBox {
        &self.bounds
    }

    pub fn coordinates(&self) -> &Coordinates {
        &self.coordinates
    }
}
