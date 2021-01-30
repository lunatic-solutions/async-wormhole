use std::cell::Cell;
use std::ptr;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

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
            AsyncWormhole::<_, _, fn(), fn()>::new(stack, |mut yielder| {
                yielder.async_suspend(async { 42 });
            })
            .unwrap();
        })
    });

    c.bench_function("async switch", |b| {
        b.iter_batched(
            || {
                let stack = EightMbStack::new().unwrap();
                let async_ = AsyncWormhole::<_, _, fn(), fn()>::new(stack, |mut yielder| {
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

    c.bench_function("async switch with pre and post poll hooks", |b| {
        b.iter_batched(
            || {
                let stack = EightMbStack::new().unwrap();
                let mut async_ = AsyncWormhole::<_, _, fn(), fn()>::new(stack, |mut yielder| {
                    yielder.async_suspend(async { 42 });
                })
                .unwrap();
                async_.set_pre_poll(|| {
                    let _ = 33 + 34;
                });
                // post_poll_pending will never be called, because future resolves with value on first try.
                async_
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
