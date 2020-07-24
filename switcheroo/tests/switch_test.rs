use switcheroo::Generator;
use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

#[test]
fn switch_stack() {
    let stack = PreAllocatedStack::new(1 * 1024 * 1024).unwrap(); // 1 MB
    let mut add_one = Generator::new(stack, |yielder, mut input| {
        loop {
          if input == 0 { break }
          input = yielder.suspend(Some(input + 1));
        }
      });
      assert_eq!(add_one.resume(2), Some(3));
      assert_eq!(add_one.resume(127), Some(128));
      assert_eq!(add_one.resume(-1), Some(0));
      assert_eq!(add_one.resume(0), None);
      assert_eq!(add_one.resume(0), None);
}

#[test]
fn extend_small_stack() {
    let stack = PreAllocatedStack::new(1 * 1024 * 1024).unwrap(); // 1 MB
    let mut blow_stack = Generator::new(stack, |yielder, input| {
        rec(input);
        yielder.suspend(Some(0));
    });
    unsafe { set_signal_handler(PreAllocatedStack::signal_handler); }
    // This will use more than the first 4Kb allocated to the stack;
    blow_stack.resume(6);
}

fn rec(n: u64) -> u64 {
  let x: [u64; 64] = [1; 64];
  if n < 1 {
      x[0]
  } else {
      rec(n - 1)
  }
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