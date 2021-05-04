// All architectures expose a similar api. Here I just want to take some time explaining the general
// idea behind all of them.
//
// At the core of the implementation there are 3 functions:
// * `init(stack: Stack, f: unsafe extern "C" fn(usize, *mut  usize))`
// * `swap_and_link_stacks(arg: usize, new_sp: *mut usize, sp: *mut usize) -> (usize, *mut usize)`
// * `swap(arg: usize, new_sp: *mut usize, sp: *mut usize) -> (usize, *mut usize)`
//
// ### init
// `init` takes a **stack** and a **pointer to a function**. It will prepare the stack so it is ready
// to be switched to. Once we switch to it the function we set up here will be called.
//
// Unix and Windows operating systems require different stack setups. Here is an illustration on how
// the stacks look after the call to `init`:
// ```
//      +                  +
//      |     .......      |
//      |                  |
//      |Deallocation stack|
//      +------------------+
//      |Stack limit       |
//      +------------------+
//      |Stack base        |        +                  +
//      +------------------+        |                  |
// +----+Stack frame ptr   |        |                  |
// |    +------------------+        |    .........     |
// |    |Trampoline        |        |                  |
// |    +------------------+   +----+Stack frame ptr   |
// +---->Caller frame      |   |    +------------------+
//      +------------------+   |    |Trampoline 2 ptr  |
//      |Function ptr      |   |    +------------------+
//      +------------------+   +---->Caller frame      |
//                                 +------------------+
//                                  |Trampoline 1 ptr  |
//                                  +------------------+
//                                  |Function ptr      |
//                                  +------------------+
//                                  |Alignment         |
//                                  +------------------+
//
//            Windows                      Unix
// ```
// Windows needs to preserve some extra information across context switches, like the stack base, top
// and deallocation values. If they are not present Windows will not know how to grow the stack.
// The [Boost.Context](https://www.boost.org/doc/libs/1_61_0/libs/context/doc/html/context/overview.html)
// library also preserves some other information, like the current
// [Fiber](https://docs.microsoft.com/en-us/windows/win32/procthread/fibers) data, but I don't expect
// anyone to use switcheroo and Windows Fibers in the same app.
//
// The **Caller frame** value will be filled in by the `swap_and_link_stacks` function to link the 2
// stacks from different contexts. At this point of time we can't know from where we are jumping to
// the stack.
//
// ### swap_and_link_stacks
// This function is really similar to `swap`, but it's expected to be the first one called when jumping
// to a new stack. It will write the **Caller frame** data inside the new stack, basically linking them
// together. Once this data exists on the new stack we don't need to call it anymore and can switch
// stacks with just the `swap` function.
//
// The swap functions will:
// 1. Preserve the frame pointer and instruction pointer of the current context.
//    On Windows, deallocation stack, stack limit and base stack are also preserved.
// 2. Change the stack pointer to the new stack.
// 3. Pop the frame pointer and instruction pointer from the new stack.
// 4. Jump to the instruction.
//
// Notice that the instruction pointer points to a cryptic **Trampoline 2** function and not to the
// passed in **Function**. Trampoline 1 and 2 contain some extra assembler information so that it's
// possible to re-create a backtrace across contexts if we panic inside the new context.

#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
mod unix_x64;
#[cfg(all(target_family = "unix", target_arch = "x86_64"))]
pub use self::unix_x64::*;

#[cfg(all(target_family = "unix", target_arch = "aarch64"))]
mod unix_aarch64;
#[cfg(all(target_family = "unix", target_arch = "aarch64"))]
pub use self::unix_aarch64::*;

#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
mod windows_x64;
#[cfg(all(target_family = "windows", target_arch = "x86_64"))]
pub use self::windows_x64::*;
