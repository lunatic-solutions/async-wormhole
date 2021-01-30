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
//! ## Example
//!
//! ```rust
//! use async_wormhole::{AsyncWormhole, AsyncYielder};
//! use switcheroo::stack::*;
//!
//! // non-async function
//! #[allow(improper_ctypes_definitions)]
//! extern "C" fn non_async(mut yielder: AsyncYielder<u32>) -> u32 {
//! 	// Suspend the runtime until async value is ready.
//! 	// Can contain .await calls.
//!     yielder.async_suspend(async { 42 })
//! }
//!
//! fn main() {
//!     let stack = EightMbStack::new().unwrap();
//!     let task = AsyncWormhole::<_, _, fn(), fn()>::new(stack, |yielder| {
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

use switcheroo::Generator;
use switcheroo::Yielder;

use std::cell::Cell;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

pub use switcheroo::stack;

/// AsyncWormhole represents a Future that uses a generator with a separate stack to execute a closure.
///
/// It has the capability to .await on other Futures in the closure using the received
/// [AsyncYielder](struct.AsyncYielder). Once all Futures have been awaited on AsyncWormhole will resolve
/// to the return value of the provided closure.
///
/// For fine-grained poll control, two additional functions can be provided:
/// * [AsyncWormhole::set_pre_poll](struct.AsyncWormhole.html#method.set_pre_poll)
/// * [AsyncWormhole::set_post_poll_pending](struct.AsyncWormhole.html#method.set_post_poll_pending)
/// One common use cases for them is to preserve some thread local state across the execution of the closure.
/// Every time an executor polls AsyncWormhole, the `pre_poll` function will be called and every time
/// AsyncWormhole returns `Poll::Pending`, `post_poll_pending` will be called. Between this two calls we
/// have a guarantee that the executor will not be able to move the execution another thread.
pub struct AsyncWormhole<'a, Stack, Output, P1, P2>
where
    Stack: stack::Stack,
    P1: Fn(),
    P2: Fn(),
{
    generator: Cell<Generator<'a, Waker, Option<Output>, Stack>>,
    pre_poll: Option<P1>,
    post_poll_pending: Option<P2>,
}

unsafe impl<Stack, Output, P1, P2> Send for AsyncWormhole<'_, Stack, Output, P1, P2>
where
    Stack: stack::Stack,
    P1: Fn(),
    P2: Fn(),
{
}

impl<'a, Stack, Output, P1, P2> AsyncWormhole<'a, Stack, Output, P1, P2>
where
    Stack: stack::Stack,
    P1: Fn(),
    P2: Fn(),
{
    /// Returns a new AsyncWormhole, using the passed `stack` to execute the closure `f` on.
    /// The closure will not be executed right away, only if you pass AsyncWormhole to an
    /// async executor (.await on it)
    pub fn new<F>(stack: Stack, f: F) -> Result<Self, Error>
    where
        // TODO: This needs to be Send, but because Wasmtime's structures are not Send for now I don't
        // enforce it on an API level. According to
        // https://github.com/bytecodealliance/wasmtime/issues/793#issuecomment-692740254
        // it is safe to move everything connected to a Store to a different thread all at once, but this
        // is impossible to express with the type system.
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        let generator = Generator::new(stack, |yielder, waker| {
            let async_yielder = AsyncYielder::new(yielder, waker);
            let finished = Some(f(async_yielder));
            yielder.suspend(finished);
        });

        Ok(Self {
            generator: Cell::new(generator),
            pre_poll: None,
            post_poll_pending: None,
        })
    }

    /// Sets a function that will be called when an executor polls AsyncWormhole.
    pub fn set_pre_poll(&mut self, f: P1) {
        self.pre_poll = Some(f);
    }

    /// Sets a function that will be called if AsyncWormhole needs to wait on another future to
    /// complete and returns `Poll::Pending` to the executor.
    ///
    /// Next time poll is called, it may happen on another thread, so here is a good point to
    /// preserve all thread local variables we may need to restore again in
    /// [AsyncWormhole::set_pre_poll](struct.AsyncWormhole.html#method.set_pre_poll)
    pub fn set_post_poll_pending(&mut self, f: P2) {
        self.post_poll_pending = Some(f);
    }

    /// Get the stack from the internal generator.
    pub fn stack(self) -> Stack {
        self.generator.into_inner().stack().unwrap()
    }
}

impl<'a, Stack, Output, P1, P2> Future for AsyncWormhole<'a, Stack, Output, P1, P2>
where
    Stack: stack::Stack + Unpin,
    P1: Fn() + Unpin,
    P2: Fn() + Unpin,
{
    type Output = Option<Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // If pre_poll is provided execute it before entering separate stack
        if let Some(pre_poll) = &self.pre_poll {
            pre_poll()
        }

        match self.generator.get_mut().resume(cx.waker().clone()) {
            // If we call the future after it completed it will always return Poll::Pending.
            // But polling a completed future is either way undefined behaviour.
            None | Some(None) => {
                // If post_poll_pending is provided execute it before returning a Poll::Pending
                if let Some(post_poll_pending) = &self.post_poll_pending {
                    post_poll_pending()
                }
                Poll::Pending
            }
            Some(out) => {
                // Poll one last time to finish the generator
                self.generator.get_mut().resume(cx.waker().clone());
                Poll::Ready(out)
            }
        }
    }
}

#[derive(Clone)]
pub struct AsyncYielder<'a, Output> {
    yielder: &'a Yielder<Waker, Option<Output>>,
    waker: Waker,
}

impl<'a, Output> AsyncYielder<'a, Output> {
    pub(crate) fn new(yielder: &'a Yielder<Waker, Option<Output>>, waker: Waker) -> Self {
        Self { yielder, waker }
    }

    /// Takes an `impl Future` and awaits it, returning the value from it once ready.
    pub fn async_suspend<Fut, R>(&mut self, mut future: Fut) -> R
    where
        Fut: Future<Output = R>,
    {
        loop {
            let future = unsafe { Pin::new_unchecked(&mut future) };
            let mut cx = Context::from_waker(&mut self.waker);
            self.waker = match future.poll(&mut cx) {
                Poll::Pending => self.yielder.suspend(None),
                Poll::Ready(result) => return result,
            };
        }
    }
}
