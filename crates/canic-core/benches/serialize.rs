use criterion::Criterion;

///
/// BENCH TEMPLATE
///
/// Intentionally minimal stub kept for future benchmarking work.
/// Safe to compile; does not perform any measurements.
///

// Runs the placeholder benchmark body to keep this bench target valid.
const fn bench_stub(_criterion: &mut Criterion) {
    // Intentionally empty.
    // Future contributors can replace with real benchmarks.
}

// Executes the placeholder benchmark without parsing CLI args.
fn main() {
    let mut criterion = Criterion::default();
    bench_stub(&mut criterion);
    criterion.final_summary();
}
