use std::io::Error;

use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

#[test]
fn crate_1mb_stack() -> Result<(), Error> {
    PreAllocatedStack::new(1 * 1024 * 1024)?; // 1 MB
    Ok(())
}

#[test]
fn crate_100tb_of_stacks() -> Result<(), Error> {
    let mut stacks = vec![];
    for _i in 0..100 {
        let stack = PreAllocatedStack::new(1024 * 1024 * 1024 * 1024); // 1 TB
        assert!(stack.is_ok());
        stacks.push(stack);
    }
    
    Ok(())
}

#[test]
fn grow_1x_8kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(8 * 1024)?; // 8 KB
    stack.grow()?;
    Ok(())
}

#[test]
fn grow_2x_16kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(16 * 1024)?; // 16 KB
    stack.grow()?;
    stack.grow()?;
    Ok(())
}

#[test]
fn fail_on_3x_grow_16kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(16 * 1024)?; // 16 KB
    stack.grow()?;
    stack.grow()?;
    let fail = stack.grow().is_err();
    assert_eq!(fail, true);
    Ok(())
}

#[test]
fn allow_access_inside_first_4kb() -> Result<(), Error> {
    let stack = PreAllocatedStack::new(4 * 1024)?; // 4 KB
    let bottom = stack.bottom();
    unsafe {
        *(bottom.sub(4 * 1024)) = 64;
        assert_eq!(*(bottom.sub(4 * 1024)), 64);
    }
    Ok(())
}

#[test]
fn trigger_signal_and_grow_stack_outside_first_4kb() -> Result<(), Error> {
    let stack = PreAllocatedStack::new(8 * 1024)?; // 8 KB
    let bottom = stack.bottom();
    stack.give_to_signal();
    unsafe {
        set_signal_handler(PreAllocatedStack::signal_handler);
        *(bottom.sub(4 * 1024 + 1)) = 64;
        assert_eq!(*(bottom.sub(4 * 1024 + 1)), 64);
    }
    PreAllocatedStack::take_from_signal(); // Take the stack from the thread local variable so it can get dropped.
    Ok(())
}

#[cfg(target_family = "unix")]
unsafe fn set_signal_handler(
    f: unsafe extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) -> bool,
) {
    let register = |signal: i32| {
        let mut handler: libc::sigaction = std::mem::zeroed();
        // SA_SIGINFO gives us access to information like the program
        // counter from where the fault happened.
        //
        // SA_ONSTACK allows us to handle signals on an alternate stack,
        // so that the handler can run in response to running out of
        // stack space on the main stack. Rust installs an alternate
        // stack with sigaltstack, so we rely on that.
        handler.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
        handler.sa_sigaction = f as usize;
        libc::sigemptyset(&mut handler.sa_mask);
        if libc::sigaction(signal, &handler, std::ptr::null_mut()) != 0 {
            panic!(
                "unable to install signal handler: {}",
                Error::last_os_error(),
            );
        }
    };

    // On Darwin, guard page accesses are raised as SIGBUS.
    if cfg!(target_os = "macos") {
        register(libc::SIGBUS);
    } else {
        register(libc::SIGSEGV);
    }
}

#[cfg(target_family = "windows")]
unsafe fn set_signal_handler(
    f: unsafe extern "system" fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool,
) {
    // WASMTIME expects the signal handler to return true/false, but the windows API expects an i32 value.
    // We use here a wrapper function. It's a bit hard to wrap around a fn that is not a closure and we are
    // forced to apply a little static variable trick here. Notice that this code would not work if we passed
    // 2 different `f` arguments in two different calls, both handlers would reference the last one. But for
    // our testing purposes this is ok, as we will always use `PreAllocatedStack::signal_handler` as `f`.
    static mut F: Option<unsafe extern "system" fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool> = None;
    F = Some(f);
    unsafe extern "system" fn helper_handler(exception_info: winapi::um::winnt::PEXCEPTION_POINTERS) -> winapi::um::winnt::LONG {
        let f = F.unwrap();

        // If it's not a guard page violation or the stack pointer is not inside a guard page, let the next
        // handler take care of it.
        if !f(exception_info) {
            winapi::vc::excpt::EXCEPTION_CONTINUE_SEARCH
        } else {
            winapi::vc::excpt::EXCEPTION_CONTINUE_EXECUTION
        }
    }

    if winapi::um::errhandlingapi::AddVectoredExceptionHandler(1, Some(helper_handler)).is_null() {
        panic!("failed to add exception handler: {}", Error::last_os_error());
    }
}