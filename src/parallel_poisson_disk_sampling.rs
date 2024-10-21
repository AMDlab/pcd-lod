use std::collections::HashSet;

use itertools::Itertools;
use nalgebra::{
    allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, RealField, Vector, Vector3,
    U3,
};
use num_traits::ToPrimitive;
use rand::seq::SliceRandom;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{grid::Grid, has_position::HasPosition, misc::min_max, point::Point};

#[derive(Debug)]
pub struct ParallelPoissonDiskSampling<'a> {
    radius: f64,
    half_radius: f64,
    grid: Vec<Vec<Vec<Grid<'a, Point>>>>,
    grid_count: Vector3<usize>,
    grid_min: Vector3<f64>,
    grid_max: Vector3<f64>,
    partitions: Vec<Vector3<usize>>,
    partitions_count: usize,
    grid_cell_size: f64,
}

impl<'a> ParallelPoissonDiskSampling<'a> {
    pub fn new(inputs: Vec<&'a Point>, radius: f64) -> Self {
        let (grid_min, grid_max) = min_max(inputs.iter().map(|pt| pt.position()));
        let size = grid_max - grid_min;

        // `cell_size` refers following article
        // https://sighack.com/post/poisson-disk-sampling-bridsons-algorithm
        // "Understanding the Cell Size" section
        let grid_cell_size = radius / (3.0_f64).sqrt();
        let half_radius = radius / 2.;

        let grid_count = size.map(|v| (v / grid_cell_size).ceil().max(1.) as usize);

        // println!("grid_size: {:?}", u_grid_size);
        let mut grid: Vec<Vec<Vec<Grid<'_, Point>>>> = vec![];
        for _ in 0..grid_count.z {
            let mut gz = vec![];
            for _ in 0..grid_count.y {
                let mut gy = vec![];
                for _ in 0..grid_count.x {
                    gy.push(Grid::new());
                }
                gz.push(gy);
            }
            grid.push(gz);
        }

        inputs.iter().for_each(|pt| {
            let i = index(pt.position(), &grid_min, grid_cell_size);
            grid[i.z][i.y][i.x].insert(pt);
        });

        // 3 x 3 x 3 partitions
        let mut partitions = (0..3)
            .flat_map(|z| {
                (0..3)
                    .flat_map(|y| {
                        (0..3)
                            .filter_map(|x| {
                                let v = Vector3::new(x, y, z);
                                if v.x < grid_count.x && v.y < grid_count.y && v.z < grid_count.z {
                                    Some(v)
                                } else {
                                    None
                                }
                            })
                            .collect_vec()
                    })
                    .collect_vec()
            })
            .collect_vec();

        // println!("partitions: {:?}", &partitions);

        // randomize orders of addresses
        let mut rng = rand::thread_rng();
        partitions.shuffle(&mut rng);

        let partitions_count = partitions.len();

        Self {
            radius,
            half_radius,
            grid,
            grid_count,
            grid_min,
            grid_max,
            grid_cell_size,
            partitions,
            partitions_count,
        }
    }

    pub fn samples(&self) -> Vec<&Point> {
        self.grid
            .iter()
            .flatten()
            .flatten()
            .filter_map(|g| g.representative())
            .collect()
    }

    pub fn is_completed(&self) -> bool {
        self.partitions.is_empty()
    }

    pub fn max_iterations(&self) -> usize {
        self.partitions_count
    }

    pub fn step(&mut self) -> anyhow::Result<()> {
        let divs = self.grid_count.map(|i| (i as f64 / 3_f64).ceil() as usize);
        let address = self.partitions.pop().ok_or(anyhow::anyhow!("no address"))?;
        // println!("address: {:?}", address);

        let count = self.grid_count.clone();
        let items = (0..=divs.z)
            .flat_map(|z| {
                (0..=divs.y).flat_map(move |y| {
                    (0..=divs.x).filter_map(move |x| {
                        let item = Vector3::new(x, y, z) * 3 + address;
                        if item.x >= count.x || item.y >= count.y || item.z >= count.z {
                            None
                        } else {
                            Some(item)
                        }
                    })
                })
            })
            .collect_vec();

        if self.partitions.len() + 1 == self.partitions_count {
            // sample at first time
            let seeds = items
                .into_par_iter()
                .filter_map(|addr| {
                    let g = &self.grid[addr.z][addr.y][addr.x];
                    g.candidates().first().cloned()
                })
                .collect::<Vec<_>>();

            for pt in seeds {
                let i = index(pt.position(), &self.grid_min, self.grid_cell_size);
                self.grid[i.z][i.y][i.x].set(pt.clone());
            }
        } else {
            let next = items
                .into_par_iter()
                .filter_map(|i| {
                    let g = &self.grid[i.z][i.y][i.x];
                    g.candidates()
                        .par_iter()
                        .find_any(|p| self.is_valid(p))
                        .cloned()
                })
                .collect::<Vec<_>>();
            // println!("#next: {}", next.len());
            for pt in next {
                let i = index(pt.position(), &self.grid_min, self.grid_cell_size);
                self.grid[i.z][i.y][i.x].set(pt.clone());
            }
        }

        Ok(())
    }

    fn is_valid(&self, p: &Point) -> bool {
        let i = index(p.position(), &self.grid_min, self.grid_cell_size);

        for dz in -1..=1 {
            let z = i.z as isize + dz;
            if 0 <= z && z < self.grid_count.z as isize {
                for dy in -1..=1 {
                    let y = i.y as isize + dy;
                    if 0 <= y && y < self.grid_count.y as isize {
                        for dx in -1..=1 {
                            if dz == 0 && dy == 0 && dx == 0 {
                                continue;
                            }
                            let x = i.x as isize + dx;
                            if 0 <= x && x < self.grid_count.x as isize {
                                if let Some(q) =
                                    self.grid[z as usize][y as usize][x as usize].representative()
                                {
                                    let dist = (p.position() - q.position()).norm();
                                    if dist <= self.radius {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        true
    }
}

fn index(point: &OPoint<f64, U3>, grid_min: &Vector3<f64>, cell_size: f64) -> Vector3<usize> {
    let n = point.coords - grid_min;
    n.map(|x| (x / cell_size).floor().to_usize().unwrap())
}
