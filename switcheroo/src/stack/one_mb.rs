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
use winapi::um::winnt::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_GUARD, PAGE_NOACCESS, PAGE_READWRITE,
};

use super::Stack;

/// A 1 Mb Stack (1 Mb + 4 Kb).
///
/// On Unix platforms this will simply reserve 1 Mb + 4 Kb of memory to be used as a stack (without
/// a guard page). Mmap will be called with the MAP_NORESERVE flag to allow us to overcommit on stack
/// allocations.
///
/// On Windows it will reserve 1 Mb + 4Kb of memory + 4 pages on top for the exception handler. Only the
/// bottom of the stack will be marked as commited, while the rest will be reserved. This allows us
/// to overcommit on stack allocations. The memory is specifically set up with guard pages in a way
/// that Windows expect it to be, so that the OS can automatically grow and commit memory.
pub struct OneMbStack(*mut usize);

unsafe impl Send for OneMbStack {}

const ONE_MB: usize = 1 * 1024 * 1024 + 4096;
#[cfg(target_family = "windows")]
const EXCEPTION_ZONE: usize = 4 * 4096;

impl Stack for OneMbStack {
    #[cfg(target_family = "unix")]
    fn new() -> Result<Self, Error> {
        let ptr = unsafe {
            mmap(
                ptr::null_mut(),
                ONE_MB,
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
        unsafe { self.0.add(ONE_MB / size_of::<usize>()) }
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
                ONE_MB + EXCEPTION_ZONE,
                MEM_RESERVE,
                PAGE_NOACCESS,
            ) as *mut usize;
            if ptr.is_null() {
                return Err(Error::last_os_error());
            }
            // Commit 3 bottom pages (1 read/write and 2 guard pages)
            let bottom_2 = VirtualAlloc(
                ptr.add((ONE_MB + EXCEPTION_ZONE - 3 * 4096) / size_of::<usize>()) as *mut c_void,
                3 * 4096,
                MEM_COMMIT,
                PAGE_GUARD | PAGE_READWRITE,
            );
            if bottom_2.is_null() {
                return Err(Error::last_os_error());
            }

            let old_protect: u32 = 0;
            let bottom_1 = VirtualProtect(
                ptr.add((ONE_MB + EXCEPTION_ZONE - 1 * 4096) / size_of::<usize>()) as *mut c_void,
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
        unsafe { self.0.add((ONE_MB + EXCEPTION_ZONE) / size_of::<usize>()) }
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
impl Drop for OneMbStack {
    fn drop(&mut self) {
        let result = unsafe { libc::munmap(self.0 as *mut libc::c_void, ONE_MB) };
        debug_assert_eq!(result, 0);
    }
}

#[cfg(target_family = "windows")]
impl Drop for OneMbStack {
    fn drop(&mut self) {
        let result = unsafe { VirtualFree(self.0 as *mut winapi::ctypes::c_void, 0, MEM_RELEASE) };
        debug_assert_ne!(result, 0);
    }
}
