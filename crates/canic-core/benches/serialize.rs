use criterion::{Criterion, criterion_group, criterion_main};

///
/// BENCH TEMPLATE
///
/// Intentionally minimal stub kept for future benchmarking work.
/// Safe to compile; does not perform any measurements.
///

const fn bench_stub(_c: &mut Criterion) {
    // Intentionally empty.
    // Future LLM / developer can replace with real benchmarks.
}

criterion_group!(benches, bench_stub);
criterion_main!(benches);
