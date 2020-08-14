use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::ptr;

use crate::Stack;
use super::page_size;

thread_local! {
    /// A stack growth is triggered by accessing a guard page. This will raise a signal with the OS and
    /// inside the signal handler the stack is extended. There is no other way of passing the currently
    /// used stack to the signal handler except saving it in a thread local variable. Signals generated
    /// in response to hardware exceptions, like SIGSEGV, SIGBUS, SIGILL, .. are called thread-directed
    /// signals and are guaranteed to be handled by the same thread that raised them.
    /// Every time we want to make the stack available to the signal handler we need to first call the
    /// `give_to_signal` method. To get back the stack we need to call `take_from_signal`.
    pub(crate) static CURRENT_STACK: Cell<Option<PreAllocatedStack>> = Cell::new(None);
}

/// Divdes the stack in 2 parts:
/// * A usable area (from bottom[excluding] to top[including]), that can be written to and read from.
/// * A guarded area (from top to guard_top), when accessed will trigger a OS signal.
/// This allows us to reserver a bigger virtual memory space from the OS, but only marking it read/write
/// once we actually need it. We assume here that virtual memory is cheap and that there is no big cost
/// in pre-allocating a big amount of it. This is generally true on all 64 bit Operating Systems.
/// This is also how Windows internally manages stacks of threads. It just keeps removing the guard page
/// until it reaches the maximum allowed stack size (the limit is kept in a the TIB[1).
/// We basically re-implement the default Windows behaviour in Unix operating systems.
/// [1] https://en.wikipedia.org/wiki/Win32_Thread_Information_Block
pub struct PreAllocatedStack {
    guard_top: *mut u8,
    top: *mut u8,
    bottom: *mut u8,
}

impl Stack for PreAllocatedStack {
    /// The passed `total_size` should be a value of 4KB * 2^x to get the most out of the `Stack::grow()`
    /// function. The starting usable size is 1 page (4KB).
    fn new(total_size: usize) -> Result<Self, Error> {
        unsafe {
            // Add 1 extra pages at the top of the stack.
            let total_size = total_size + page_size();
            let guard_top = Self::alloc(total_size)?;
            let bottom = guard_top.add(total_size);
            let top = Self::extend_usable(bottom, page_size())?;
            Ok(Self {
                guard_top,
                top,
                bottom,
            })
        }
    }

    fn bottom(&self) -> *mut u8 {
        self.bottom
    }

    fn top(&self) -> *mut u8 {
        self.top
    }

    fn guard_top(&self) -> *mut u8 {
        self.guard_top
    }

    fn give_to_signal(self) {
        CURRENT_STACK.with(|stack| stack.set(Some(self)))
    }

    fn take_from_signal() -> Option<Self> {
        CURRENT_STACK.with(|stack| stack.take())
    }

    /// This signal handler will return true if it handeled the signal. This plays nicely with
    /// WASMTIME's `set_signal_handler`. The conditions under which this signal handler will try
    /// to grow the stack are:
    /// * The signal was of type SIGSEGV or SIGBUS
    /// * the stack pointer points inside the stack's guarded area
    /// The signal will attempt to grow the stack, if there is not enough guarded space to be used
    /// it will return false to signalise WASMTIME to raise a trap.
    unsafe extern "C" fn signal_handler(
        signum: libc::c_int,
        siginfo: *mut libc::siginfo_t,
        _context: *mut libc::c_void,
    ) -> bool {
        // On Darwin, guard page accesses are raised as SIGBUS.
        let expected_guard_page_signal = if cfg!(target_os = "macos") {
            libc::SIGBUS
        } else {
            libc::SIGSEGV
        };
        if signum != expected_guard_page_signal {
            return false;
        }

        debug_assert!(!siginfo.is_null(), "siginfo must not be null");

        CURRENT_STACK.with(|stack| {
            let si_addr = (*siginfo).si_addr;
            let mut stack = match stack.take() {
                Some(stack) => stack,
                None => panic!("Stack's signal handler can't find a stack"),
            };
            if stack.stack_pointer_inside_guard(si_addr as *mut u8) {
                let result = stack.grow();
                if result.is_ok() {
                    stack.give_to_signal();
                    return true;
                }
            }
            stack.give_to_signal();
            return false;
        })
    }
}

impl PreAllocatedStack { 
    unsafe fn alloc(size: usize) -> Result<*mut u8, Error> {
        use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_NORESERVE, MAP_PRIVATE, PROT_NONE}; 

        let ptr = mmap(
            ptr::null_mut(),
            size,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANON | MAP_NORESERVE,
            -1,
            0,
        );
        if ptr == MAP_FAILED {
            Err(Error::last_os_error())
        } else {
            Ok(ptr as *mut u8)
        }
    }

    /// Mark the bottom part between `top` and `top_guard` writable.
    /// Notice that when a new stack is allocated, bottom and top are at the same address;
    unsafe fn extend_usable(top: *mut u8, size: usize) -> Result<*mut u8, Error> {
        use libc::{mprotect, PROT_READ, PROT_WRITE};

        if mprotect(
            top.sub(size) as *mut libc::c_void,
            size,
            PROT_READ | PROT_WRITE,
        ) == 0
        {
            Ok(top.sub(size))
        } else {
            Err(Error::last_os_error())
        }
    }

    /// Returns true if `sp` points to a guard page.
    fn stack_pointer_inside_guard(&self, sp: *mut u8) -> bool {
        self.guard_top <= sp && sp < self.top
    }

    /// Doubles the usable stack size if possible.
    fn grow(&mut self) -> Result<(), Error> {
        let usable_size = unsafe { self.bottom.sub(self.top as usize) as usize };
        let total_size = unsafe { self.bottom.sub(self.guard_top as usize) as usize };

        if 2 * usable_size > total_size {
            Err(Error::new(
                ErrorKind::Other,
                format!("Stack maximum reached: {}", total_size),
            ))
        } else {
            self.top = unsafe { PreAllocatedStack::extend_usable(self.top, usable_size)? };
            Ok(())
        }
    }
}

impl Drop for PreAllocatedStack {
    fn drop(&mut self) {
        let total_size = unsafe { self.bottom.sub(self.guard_top as usize) as usize };
        let result = unsafe { libc::munmap(self.guard_top as *mut libc::c_void, total_size) };
        debug_assert_eq!(result, 0);
    }
}