#[cfg(target_family = "unix")]
mod unix;
#[cfg(target_family = "windows")]
mod windows;

#[cfg(target_family = "unix")]
pub use self::unix::*;

#[cfg(target_family = "windows")]
pub use self::windows::*;

use std::sync::atomic::{AtomicUsize, Ordering};

/// Returns page size in bytes
pub fn page_size() -> usize {
    #[cold]
    #[cfg(target_family = "unix")]
    pub fn sys_page_size() -> usize {
        unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
    }
    
    #[cold]
    #[cfg(target_family = "windows")]
    pub fn sys_page_size() -> usize {
        use winapi::um::sysinfoapi::{SYSTEM_INFO, LPSYSTEM_INFO};
        use winapi::um::sysinfoapi::GetSystemInfo;

        unsafe { 
            let mut info: SYSTEM_INFO = std::mem::zeroed();
            GetSystemInfo(&mut info as LPSYSTEM_INFO);
            info.dwPageSize as usize
         }
    }

    static PAGE_SIZE_CACHE: AtomicUsize = AtomicUsize::new(0);
    match PAGE_SIZE_CACHE.load(Ordering::Relaxed) {
        0 => {
            // Assure that we are using 4KB pages on all platforms.
            let page_size = sys_page_size();
            assert_eq!(page_size, 4096);

            PAGE_SIZE_CACHE.store(page_size, Ordering::Relaxed);
            page_size
        }
        page_size => page_size,
    }
}