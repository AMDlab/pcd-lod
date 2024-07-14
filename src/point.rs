use nalgebra::Point3;
use serde::{Deserialize, Serialize};

use crate::prelude::Color;

/// Point struct that holds the position and color
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Point {
    pub position: Point3<f64>,
    pub color: Option<Color>,
    pub intensity: Option<f64>,
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
        let intensity = split.next();
        match (x, y, z, r, g, b, intensity) {
            (Some(x), Some(y), Some(z), r, g, b, intensity) => {
                let x = x.parse()?;
                let y = y.parse()?;
                let z = z.parse()?;

                let (color, intensity) = match (r, g, b, intensity) {
                    (Some(r), Some(g), Some(b), Some(intensity)) => {
                        let r = r.parse()?;
                        let g = g.parse()?;
                        let b = b.parse()?;
                        let intensity = intensity.parse()?;
                        (Some(Color::new(r, g, b)), Some(intensity))
                    }
                    (Some(r), Some(g), Some(b), None) => {
                        let r = r.parse()?;
                        let g = g.parse()?;
                        let b = b.parse()?;
                        (Some(Color::new(r, g, b)), None)
                    }
                    (Some(intensity), _, _, _) => {
                        let intensity = intensity.parse()?;
                        (None, Some(intensity))
                    }
                    _ => (None, None),
                };

                Ok(Point {
                    position: Point3::new(x, y, z),
                    color,
                    intensity,
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
