# async-wormhole

[Documentation](https://docs.rs/async-wormhole/0.1.1/async_wormhole/)

> This library is still super experimental, I use it to prototype the foundation for
> [Lunatic](https://dev.to/bkolobara/lunatic-actor-based-webassembly-runtime-for-the-backend-36oj) .
>
> **Currently only works in Rust nightly, as it depends on [switcheroo](https://github.com/bkolobara/async-wormhole/tree/master/switcheroo).**

async-wormhole allows you to call .await async calls across non-async functions, like extern "C" or JIT generated code.
It runs on Windows, MacOs and Linux (x64 & AArch64).

## Motivation

Sometimes, when running inside an async environment you need to call into JIT generated code (e.g. wasm)
and .await from there. Because the JIT code is not available at compile time, the Rust compiler can't
do their "create a state machine" magic. In the end you can't have `.await` statements in non-async
functions.

This library creates a special stack for executing the JIT code, so it's possible to suspend it at any
point of the execution. Once you pass it a closure inside `AsyncWormhole::new` you will get back a future
that you can `.await` on. The passed in closure is going to be executed on a new stack.

Sometimes you also need to preserve thread local storage as the code inside the closure expects it to stay
the same, but the actual execution can be moved between threads. There is a proof of concept API to allow
you to move your thread local storage with the closure across threads.

## Example

```rust
use async_wormhole::{AsyncWormhole, AsyncYielder};

// non-async function
extern "C" fn non_async(mut yielder: AsyncYielder<u32>) -> u32 {
	// Suspend the runtime until async value is ready.
	// Can contain .await calls.
    yielder.async_suspend(async { 42 })
}

fn main() {
    let task: AsyncWormhole<u32, ()> = AsyncWormhole::new(|yielder| {
        let result = non_async(yielder);
        assert_eq!(result, 42);
        64
    })
    .unwrap();

    let outside = futures::executor::block_on(task);
    assert_eq!(outside.unwrap(), 64);
}
```

## Performance

There should be almost no performance overhead to `.await` calls inside the closure passed to
`AsyncWormhole::new` and caught by `async_suspend`.
But instantiating a new AsyncWormhole will require one memory allocation.
And of course you are not going to get perfectly sized stacks.

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
