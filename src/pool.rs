use std::cell::Cell;
use std::io::Error;
use std::thread::LocalKey;

use crossbeam::queue::ArrayQueue;
use switcheroo::stack::*;

use super::{AsyncWormhole, AsyncYielder};

/// A pool of AsyncWormholes.
/// Creating an AsyncWormholes can be costly, as they need a memory allocation for the stack.
/// OneMbAsyncPool keeps a pool of 1 Mb stacks ready to create new AsyncWormholes "fast".
///
/// ### Safety
///
/// The stack is not cleared before reuse and may contain sensitive data from the previous use.
pub struct OneMbAsyncPool {
    pool: ArrayQueue<OneMbStack>,
}

unsafe impl Sync for OneMbAsyncPool {}

impl OneMbAsyncPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            pool: ArrayQueue::new(capacity),
        }
    }

    pub fn with_tls<'a, F, Output, TLS, const TLS_COUNT: usize>(
        &self,
        tls: [&'static LocalKey<Cell<*const TLS>>; TLS_COUNT],
        f: F,
    ) -> Result<AsyncWormhole<'a, OneMbStack, Output, TLS, TLS_COUNT>, Error>
    where
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        match self.pool.pop() {
            None => {
                let stack = OneMbStack::new()?;
                let wormhole = AsyncWormhole::new_with_tls(tls, stack, f)?;
                Ok(wormhole)
            }
            Some(stack) => {
                let wormhole = AsyncWormhole::new_with_tls(tls, stack, f)?;
                Ok(wormhole)
            }
        }
    }

    pub fn recycle<Output, TLS, const TLS_COUNT: usize>(
        &self,
        async_wormhole: AsyncWormhole<OneMbStack, Output, TLS, TLS_COUNT>,
    ) {
        // If we push over the capacity just drop the stack.
        let _ = self.pool.push(async_wormhole.stack());
    }
}
