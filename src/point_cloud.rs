use crate::point::Point;

#[derive(Clone, Debug)]
pub struct PointCloud {
    points: Vec<Point>,
}

impl PointCloud {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<&Point> {
        self.points.get(idx)
    }

    pub fn get_closest_distance(&self, index: usize) -> Option<f64> {
        self.get_closest_point(index)
            .and_then(|closest| self.get(index).map(|point| point.distance(closest)))
    }

    pub fn get_closest_point(&self, index: usize) -> Option<&Point> {
        match self.points.get(index) {
            Some(point) => {
                let distances = self.points.iter().enumerate().filter_map(|(idx, other)| {
                    if idx == index {
                        None
                    } else {
                        Some((idx, point.distance_squared(other)))
                    }
                });
                let closest = distances
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                closest.and_then(|(idx, _)| self.points.get(idx))
            }
            None => None,
        }
    }
}
