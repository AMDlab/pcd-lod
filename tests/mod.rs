use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use pcd_lod::prelude::{ParallelPoissonDiskSampling, Point, PoissonDiskSampling};

fn criterion_benchmark(c: &mut Criterion) {
    let pcd = include_str!("../data/pcd.txt");
    let points = pcd
        .lines()
        .filter_map(|line| {
            let line = line.split_terminator(",").join(" ");
            Point::try_parse(&line).ok()
        })
        .collect_vec();

    let radius = 5.;

    c.bench_function("non parallels", |b| {
        b.iter(|| {
            let sampler = PoissonDiskSampling::new();
            sampler.sample(&points, radius);
        })
    });

    c.bench_function("parallels", |b| {
        b.iter(|| {
            let mut sampler = ParallelPoissonDiskSampling::new(points.iter().collect(), radius);
            for _ in 0..=sampler.max_iterations() {
                let _ = sampler.step();
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
