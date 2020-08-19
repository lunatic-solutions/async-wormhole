mod eight_mb;
pub use eight_mb::EightMbStack;

pub trait Stack: Sized {
    /// Returns a new stack.
    fn new() -> Result<Self, std::io::Error>;

    /// Returns a pointer to the bottom of the stack.
    fn bottom(&self) -> *mut usize;

    /// Returns a pointer to the top of the stack.
    fn top(&self) -> *mut usize;

    /// Returns a pointer to the deallocation stack (a Windows construct).
    fn deallocation(&self) -> *mut usize;
}
