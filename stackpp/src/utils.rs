use std::io::Error;

#[cfg(target_family = "unix")]
pub unsafe fn set_signal_handler(
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
pub unsafe fn set_signal_handler(
    _f: unsafe extern "system" fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool,
) {
    // According to: https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/resetstkoflw?view=vs-2019
    // Windows will automatically move the guard page if there is enough space on the stack and re-run the instruction,
    // until the Stack limit (specified in the Thread Information Block GS:[0x10]) is reached and then it will rais an
    // exception. This is exactly the behaviour we want and manually do on unix systems.

    // WASMTIME expects the signal handler to return true/false, but the windows API expects an i32 value.
    // We use here a wrapper function. It's a bit hard to wrap around a fn that is not a closure and we are
    // forced to apply a little static variable trick here. Notice that this code would not work if we passed
    // 2 different `f` arguments in two different calls, both handlers would reference the last one. But for
    // our testing purposes this is ok, as we will always use `PreAllocatedStack::signal_handler` as `f`.

    // static mut F: Option<unsafe extern "system" fn(winapi::um::winnt::PEXCEPTION_POINTERS) -> bool> = None;
    // F = Some(f);
    // unsafe extern "system" fn helper_handler(exception_info: winapi::um::winnt::PEXCEPTION_POINTERS) -> winapi::um::winnt::LONG {
    //     let f = F.unwrap();

    //     // If it's not a guard page violation or the stack pointer is not inside a guard page, let the next
    //     // handler take care of it.
    //     if !f(exception_info) {
    //         winapi::vc::excpt::EXCEPTION_CONTINUE_SEARCH
    //     } else {
    //         winapi::vc::excpt::EXCEPTION_CONTINUE_EXECUTION
    //     }
    // }

    // if winapi::um::errhandlingapi::AddVectoredExceptionHandler(1, Some(helper_handler)).is_null() {
    //     panic!("failed to add exception handler: {}", Error::last_os_error());
    // }
}