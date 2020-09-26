use async_wormhole::{AsyncWormhole, AsyncYielder};
use switcheroo::stack::*;

// non-async function
#[allow(improper_ctypes_definitions)]
extern "C" fn non_async(mut yielder: AsyncYielder<u32>) -> u32 {
    // Suspend the runtime until async value is ready
    yielder.async_suspend(async { 42 })
}

fn main() {
    let stack = EightMbStack::new().unwrap();
    let task= AsyncWormhole::<_, _, ()>::new(stack, |yielder| {
        let result = non_async(yielder);
        assert_eq!(result, 42);
        64
    })
    .unwrap();

    let outside = futures::executor::block_on(task);
    assert_eq!(outside.unwrap(), 64);
}
