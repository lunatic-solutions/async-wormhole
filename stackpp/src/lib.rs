pub mod pre_allocated_stack;
pub mod utils;

use std::io::Error;
pub use pre_allocated_stack::PreAllocatedStack;

pub trait Stack: Sized {
    /// Returns a new stack.
    fn new(total_size: usize) -> Result<Self, Error>;

    /// Returns a pointer to the bottom of the stack.
    fn bottom(&self) -> *mut u8;

    /// Returns a pointer to the top of the stack.
    fn top(&self) -> *mut u8;

    /// Returns a pointer to the guard_top of the stack.
    fn guard_top(&self) -> *mut u8;

    /// Consumes the stack and make it available inside the signal handler.
    /// Should be called after we got a pointer to the bottom.
    #[cfg(target_family = "unix")]
    fn give_to_signal(self);

    /// Get the stack back from the signal handler.
    #[cfg(target_family = "unix")]
    fn take_from_signal() -> Option<Self>;

    /// Handle signals to check for stack oveflows and extend the stack if necessary.
    #[cfg(target_family = "unix")]
    unsafe extern "C" fn signal_handler(
        signum: libc::c_int,
        siginfo: *mut libc::siginfo_t,
        _context: *mut libc::c_void,
    ) -> bool;
    /// On Windows the OS does actually the work for us and this function always should return false.
    #[cfg(target_family = "windows")]
    unsafe extern "system" fn signal_handler(
        exception_info: winapi::um::winnt::PEXCEPTION_POINTERS
    ) -> bool;
}
