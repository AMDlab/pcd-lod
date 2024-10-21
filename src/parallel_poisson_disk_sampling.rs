use std::collections::HashSet;

use itertools::Itertools;
use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, RealField, U3};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{has_position::HasPosition, point::Point};

#[derive(Debug, Clone)]
pub struct ParallelPoissonDiskSampling<'a> {
    inputs: Vec<&'a Point>,
}

impl<'a> ParallelPoissonDiskSampling<'a> {
    pub fn new(inputs: Vec<&'a Point>) -> Self {
        Self { inputs }
    }
}

impl<'a> ParallelPoissonDiskSampling<'a> {}
