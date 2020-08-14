#![feature(asm)]

mod arch;

use std::marker::PhantomData;
use std::cell::Cell;

pub struct Generator<'a, Input: 'a, Output: 'a, Stack: stackpp::Stack> {
    stack: Option<Stack>,
    stack_ptr: *mut u8,
    phantom: PhantomData<(&'a (), *mut Input, *const Output)>,
}

impl<'a, Input, Output, Stack> Generator<'a, Input, Output, Stack>
where
    Input: 'a,
    Output: 'a,
    Stack: stackpp::Stack,
{
    pub fn new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
    where
        F: FnOnce(&Yielder<Input, Output>, Input) + 'a,
    {
        unsafe extern "C" fn generator_wrapper<Input, Output, Stack, F>(f_ptr: usize, stack_ptr: *mut u8) -> !
            where Stack: stackpp::Stack,
                  F: FnOnce(&Yielder<Input, Output>, Input)
        {
            let f = std::ptr::read(f_ptr as *const F);
            let (data, stack_ptr) = arch::swap(0, stack_ptr);
            let input = std::ptr::read(data as *const Input);
            let yielder = Yielder::new(stack_ptr);
            f(&yielder, input);
            // Any other call to resume will just yield back.
            loop { yielder.suspend(None); }
        }

        let stack_ptr = unsafe { arch::init(&stack, generator_wrapper::<Input, Output, Stack, F>) };
        let stack_ptr = unsafe { arch::swap(&f as *const F as usize, stack_ptr).1 };
        // We can't drop f when returning from this function. Maybe store it inside the Generator struct so it
        // doesn't get dropped before the generator.
        std::mem::forget(f);

        Generator {
            stack: Some(stack),
            stack_ptr,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn resume(&mut self, input: Input) -> Option<Output> {
        let stack = self.stack.take();
        debug_assert!(stack.is_some());
        #[cfg(target_family = "unix")]
        stack.unwrap().give_to_signal(); // Unix only
        let (data_out, stack_ptr) = unsafe { arch::swap(&input as *const Input as usize, self.stack_ptr) };
        #[cfg(target_family = "unix")] // Unix only
        let stack = Stack::take_from_signal();
        debug_assert!(stack.is_some());
        self.stack = Some(stack.unwrap());
        self.stack_ptr = stack_ptr;
        std::mem::forget(input);
        unsafe { std::ptr::read(data_out as *const Option<Output>) }
    }
}

pub struct Yielder<Input, Output> {
    stack_ptr: Cell<*mut u8>,
    phantom: PhantomData<(*const Input, *mut Output)>,
}

impl<Input, Output> Yielder<Input, Output> {
    fn new(stack_ptr: *mut u8) -> Yielder<Input, Output> {
        Yielder {
            stack_ptr: Cell::new(stack_ptr),
            phantom: PhantomData
        }
    }

    #[inline(always)]
    pub fn suspend(&self, val: Option<Output>) -> Input {
        unsafe {
            let (data, stack_ptr) = arch::swap(&val as *const Option<Output> as usize, self.stack_ptr.get());
            self.stack_ptr.set(stack_ptr);
            std::mem::forget(val);
            std::ptr::read(data as *const Input)
        }
    }
}