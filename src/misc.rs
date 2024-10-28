use nalgebra::{allocator::Allocator, DefaultAllocator, DimName, OPoint, OVector, RealField};

pub fn min_max<'a, T: RealField + Copy + num_traits::ToPrimitive, D: DimName>(
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
