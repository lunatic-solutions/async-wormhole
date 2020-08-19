use criterion::{criterion_group, criterion_main, Criterion};

use switcheroo::stack::*;
use switcheroo::Generator;

fn switcheroo(c: &mut Criterion) {
    // Bench allocation
    c.bench_function("create 8 MB stack", |b| b.iter(|| EightMbStack::new()));

    c.bench_function("switch stacks", |b| {
        let stack = EightMbStack::new().unwrap();
        let mut gen = Generator::new(stack, |yielder, input| {
            yielder.suspend(Some(input + 1));
        });
        b.iter(|| gen.resume(2))
    });
}

criterion_group!(benches, switcheroo);
criterion_main!(benches);
