use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use async_wormhole::AsyncWormhole;


fn async_bench(c: &mut Criterion) {
    c.bench_function("async switch", |b| {
        b.iter_batched_ref(
            || {
                AsyncWormhole::new(|_yielder| {
                    42
                }).unwrap()
            },
            |task| {
                futures::executor::block_on(task)
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, async_bench);
criterion_main!(benches);
