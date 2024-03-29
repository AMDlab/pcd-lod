use nalgebra::Point3;
use serde::{Deserialize, Serialize};

use crate::prelude::Color;

/// Point struct that holds the position and color
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Point {
    pub position: Point3<f64>,
    pub color: Option<Color>,
}

impl Point {
    pub fn try_parse(line: &str) -> anyhow::Result<Self> {
        let mut split = line.split_whitespace();
        let x = split.next();
        let y = split.next();
        let z = split.next();
        let r = split.next();
        let g = split.next();
        let b = split.next();
        match (x, y, z, r, g, b) {
            (Some(x), Some(y), Some(z), Some(r), Some(g), Some(b)) => {
                let x = x.parse().unwrap();
                let y = y.parse().unwrap();
                let z = z.parse().unwrap();
                let r = r.parse().unwrap();
                let g = g.parse().unwrap();
                let b = b.parse().unwrap();
                Ok(Point {
                    position: Point3::new(x, y, z),
                    color: Some(Color::new(r, g, b)),
                })
            }
            (Some(x), Some(y), Some(z), _, _, _) => {
                let x = x.parse().unwrap();
                let y = y.parse().unwrap();
                let z = z.parse().unwrap();
                Ok(Point {
                    position: Point3::new(x, y, z),
                    color: None,
                })
            }
            _ => Err(anyhow::anyhow!("Invalid point format")),
        }
    }

    pub fn distance(&self, other: &Self) -> f64 {
        let d = self.distance_squared(other);
        d.sqrt()
    }

    pub fn distance_squared(&self, other: &Self) -> f64 {
        let d = self.position - other.position;
        d.magnitude_squared()
    }
}
