use async_wormhole::AsyncWormhole;
use backtrace::Backtrace;
use switcheroo::stack::*;

fn main() {
    let stack = EightMbStack::new().unwrap();
    let task = AsyncWormhole::<_, _, fn()>::new(stack, |_yielder| {
        let b = Backtrace::new();
        println!("{:?}", b);
        panic!("Panic inside wormhole")
    })
    .unwrap();

    futures::executor::block_on(task);
}
