use criterion::{criterion_group, criterion_main, Criterion};

use stackpp::*;

fn stackpp(c: &mut Criterion) {
    // Test allocation & drop
    c.bench_function("create 8 MB stack", |b| {
        b.iter(|| EightMbStack::new())
    });
}

criterion_group!(benches, stackpp);
criterion_main!(benches);
