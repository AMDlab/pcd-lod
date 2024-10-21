use std::collections::HashSet;

use itertools::Itertools;
use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, RealField, U3};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::grid::Grid;
use crate::{has_position::HasPosition, point::Point};

#[derive(Debug, Clone)]
pub struct PoissonDiskSampling<T, P> {
    phantom: std::marker::PhantomData<(T, P)>,
}

impl<T, P> Default for PoissonDiskSampling<T, P> {
    fn default() -> Self {
        Self {
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T, P> PoissonDiskSampling<T, P> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: RealField + Copy + num_traits::ToPrimitive, P: HasPosition<T, U3> + Sync + Send>
    PoissonDiskSampling<T, P>
{
    pub fn sample(&self, inputs: &[P], radius: T) -> Vec<P> {
        let (min, max) = min_max(inputs.iter().map(|pt| pt.position()));
        let size = max - min;

        // `cell_size` refers following article
        // https://sighack.com/post/poisson-disk-sampling-bridsons-algorithm
        // "Understanding the Cell Size" section
        let cell_size = radius / T::from_usize(3).unwrap().sqrt();
        let half_radius = radius / T::from_usize(2).unwrap();
        // let radius_squared = radius * radius;
        // let radius_2_squared = (radius * T::from_usize(2).unwrap()).powi(2);

        let grid_size = size.map(|x| (x / cell_size).ceil().max(T::one()));
        let u_grid_size = grid_size.map(|x| x.to_usize().unwrap());
        // println!("grid_size: {:?}", u_grid_size);
        let mut grid: Vec<Vec<Vec<Grid<'_, P>>>> = vec![];
        for _ in 0..u_grid_size.z {
            let mut gz = vec![];
            for _ in 0..u_grid_size.y {
                let mut gy = vec![];
                for _ in 0..u_grid_size.x {
                    gy.push(Grid::new());
                }
                gz.push(gy);
            }
            grid.push(gz);
        }

        let index = |point: &OPoint<T, U3>| {
            let n = point.coords - min;
            n.map(|x| (x / cell_size).floor().to_usize().unwrap())
        };

        inputs.iter().for_each(|pt| {
            let i = index(pt.position());
            grid[i.z][i.y][i.x].insert(pt);
        });

        let mut indices: HashSet<(usize, usize, usize)> = grid
            .iter()
            .enumerate()
            .flat_map(|(iz, gz)| {
                gz.iter()
                    .enumerate()
                    .flat_map(|(iy, gy)| {
                        gy.iter()
                            .enumerate()
                            .filter_map(|(ix, g)| match !g.candidates().is_empty() {
                                true => Some(ix),
                                false => None,
                            })
                            .map(move |ix| (ix, iy))
                    })
                    .map(move |(ix, iy)| (ix, iy, iz))
            })
            .collect();

        // println!("indices: {:?}", indices.len());

        let mut actives = vec![];

        let insert = |p: P,
                      actives: &mut Vec<P>,
                      grid: &mut Vec<Vec<Vec<Grid<'_, P>>>>,
                      indices: &mut HashSet<(usize, usize, usize)>| {
            actives.push(p.clone());
            let i = index(p.position());
            grid[i.z][i.y][i.x].set(p.clone());
            indices.remove(&(i.x, i.y, i.z));
        };

        let is_valid = |p: &P, grid: &Vec<Vec<Vec<Grid<'_, P>>>>| {
            let i = index(p.position());
            for dz in -1..1 {
                let z = i.z as isize + dz;
                if 0 <= z && z < u_grid_size.z as isize {
                    for dy in -1..1 {
                        let y = i.y as isize + dy;
                        if 0 <= y && y < u_grid_size.y as isize {
                            for dx in -1..1 {
                                if dz == 0 && dy == 0 && dx == 0 {
                                    continue;
                                }
                                let x = i.x as isize + dx;
                                if 0 <= x && x < u_grid_size.x as isize {
                                    if let Some(q) =
                                        grid[z as usize][y as usize][x as usize].representative()
                                    {
                                        let dist = p.position() - q.position();
                                        if dist.norm() <= radius {
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
        };

        let i = *indices.iter().next().unwrap();
        indices.remove(&i);
        let start = grid[i.2][i.1][i.0].candidates().first().unwrap().clone();
        insert(start.clone(), &mut actives, &mut grid, &mut indices);

        while !indices.is_empty() {
            let current = match actives.is_empty() {
                true => {
                    let i = *indices.iter().next().unwrap();
                    indices.remove(&i);
                    let next = grid[i.2][i.1][i.0].candidates().iter().find_map(|p| {
                        if is_valid(p, &grid) {
                            Some(p)
                        } else {
                            None
                        }
                    });
                    match next {
                        Some(next) => {
                            insert((*next).clone(), &mut actives, &mut grid, &mut indices);
                        }
                        _ => {
                            indices.remove(&i);
                        }
                    }
                    // insert(next.clone(), &mut actives, &mut grid, &mut indices);
                    continue;
                }
                false => actives.first().unwrap(),
            };
            let i = index(current.position());
            let neighbor_indices = (-1..=1)
                .flat_map(|dz| {
                    let z = i.z as isize + dz;
                    if 0 <= z && z < u_grid_size.z as isize {
                        (-1..=1)
                            .flat_map(|dy| {
                                let y = i.y as isize + dy;
                                if 0 <= y && y < u_grid_size.y as isize {
                                    (-1..=1)
                                        .filter_map(|dx| {
                                            if dz == 0 && dy == 0 && dx == 0 {
                                                return None;
                                            }

                                            let x = i.x as isize + dx;
                                            if 0 <= x && x < u_grid_size.x as isize {
                                                let j = (x as usize, y as usize, z as usize);
                                                if grid[j.2][j.1][j.0].visited() {
                                                    None
                                                } else {
                                                    Some(j)
                                                }
                                            } else {
                                                None
                                            }
                                        })
                                        .collect_vec()
                                } else {
                                    vec![]
                                }
                            })
                            .collect_vec()
                    } else {
                        vec![]
                    }
                })
                .collect_vec();

            let next = neighbor_indices.into_iter().find_map(|(x, y, z)| {
                let cand = grid[z][y][x].candidates();
                cand.par_iter()
                    .find_any(|q| {
                        let dist_squared = (current.position() - q.position()).norm_squared();
                        // radius_squared <= dist_squared && dist_squared <= radius_2_squared
                        half_radius <= dist_squared.sqrt()
                            && dist_squared.sqrt() <= radius
                            && is_valid(q, &grid)
                        // is_valid(q, &grid)
                    })
                    .map(|next| (*next).clone())
            });

            match next {
                Some(p) => {
                    insert(p, &mut actives, &mut grid, &mut indices);
                }
                _ => {
                    actives.remove(0);
                }
            };
        }

        // collect result
        grid.into_iter()
            .flat_map(|gz| {
                gz.into_iter()
                    .flat_map(|gy| gy.into_iter().filter_map(|g| g.representative().cloned()))
            })
            .collect()
    }
}

fn min_max<'a, T: RealField + Copy + num_traits::ToPrimitive, D: DimName>(
    inputs: impl Iterator<Item = &'a OPoint<T, D>>,
) -> (OVector<T, D>, OVector<T, D>)
where
    DefaultAllocator: Allocator<D>,
{
    let mut min = OVector::<T, D>::from_element(T::max_value().unwrap());
    let mut max = -min.clone();

    for point in inputs {
        for i in 0..D::dim() {
            min[i] = min[i].min(point[i]);
            max[i] = max[i].max(point[i]);
        }
    }

    (min, max)
}
