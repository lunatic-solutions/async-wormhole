#![feature(min_const_generics)]

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
//! use switcheroo::stack::*;
//!
//! // non-async function
//! extern "C" fn non_async(mut yielder: AsyncYielder<u32>) -> u32 {
//! 	// Suspend the runtime until async value is ready.
//! 	// Can contain .await calls.
//!     yielder.async_suspend(async { 42 })
//! }
//!
//! fn main() {
//!     let stack = EightMbStack::new().unwrap();
//!     let task = AsyncWormhole::new(stack, |yielder| {
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

pub mod pool;

use switcheroo::stack;
use switcheroo::Generator;
use switcheroo::Yielder;

use std::cell::Cell;
use std::convert::TryInto;
use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::thread::LocalKey;

// This structure holds one thread local variable that is preserved across context switches.
// This gives code that use thread local variables inside the closure the impression that they are
// running on the same thread they started even if they have been moved to a different one.
struct ThreadLocal<TLS: 'static> {
    reference: &'static LocalKey<Cell<*const TLS>>,
    value: *const TLS,
}

impl<TLS> Copy for ThreadLocal<TLS> {}
impl<TLS> Clone for ThreadLocal<TLS> {
    fn clone(&self) -> Self {
        ThreadLocal {
            reference: self.reference,
            value: self.value,
        }
    }
}

/// AsyncWormhole captures a stack and a closure. It also implements Future and can be awaited on.
pub struct AsyncWormhole<'a, Stack: stack::Stack, Output, TLS: 'static, const TLS_COUNT: usize> {
    generator: Cell<Generator<'a, Waker, Option<Output>, Stack>>,
    preserved_thread_locals: [ThreadLocal<TLS>; TLS_COUNT],
}

unsafe impl<Stack: stack::Stack, Output, TLS, const TLS_COUNT: usize> Send
    for AsyncWormhole<'_, Stack, Output, TLS, TLS_COUNT>
{
}

impl<'a, Stack: stack::Stack, Output> AsyncWormhole<'a, Stack, Output, (), 0> {
    /// Returns a new AsyncWormhole, using the passed `stack` to execute the closure `f` on.
    /// The closure will not be executed right away, only if you pass AsyncWormhole to an
    /// async executor (.await on it).
    pub fn new<F>(stack: Stack, f: F) -> Result<Self, Error>
    where
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        AsyncWormhole::new_with_tls([], stack, f)
    }
}

impl<'a, Stack: stack::Stack, Output, TLS, const TLS_COUNT: usize>
    AsyncWormhole<'a, Stack, Output, TLS, TLS_COUNT>
{
    /// Similar to `new`, but allows you to capture thread local variables inside the closure.
    /// During the execution of the future an async executor can move the closure `f` between
    /// threads. From the perspective of the code inside the closure `f` the thread local
    /// variables will be moving with it from thread to thread.
    ///
    /// ### Safety
    ///
    /// If the thread local variable is only set and used inside of the `f` closure than it's safe
    ///  to use it. Outside of the closure the content of it will be unpredictable.
    pub fn new_with_tls<F>(
        tls_refs: [&'static LocalKey<Cell<*const TLS>>; TLS_COUNT],
        stack: Stack,
        f: F,
    ) -> Result<Self, Error>
    where
        // TODO: This needs to be Send, but because Wasmtime's strucutres are not Send for now I don't
        // enforce it on an API level. Accroding to
        // https://github.com/bytecodealliance/wasmtime/issues/793#issuecomment-692740254
        // it is safe to move everything connected to a Store to a different thread all at once, but this
        // is impossible to express with the type system.
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        let generator = Generator::new(stack, |yielder, waker| {
            let async_yielder = AsyncYielder::new(yielder, waker);
            yielder.suspend(Some(f(async_yielder)));
        });

        let preserved_thread_locals = tls_refs
            .iter()
            .map(|tls_ref| ThreadLocal {
                reference: tls_ref,
                value: tls_ref.with(|v| v.get()),
            })
            .collect::<Vec<ThreadLocal<TLS>>>()
            .as_slice()
            .try_into()
            .unwrap();

        Ok(Self {
            generator: Cell::new(generator),
            preserved_thread_locals,
        })
    }

    /// Get the stack from the internal generator.
    pub fn stack(self) -> Stack {
        self.generator.into_inner().stack()
    }
}

impl<'a, Stack: stack::Stack + Unpin, Output, TLS: Unpin, const TLS_COUNT: usize> Future
    for AsyncWormhole<'a, Stack, Output, TLS, TLS_COUNT>
{
    type Output = Option<Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Restore thread local values when re-entering execution
        for tls in self.preserved_thread_locals.iter() {
            tls.reference.with(|v| v.set(tls.value));
        }

        match self.generator.get_mut().resume(cx.waker().clone()) {
            // If we call the future after it completed it will always return Poll::Pending.
            // But polling a completed future is either way undefined behaviour.
            None | Some(None) => {
                // Preserve all thread local values
                for tls in self.preserved_thread_locals.iter_mut() {
                    tls.reference.with(|v| tls.value = v.get());
                }
                Poll::Pending
            }
            Some(out) => Poll::Ready(out),
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
