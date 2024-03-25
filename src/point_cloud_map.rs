use std::collections::HashMap;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{bounding_box::BoundingBox, point::Point, point_cloud_unit::PointCloudUnit, LODKey};

pub struct PointCloudMap {
    lod: u32,
    bounds: BoundingBox,
    map: HashMap<LODKey, PointCloudUnit>,
}

impl PointCloudMap {
    pub fn root(lod: u32, bounds: BoundingBox, points: &Vec<Point>) -> Self {
        Self {
            lod,
            bounds,
            map: vec![(
                (0, 0, 0),
                PointCloudUnit {
                    points: points.clone(),
                },
            )]
            .into_iter()
            .collect(),
        }
    }

    pub fn lod(&self) -> u32 {
        self.lod
    }

    pub fn bounds(&self) -> &BoundingBox {
        &self.bounds
    }

    pub fn divide(&self, threshold: usize) -> Self {
        let next_lod = self.lod + 1;
        let div = 2_f64.powf(next_lod as f64);
        let min = &self.bounds.min;
        let unit = self.bounds.max_size() / div;

        let mut next = HashMap::new();

        self.map.iter().for_each(|(_k, v)| {
            if v.points.len() > threshold {
                let pts: Vec<(LODKey, Point)> = v
                    .points
                    .par_iter()
                    .map(|v| {
                        let position = v.position;
                        let x = position.x;
                        let y = position.y;
                        let z = position.z;
                        let ix = ((x - min.x) / unit).floor() as i32;
                        let iy = ((y - min.y) / unit).floor() as i32;
                        let iz = ((z - min.z) / unit).floor() as i32;
                        let key = (ix, iy, iz);
                        (key, v.clone())
                    })
                    .collect();
                for (key, v) in pts {
                    next.entry(key).or_insert_with(Vec::new).push(v);
                }
            }
        });

        Self {
            lod: next_lod,
            bounds: self.bounds.clone(),
            map: next
                .into_iter()
                .map(|pts| {
                    let (key, points) = pts;
                    (key, PointCloudUnit { points })
                })
                .collect(),
        }
    }

    pub fn map(&self) -> &HashMap<LODKey, PointCloudUnit> {
        &self.map
    }
}
