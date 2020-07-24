use async_wormhole::AsyncWormhole;
use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

#[test]
fn async_yield() {
    unsafe { set_signal_handler(PreAllocatedStack::signal_handler); }
    let task = AsyncWormhole::new(|mut yielder| {
        let x = yielder.async_suspend(async { 5 } );
        assert_eq!(x, 5);
        let y = yielder.async_suspend(async { true } );
        assert_eq!(y, true);
        42
    }).unwrap();
    let output = futures::executor::block_on(task);
    assert_eq!(output, 42);
}

unsafe fn set_signal_handler(
    f: unsafe extern "C" fn(libc::c_int, *mut libc::siginfo_t, *mut libc::c_void) -> bool,
) {
    let register = |signal: i32| {
        let mut handler: libc::sigaction = std::mem::zeroed();
        // The flags here are relatively careful, and they are...
        //
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
                std::io::Error::last_os_error(),
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