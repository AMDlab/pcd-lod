use crate::prelude::Point;

pub struct PointCloudUnit {
    pub points: Vec<Point>,
}

impl PointCloudUnit {
    pub fn uniform_sample_points(&self, threshold: usize) -> Vec<Point> {
        let n = self.points.len();
        let u = (n as f64) / (threshold as f64);
        (0..threshold)
            .map(|i| {
                let idx = (i as f64) * u;
                let idx = idx.floor() as usize;
                self.points[idx].clone()
            })
            .collect()
    }
}
