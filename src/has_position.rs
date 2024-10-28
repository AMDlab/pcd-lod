use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, RealField, U3};

use crate::point::Point;

pub trait HasPosition<T: RealField, D: DimName>: Clone
where
    DefaultAllocator: Allocator<D>,
{
    fn position(&self) -> &OPoint<T, D>;
}

impl HasPosition<f64, U3> for Point {
    fn position(&self) -> &OPoint<f64, U3> {
        &self.position
    }
}
