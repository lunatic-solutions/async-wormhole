use switcheroo::stack::*;
use switcheroo::Generator;
use switcheroo::Yielder;

use std::future::Future;
use std::io::Error;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

/// TODO: If poll finishes until completion the future should maybe return the last value.
/// But actually polling an already finished future is UB, so nothing really we need to do here.
pub struct AsyncWormhole<'a, Output> {
    generator: Generator<'a, std::task::Waker, Output, EightMbStack>,
}

impl<'a, Output> AsyncWormhole<'a, Output> {
    pub fn new<F>(f: F) -> Result<Self, Error>
    where
        F: FnOnce(AsyncYielder<Output>) -> Output + 'a,
    {
        let stack = EightMbStack::new()?;
        let generator = Generator::new(stack, |yielder, waker| {
            let async_yielder = AsyncYielder::new(yielder, waker);
            yielder.suspend(Some(f(async_yielder)));
        });

        Ok(Self { generator })
    }
}

impl<'a, Output> Future for AsyncWormhole<'a, Output> {
    type Output = Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.generator.resume(cx.waker().clone()) {
            None => Poll::Pending,
            Some(out) => Poll::Ready(out),
        }
    }
}

pub struct AsyncYielder<'a, Output> {
    yielder: &'a Yielder<Waker, Output>,
    waker: Waker,
}

impl<'a, Output> AsyncYielder<'a, Output> {
    pub(crate) fn new(yielder: &'a Yielder<Waker, Output>, waker: Waker) -> Self {
        Self { yielder, waker }
    }

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
