//! async-wormhole allows you to call `.await` async calls across non-async functions, like extern "C" or JIT
//! generated code.
//!
//! ## Motivation
//!
//! Sometimes, when running inside an async environment you need to call into JIT generated code (e.g. wasm)
//! and .await from there. Because the JIT code is not available at compile time, the Rust compiler can't
//! do their "create a state machine" magic. In the end you can't have `.await` statements in non-async
//! functions.
//!
//! This library creates a special stack for executing the JIT code, so it's possible to suspend it at any
//! point of the execution. Once you pass it a closure inside [AsyncWormhole::new](struct.AsyncWormhole.html#method.new)
//! you will get back a future that you can `.await` on. The passed in closure is going to be executed on a
//! new stack.
//!
//! Sometimes you also need to preserve thread local storage as the code inside the closure expects it to stay
//! the same, but the actual execution can be moved between threads. There is a
//! [proof of concept API](struct.AsyncWormhole.html#method.preserve_tls)
//! to allow you to move your thread local storage with the closure across threads.
//!
//! ## Example
//!
//! ```rust
//! use async_wormhole::{AsyncWormhole, AsyncYielder};
//!
//! // non-async function
//! extern "C" fn non_async(mut yielder: AsyncYielder<u32>) -> u32 {
//! 	// Suspend the runtime until async value is ready.
//! 	// Can contain .await calls.
//!     yielder.async_suspend(async { 42 })
//! }
//!
//! fn main() {
//!     let task: AsyncWormhole<u32, ()> = AsyncWormhole::new(|yielder| {
//!         let result = non_async(yielder);
//!         assert_eq!(result, 42);
//!         64
//!     })
//!     .unwrap();
//!
//!     let outside = futures::executor::block_on(task);
//!     assert_eq!(outside.unwrap(), 64);
//! }
//! ```

use switcheroo::stack::*;
use switcheroo::Generator;
use switcheroo::Yielder;

use std::ptr;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::thread::LocalKey;
use std::cell::Cell;

/// This structure holds one thread local variable that is preserved across context switches.
/// This gives code that use thread local variables inside the closure the impression that they are
/// running on the same thread they started even if they have been moved to a different tread.
/// TODO: This code is currently higly specific to WASMTIME's signal handler TLS and could be
/// generalized. The only issue is that we can't have `Default` traits on pointers and we need to
/// get rid of *const TLS in Wasmtime.
struct ThreadLocal<TLS: 'static> {
    ptr: &'static LocalKey<Cell<*const TLS>>,
    value: *const TLS,
}

pub struct AsyncWormhole<'a, Output, TLS: 'static> {
    generator: Cell<Generator<'a, Waker, Option<Output>, EightMbStack>>,
    thread_local: Option<ThreadLocal<TLS>>
}

unsafe impl<Output, TLS> Send for AsyncWormhole<'_, Output, TLS> {}

impl<'a, Output, TLS> AsyncWormhole<'a, Output, TLS> {
    /// Takes a closure and returns an `impl Future` that can be awaited on.
    pub fn new<F>(f: F) -> Result<Self, Error>
    where
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        let stack = EightMbStack::new()?;
        let generator = Generator::new(stack, |yielder, waker| {
            let async_yielder = AsyncYielder::new(yielder, waker);
            yielder.suspend(Some(f(async_yielder)));
        });

        Ok(Self { generator: Cell::new(generator), thread_local: None })
    }

    /// Takes a reference to the to be preserved TLS variable.
    pub fn preserve_tls(&mut self, tls: &'static LocalKey<Cell<*const TLS>>) {
        self.thread_local = Some(ThreadLocal {
            ptr: tls,
            value: ptr::null(),
        });
    }
}

impl<'a, Output, TLS: Unpin> Future for AsyncWormhole<'a, Output, TLS> {
    type Output = Option<Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If we saved a TLS value, put it back in.
        // If this is the first `poll` it will overwrite the existing TLS value with null.
        match &self.thread_local {
            None => {},
            Some(tls) => {
                tls.ptr.with(|v| v.set(tls.value))
            }
        };

        match self.generator.get_mut().resume(cx.waker().clone()) {
            // If we call the future after it completed it will always return Poll::Pending.
            // But polling a completed future is either way undefined behaviour.
            None | Some(None) => {
                // Preserve any TLS value if set
                match self.thread_local.take() {
                    None => {},
                    Some(mut tls) => tls.ptr.with(|v|
                        tls.value = v.get()
                    )
                };
                Poll::Pending
            },
            Some(out) => Poll::Ready(out),
        }
    }
}

pub struct AsyncYielder<'a, Output> {
    yielder: &'a Yielder<Waker, Option<Output>>,
    waker: Waker,
}

impl<'a, Output> AsyncYielder<'a, Output> {
    pub(crate) fn new(yielder: &'a Yielder<Waker, Option<Output>>, waker: Waker) -> Self {
        Self { yielder, waker }
    }

    /// Takes an `impl Future` and awaits it, returning the value from it once ready.
    pub fn async_suspend<Fut, R>(&mut self, future: Fut) -> R
    where
        Fut: Future<Output = R>,
    {
        pin_utils::pin_mut!(future);
        loop {
            let cx = &mut Context::from_waker(&mut self.waker);
            self.waker = match future.as_mut().poll(cx) {
                Poll::Pending => self.yielder.suspend(None),
                Poll::Ready(result) => return result,
            };
        }
    }
}
