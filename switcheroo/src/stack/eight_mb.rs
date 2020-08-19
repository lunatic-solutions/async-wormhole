use std::io::Error;
use std::mem::size_of;
use std::ptr;

#[cfg(target_family = "unix")]
use libc::{mmap, MAP_ANON, MAP_FAILED, MAP_NORESERVE, MAP_PRIVATE, PROT_READ, PROT_WRITE};

#[cfg(target_family = "windows")]
use winapi::ctypes::c_void;
#[cfg(target_family = "windows")]
use winapi::um::memoryapi::{VirtualAlloc, VirtualFree, VirtualProtect};
#[cfg(target_family = "windows")]
use winapi::um::winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_GUARD, PAGE_READWRITE, PAGE_NOACCESS};

use super::Stack;

/// Stack pointer to a 8 Mb pre-allocated stack.
pub struct EightMbStack(*mut usize);

const EIGHT_MB: usize = 8 * 1024 * 1024;
#[cfg(target_family = "windows")]
const EXCEPTION_ZONE: usize = 4 * 4096;

impl Stack for EightMbStack {
    #[cfg(target_family = "unix")]
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

    #[cfg(target_family = "unix")]
    fn bottom(&self) -> *mut usize {
        // The `add(size)` function for type T adds `size * size_of(T)` bytes to the pointer.
        unsafe { self.0.add(EIGHT_MB / size_of::<usize>()) }
    }
    #[cfg(target_family = "unix")]
    fn top(&self) -> *mut usize {
        self.0
    }
    #[cfg(target_family = "unix")]
    fn deallocation(&self) -> *mut usize {
        panic!("Not used on unix");
    }

    // Windows
    #[cfg(target_family = "windows")]
    fn new() -> Result<Self, Error> {
        unsafe {
            // Add extra 16 Kb on top of the stack to be used by the exception handler in case of a stack overflow.
            // Cast pointer to `usize`, because calculating offsets with `c_void` is impossible. Sometimes it has a
            // size of 0, sometimes it decides to be 1 byte.
            let ptr = VirtualAlloc(
                ptr::null_mut(),
                EIGHT_MB + EXCEPTION_ZONE,
                MEM_RESERVE,
                PAGE_NOACCESS,
            ) as *mut usize;
            if ptr.is_null() {
                return Err(Error::last_os_error());
            }
            // Commit 3 bottom pages (1 read/write and 2 guard pages)
            let bottom_2 = VirtualAlloc(
                ptr.add((EIGHT_MB + EXCEPTION_ZONE - 3 * 4096) / size_of::<usize>()) as *mut c_void,
                3 * 4096,
                MEM_COMMIT,
                PAGE_GUARD | PAGE_READWRITE,
            );
            if bottom_2.is_null() {
                return Err(Error::last_os_error());
            }

            let old_protect: u32 = 0;
            let bottom_1 = VirtualProtect(
                ptr.add((EIGHT_MB + EXCEPTION_ZONE - 1 * 4096) / size_of::<usize>())  as *mut c_void,
                1 * 4096,
                PAGE_READWRITE,
                &old_protect as *const u32 as *mut u32,
            );
            if bottom_1 == 0 {
                return Err(Error::last_os_error());
            }

            Ok(Self(ptr as *mut usize))
        }
    }

    #[cfg(target_family = "windows")]
    fn bottom(&self) -> *mut usize {
        unsafe { self.0.add((EIGHT_MB + EXCEPTION_ZONE) / size_of::<usize>()) }
    }
    #[cfg(target_family = "windows")]
    fn top(&self) -> *mut usize {
        unsafe { self.0.add(EXCEPTION_ZONE / size_of::<usize>()) }
    }
    #[cfg(target_family = "windows")]
    fn deallocation(&self) -> *mut usize {
        self.0
    }
}

#[cfg(target_family = "unix")]
impl Drop for EightMbStack {
    fn drop(&mut self) {
        let result = unsafe { libc::munmap(self.top() as *mut libc::c_void, EIGHT_MB) };
        debug_assert_eq!(result, 0);
    }
}

#[cfg(target_family = "windows")]
impl Drop for EightMbStack {
    fn drop(&mut self) {
        let result = unsafe { VirtualFree(self.0 as *mut winapi::ctypes::c_void, 0, MEM_RELEASE) };
        debug_assert_ne!(result, 0);
    }
}
