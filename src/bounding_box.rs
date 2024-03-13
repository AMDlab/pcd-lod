use std::iter::FromIterator;

use nalgebra::{zero, Point3, Vector3};
use serde::{Deserialize, Serialize};

use crate::point::Point;

#[derive(Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Point3<f64>,
    pub max: Point3<f64>,
}

impl BoundingBox {
    pub fn new(min: Point3<f64>, max: Point3<f64>) -> BoundingBox {
        BoundingBox { min, max }
    }

    pub fn min_size(&self) -> f64 {
        let size = self.size();
        size.x.min(size.y).min(size.z)
    }

    pub fn max_size(&self) -> f64 {
        let size = self.size();
        size.x.max(size.y).max(size.z)
    }

    pub fn ceil(&self, unit: f64) -> (i32, i32, i32) {
        let size = self.size();
        let cx = (size.x / unit).ceil() as i32;
        let cy = (size.y / unit).ceil() as i32;
        let cz = (size.z / unit).ceil() as i32;
        (cx, cy, cz)
    }

    pub fn size(&self) -> Vector3<f64> {
        self.max - self.min
    }

    #[allow(unused)]
    pub fn min(&self) -> &Point3<f64> {
        &self.min
    }

    #[allow(unused)]
    pub fn max(&self) -> &Point3<f64> {
        &self.max
    }

    #[allow(unused)]
    pub fn center(&self) -> Point3<f64> {
        let p = (self.max.coords + self.min.coords) * 0.5;
        Point3::from(p)
    }

    pub fn extend(&mut self, p: &Point3<f64>) {
        self.min = self.min.inf(p);
        self.max = self.max.sup(p);
    }
}

impl FromIterator<Point3<f64>> for BoundingBox {
    fn from_iter<I: IntoIterator<Item = Point3<f64>>>(iter: I) -> Self {
        let mut min: Vector3<f64> = zero();
        min.fill(f64::MAX);
        let mut max: Vector3<f64> = zero();
        max.fill(f64::MIN);
        let mut b = Self {
            min: min.into(),
            max: max.into(),
        };
        for p in iter {
            b.extend(&p);
        }
        b
    }
}

impl<'a> FromIterator<&'a Point3<f64>> for BoundingBox {
    fn from_iter<I: IntoIterator<Item = &'a Point3<f64>>>(iter: I) -> Self {
        let mut min: Vector3<f64> = zero();
        min.fill(f64::MAX);
        let mut max: Vector3<f64> = zero();
        max.fill(f64::MIN);
        let mut b = Self {
            min: min.into(),
            max: max.into(),
        };
        for p in iter {
            b.extend(p);
        }
        b
    }
}

impl<'a> FromIterator<&'a Point> for BoundingBox {
    fn from_iter<I: IntoIterator<Item = &'a Point>>(iter: I) -> Self {
        let mut min: Vector3<f64> = zero();
        min.fill(f64::MAX);
        let mut max: Vector3<f64> = zero();
        max.fill(f64::MIN);
        let mut b = Self {
            min: min.into(),
            max: max.into(),
        };
        for p in iter {
            b.extend(&p.position);
        }
        b
    }
}
