use criterion::criterion_main;

mod benchmarks;

criterion_main! {
    benchmarks::tokio::tokio_concurrent,
    benchmarks::tokio::tokio_sequential,
}
