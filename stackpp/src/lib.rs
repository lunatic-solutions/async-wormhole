pub mod pre_allocated_stack;

use std::io::Error;

pub trait Stack: Sized {
    /// The passed `size` should be a value of 4KB * 2^x to get the most out of the `Stack::grow()` function.
    /// The starting usable size is 1 page (4KB).
    fn new(total_size: usize) -> Result<Self, Error>;

    /// Returns true if the `sp` is inside the guarded stack area.
    fn stack_pointer_inside_guard(&self, sp: *mut u8) -> bool;

    /// Doubles the usable stack size if possible.
    fn grow(&mut self) -> Result<(), Error>;

    /// Returns a pointer to the bottom of the stack.
    fn bottom(&self) -> *mut u8;

    /// Consumes the stack and make it available inside the signal handler.
    /// Should be called after we got a pointer to the bottom.
    fn give_to_signal(self);

    /// Get the stack from the signal handler
    fn take_from_signal() -> Option<Self>;

    /// Handle signals raised by guard page access.
    unsafe extern "C" fn signal_handler(
        signum: libc::c_int,
        siginfo: *mut libc::siginfo_t,
        _context: *mut libc::c_void,
    ) -> bool;
}
