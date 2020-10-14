#![feature(asm, naked_functions)]

//! Switcheroo provides lightweight context switches in Rust.
//!
//! It consists of two parts:
//! 1. A stack implementation (currently only providing a [fixed 8Mb stack](stack/struct.EightMbStack.html)).
//! 2. A [generator](struct.Generator.html) implementation.
//! ## Example
//! ```
//! use switcheroo::stack::*;
//! use switcheroo::Generator;
//!
//! fn  main() {
//! 	let stack = EightMbStack::new().unwrap();
//! 	let  mut add_one = Generator::new(stack, |yielder, mut input| {
//! 		loop {
//! 			if input ==  0 {
//! 				break;
//! 			}
//! 			input = yielder.suspend(input +  1);
//! 		}
//! 	});
//!
//! 	assert_eq!(add_one.resume(2), Some(3));
//! 	assert_eq!(add_one.resume(127), Some(128));
//! 	assert_eq!(add_one.resume(0), None);
//! 	assert_eq!(add_one.resume(0), None);
//! }
// ```

mod arch;
pub mod stack;

use std::cell::Cell;
use std::marker::PhantomData;
use std::{mem, ptr};

/// Generator wraps a closure and allows suspending its execution more than once, returning
/// a value each time.
///
/// If the closure finishes each other call to [resume](struct.Generator.html#method.resume)
/// will yield `None`.
pub struct Generator<'a, Input: 'a, Output: 'a, Stack: stack::Stack> {
    stack: Stack,
    stack_ptr: Option<ptr::NonNull<usize>>,
    phantom: PhantomData<(&'a (), *mut Input, *const Output)>,
}

impl<'a, Input, Output, Stack> Generator<'a, Input, Output, Stack>
where
    Input: 'a,
    Output: 'a,
    Stack: stack::Stack,
{
    /// Create a new generator from a stack and closure.
    pub fn new<F>(stack: Stack, f: F) -> Generator<'a, Input, Output, Stack>
    where
        F: FnOnce(&Yielder<Input, Output>, Input) + 'a,
    {
        unsafe extern "C" fn generator_wrapper<Input, Output, Stack, F>(
            f_ptr: usize,
            stack_ptr: *mut usize,
        ) where
            Stack: stack::Stack,
            F: FnOnce(&Yielder<Input, Output>, Input),
        {
            let f = std::ptr::read(f_ptr as *const F);
            let (data, stack_ptr) = arch::swap(0, stack_ptr);
            let input = std::ptr::read(data as *const Input);
            let yielder = Yielder::new(stack_ptr);

            f(&yielder, input);
            // On last invocation of `suspend` return None
            yielder.suspend_(None);
        }

        let stack_ptr = unsafe { arch::init(&stack, generator_wrapper::<Input, Output, Stack, F>) };
        let f = mem::ManuallyDrop::new(f);
        let stack_ptr = unsafe {
            arch::swap_and_link_stacks(
                &f as *const mem::ManuallyDrop<F> as usize,
                stack_ptr,
                stack.bottom(),
            )
            .1
        };

        Generator {
            stack,
            stack_ptr: Some(ptr::NonNull::new(stack_ptr).unwrap()),
            phantom: PhantomData,
        }
    }

    /// Resume the generator yielding the next value.
    #[inline(always)]
    pub fn resume(&mut self, input: Input) -> Option<Output> {
        if self.stack_ptr.is_none() {
            return None;
        };
        let stack_ptr = self.stack_ptr.unwrap();
        self.stack_ptr = None;
        unsafe {
            let input = mem::ManuallyDrop::new(input);
            let (data_out, stack_ptr) = arch::swap_and_link_stacks(
                &input as *const mem::ManuallyDrop<Input> as usize,
                stack_ptr.as_ptr(),
                self.stack.bottom(),
            );

            // Should always be a pointer and never 0
            if data_out == 0 {
                return None;
            } else {
                self.stack_ptr = Some(ptr::NonNull::new_unchecked(stack_ptr));
                Some(std::ptr::read(data_out as *const Output))
            }
        }
    }

    /// Consume the generator and extract the stack.
    pub fn stack(self) -> Stack {
        self.stack
    }
}

/// Yielder is an interface provided to every generator through which it returns a value.
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

    /// Suspends the generator and returns `Some(val)` from the `resume()` invocation that resumed
    /// the generator.
    #[inline(always)]
    pub fn suspend(&self, val: Output) -> Input {
        unsafe { self.suspend_(Some(val)) }
    }

    #[inline(always)]
    unsafe fn suspend_(&self, val: Option<Output>) -> Input {
        match val {
            None => {
                // Let the resume know we are done here
                arch::swap(0, self.stack_ptr.get());
                unreachable!();
            }
            Some(val) => {
                let val = mem::ManuallyDrop::new(val);
                let (data, stack_ptr) = arch::swap(
                    &val as *const mem::ManuallyDrop<Output> as usize,
                    self.stack_ptr.get(),
                );
                self.stack_ptr.set(stack_ptr);

                std::ptr::read(data as *const Input)
            }
        }
    }
}
