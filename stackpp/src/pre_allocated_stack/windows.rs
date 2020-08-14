use std::io::Error;
use std::ptr;

use winapi::um::memoryapi::{VirtualAlloc, VirtualFree};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PAGE_GUARD, MEM_RELEASE};

use crate::Stack;
use super::page_size;

pub struct PreAllocatedStack {
    guard_top: *mut u8,
    top: *mut u8,
    bottom: *mut u8,
}

impl Stack for PreAllocatedStack {
    fn new(total_size: usize) -> Result<Self, Error> {
        unsafe {
            // Add 4 extra pages at the top of the stack if we use up the whole size, so there is enough
            // stack for the exception handler.
            let total_size = total_size + 4 * page_size();
            let guard_top = Self::alloc(total_size)?;
            let bottom = guard_top.add(total_size);
            let _ = Self::extend_usable(bottom, page_size())?;
            // The top is used to set the stack limit in the TIB and it is fixed to 4 pages under the
            // deallocation stack (guard_top).
            let top = guard_top.offset(4 * page_size() as isize);
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

    /// noop on Windows
    fn give_to_signal(self) {}

    /// noop on Windows
    fn take_from_signal() -> Option<Self> { None }

    /// Windows keep moving the guard page automatically and re-running the instruction, so there is nothing
    /// for us to do here:
    // https://docs.microsoft.com/en-us/cpp/build/stack-usage?view=vs-2019
    unsafe extern "system" fn signal_handler(_exception_info: winapi::um::winnt::PEXCEPTION_POINTERS) -> bool {
        false // noop on windows
    }
}

impl PreAllocatedStack {
    unsafe fn alloc(size: usize) -> Result<*mut u8, Error> {
        let ptr = VirtualAlloc(ptr::null_mut(), size, MEM_RESERVE, PAGE_GUARD | PAGE_READWRITE);
        if ptr.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(ptr as *mut u8)
        }
    }

    unsafe fn extend_usable(top: *mut u8, size: usize) -> Result<*mut u8, Error> {
        if !VirtualAlloc(
            top.sub(size) as *mut winapi::ctypes::c_void,
            size,
            MEM_COMMIT,
            PAGE_READWRITE,
        ).is_null()
        {
            // Add one guard page at top of the *usable* stack.
            if !VirtualAlloc(
                top.sub(size + page_size()) as *mut winapi::ctypes::c_void,
                page_size(),
                MEM_COMMIT,
                PAGE_GUARD | PAGE_READWRITE,
            ).is_null()
            {
                Ok(top.sub(size))
            } else {
                Err(Error::last_os_error())
            }
        } else {
            Err(Error::last_os_error())
        }
    }
}

impl Drop for PreAllocatedStack {
    fn drop(&mut self) {
        let result = unsafe { VirtualFree(self.guard_top as *mut winapi::ctypes::c_void, 0, MEM_RELEASE) };
        debug_assert_ne!(result, 0);
    }
}


