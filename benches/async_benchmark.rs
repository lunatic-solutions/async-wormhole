use std::cell::Cell;
use std::ptr;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use async_wormhole::pool::OneMbAsyncPool;
use async_wormhole::AsyncWormhole;
use switcheroo::stack::*;

thread_local!(
    /// Mock TLS
    pub static TLS: Cell<*const usize> = Cell::new(ptr::null())
);

fn async_bench(c: &mut Criterion) {
    c.bench_function("async_wormhole creation", |b| {
        b.iter(|| {
            let stack = EightMbStack::new().unwrap();
            AsyncWormhole::new(stack, |mut yielder| {
                yielder.async_suspend(async { 42 });
            })
            .unwrap();
        })
    });

    c.bench_function("async_wormhole creation with pool", |b| {
        let pool = OneMbAsyncPool::new(128);
        b.iter(|| {
            let wormhole = pool
                .with_tls([&TLS], |mut yielder| {
                    yielder.async_suspend(async { 42 });
                })
                .unwrap();
            pool.recycle(wormhole);
        })
    });

    c.bench_function("async switch", |b| {
        b.iter_batched(
            || {
                let stack = EightMbStack::new().unwrap();
                let async_ = AsyncWormhole::new(stack, |mut yielder| {
                    yielder.async_suspend(async { 42 });
                })
                .unwrap();
                async_
            },
            |mut task| {
                futures::executor::block_on(&mut task);
                task
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("async switch with TLS", |b| {
        b.iter_batched(
            || {
                let pool = OneMbAsyncPool::new(128);
                pool.with_tls([&TLS], |mut yielder| {
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
