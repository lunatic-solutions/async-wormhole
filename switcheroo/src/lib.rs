#![feature(asm, naked_functions)]

mod arch;
pub mod stack;

use std::cell::Cell;
use std::marker::PhantomData;

pub struct Generator<'a, Input: 'a, Output: 'a, Stack: stack::Stack> {
    stack: Stack,
    stack_ptr: *mut usize,
    phantom: PhantomData<(&'a (), *mut Input, *const Output)>,
}

impl<'a, Input, Output, Stack> Generator<'a, Input, Output, Stack>
where
    Input: 'a,
    Output: 'a,
    Stack: stack::Stack,
{
    pub fn new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
    where
        F: FnOnce(&Yielder<Input, Output>, Input) + 'a,
    {
        unsafe extern "C" fn generator_wrapper<Input, Output, Stack, F>(
            f_ptr: usize,
            stack_ptr: *mut usize,
        ) -> !
        where
            Stack: stack::Stack,
            F: FnOnce(&Yielder<Input, Output>, Input),
        {
            let f = std::ptr::read(f_ptr as *const F);
            let (data, stack_ptr) = arch::swap(0, stack_ptr);
            let input = std::ptr::read(data as *const Input);
            let yielder = Yielder::new(stack_ptr);
            f(&yielder, input);
            // Any other call to resume will just yield back.
            loop {
                yielder.suspend(None);
            }
        }

        let stack_ptr = unsafe { arch::init(&stack, generator_wrapper::<Input, Output, Stack, F>) };
        let stack_ptr = unsafe {
            arch::swap_and_link_stacks(&f as *const F as usize, stack_ptr, stack.bottom()).1
        };
        // We can't drop f when returning from this function. Maybe store it inside the Generator struct so it
        // doesn't get dropped before the generator.
        std::mem::forget(f);

        Generator {
            stack,
            stack_ptr,
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn resume(&mut self, input: Input) -> Option<Output> {
        let (data_out, stack_ptr) = unsafe {
            arch::swap_and_link_stacks(
                &input as *const Input as usize,
                self.stack_ptr,
                self.stack.bottom(),
            )
        };
        self.stack_ptr = stack_ptr;

        std::mem::forget(input);
        unsafe { std::ptr::read(data_out as *const Option<Output>) }
    }
}

pub struct Yielder<Input, Output> {
    stack_ptr: Cell<*mut usize>,
    phantom: PhantomData<(*const Input, *mut Output)>,
}

impl<Input, Output> Yielder<Input, Output> {
    fn new(stack_ptr: *mut usize) -> Yielder<Input, Output> {
        Yielder {
            stack_ptr: Cell::new(stack_ptr),
            phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn suspend(&self, val: Option<Output>) -> Input {
        unsafe {
            let (data, stack_ptr) =
                arch::swap(&val as *const Option<Output> as usize, self.stack_ptr.get());
            self.stack_ptr.set(stack_ptr);
            std::mem::forget(val);
            std::ptr::read(data as *const Input)
        }
    }
}
