use std::io::Error;
use std::ptr;
use std::mem::size_of;

use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_NORESERVE, MAP_PRIVATE, PROT_READ, PROT_WRITE}; 

use super::Stack;

/// Stack pointer to a 8 Mb pre-allocated stack.
pub struct EightMbStack(*mut usize);

const EIGHT_MB: usize = 0x8_000_000;

impl Stack for EightMbStack {
    fn new() -> Result<Self, Error> {
        let ptr = unsafe {
            mmap(
                ptr::null_mut(),
                EIGHT_MB,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANON | MAP_NORESERVE,
                -1,
                0,
            )
        };
        if ptr == MAP_FAILED {
            Err(Error::last_os_error())
        } else {
            Ok(Self(ptr as *mut usize))
        }
    }

    fn bottom(&self) -> *mut usize {
        // The `add(size)` function for type T adds `size * size_of(T)` bytes to the pointer.
        unsafe { self.0.add(EIGHT_MB / size_of::<usize>()) }
    }
    fn top(&self) -> *mut usize {
        self.0
    }
    fn deallocation(&self) -> *mut usize {
        self.top()
    }
}

impl Drop for EightMbStack {
    fn drop(&mut self) {
        let result = unsafe { libc::munmap(self.top() as *mut libc::c_void, EIGHT_MB) };
        debug_assert_eq!(result, 0);
    }
}