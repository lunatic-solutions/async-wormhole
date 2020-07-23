use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

use switcheroo::Generator;

fn switcheroo(c: &mut Criterion) {
    c.bench_function("switch between stacks", |b| {
        let stack = PreAllocatedStack::new(1 * 1024 * 1024).unwrap();
        b.iter_batched(
            || {
                Generator::new(&stack, |yielder, input| {
                    yielder.suspend(Some(input + 1));
                })
            },
            |mut generator| {
                generator.resume(2)
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, switcheroo);
criterion_main!(benches);
