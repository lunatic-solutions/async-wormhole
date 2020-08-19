use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use async_wormhole::AsyncWormhole;

fn async_bench(c: &mut Criterion) {
    c.bench_function("async_wormhole creation", |b| {
        b.iter(|| {
            AsyncWormhole::new(|mut yielder| {
                yielder.async_suspend(async { 42 });
            })
            .unwrap()
        })
    });

    c.bench_function("async switch", |b| {
        b.iter_batched(
            || {
                AsyncWormhole::new(|mut yielder| {
                    yielder.async_suspend(async { 42 });
                })
                .unwrap()
            },
            |mut task| {
                futures::executor::block_on(&mut task);
                task
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, async_bench);
criterion_main!(benches);
