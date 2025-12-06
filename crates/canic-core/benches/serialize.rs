use criterion::{Criterion, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use std::hint::black_box;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CanisterPoolEntry {
    created_at: u64,
    cycles: u64,
}

fn bench_minicbor(c: &mut Criterion) {
    let entry = CanisterPoolEntry {
        created_at: 123_456_789,
        cycles: 123_123_123, // must fit in u64 for minicbor
    };

    c.bench_function("minicbor serialize/deserialize", |b| {
        b.iter(|| {
            let bytes = minicbor_serde::to_vec(&entry).unwrap();
            let decoded: CanisterPoolEntry = minicbor_serde::from_slice(&bytes).unwrap();
            black_box(decoded)
        });
    });
}

// 👇 THIS is what makes it work
criterion_group!(benches, bench_minicbor);
criterion_main!(benches);
