use std::collections::HashMap;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    prelude::{BoundingBox, Point, PointCloudUnit},
    LODKey,
};

/// PointCloudMap struct that holds the octree of the point cloud data.
pub struct PointCloudMap {
    lod: u32,
    bounds: BoundingBox,
    octree: HashMap<LODKey, PointCloudUnit>,
}

impl PointCloudMap {
    /// Create a root octree with the given bounds and points.
    pub fn root(bounds: BoundingBox, points: &Vec<Point>) -> Self {
        Self {
            lod: 0,
            bounds,
            octree: vec![(
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

    /// Divide the octree into 8 sub octrees.
    pub fn divide(&self, threshold: usize) -> Self {
        let next_lod = self.lod + 1;
        let div = 2_f64.powf(next_lod as f64);
        let min = &self.bounds.min;
        let unit = self.bounds.max_size() / div;

        let mut next = HashMap::new();

        self.octree.iter().for_each(|(_k, v)| {
            if v.points.len() > threshold {
                let pts: Vec<(LODKey, Point)> = v
                    .points
                    .par_iter()
                    .map(|v| {
                        let position = v.position;
                        let u = (position - min) / unit;
                        let k = u.map(|v| v.floor().min(div - 1.) as i32);
                        let key = (k.x, k.y, k.z);
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
            octree: next
                .into_iter()
                .map(|pts| {
                    let (key, points) = pts;
                    (key, PointCloudUnit { points })
                })
                .collect(),
        }
    }

    pub fn map(&self) -> &HashMap<LODKey, PointCloudUnit> {
        &self.octree
    }
}
