use crate::prelude::Point;

pub struct PointCloudUnit {
    pub points: Vec<Point>,
}

impl PointCloudUnit {
    pub fn points(&self) -> &Vec<Point> {
        &self.points
    }
}
