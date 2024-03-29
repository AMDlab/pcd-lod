mod bounding_box;
mod color;
mod encoder;
mod meta;
mod point;
mod point_cloud_map;
mod point_cloud_unit;

/// key represents level of detail for hash map
type LODKey = (i32, i32, i32);

pub mod prelude {
    pub use crate::bounding_box::*;
    pub use crate::color::*;
    pub use crate::encoder::*;
    pub use crate::meta::*;
    pub use crate::point::*;
    pub use crate::point_cloud_map::*;
    pub use crate::point_cloud_unit::*;
}
