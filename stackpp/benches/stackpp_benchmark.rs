use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

use std::io::Error;

fn stackpp(c: &mut Criterion) {
    // All this tests both allocation and drop.
    c.bench_function("allocate 4 KB stack", |b| {
        b.iter(|| PreAllocatedStack::new(4 * 1024))
    });

    c.bench_function("allocate 1 MB stack", |b| {
        b.iter(|| PreAllocatedStack::new(1 * 1024 * 1024))
    });

    c.bench_function("allocate 8 MB stack", |b| {
        b.iter(|| PreAllocatedStack::new(1 * 1024 * 1024))
    });

    c.bench_function("allocate 32 MB stack", |b| {
        b.iter(|| PreAllocatedStack::new(1 * 1024 * 1024))
    });

    c.bench_function("grow 8 KB stack 1x", |b| {
        b.iter_batched(
            || PreAllocatedStack::new(8 * 1024).unwrap(),
            |mut stack| stack.grow(),
            BatchSize::SmallInput,
        )
    });

    c.bench_function("grow 8 MB stack 11x", |b| {
        b.iter_batched(
            || PreAllocatedStack::new(8 * 1024 * 1024).unwrap(),
            |mut stack| {
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
                stack.grow().unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("grow 8 KB stack 1x with signal", |b| {
        unsafe {
            set_signal_handler(PreAllocatedStack::signal_handler);
        }
        b.iter_batched(
            || PreAllocatedStack::new(8 * 1024).unwrap(),
            |stack| {
                let bottom = stack.bottom();
                stack.give_to_signal(); // Around ~17ns to put & take out on my i7-4850HQ (Macbook Pro)
                unsafe { *(bottom.sub(4 * 1024 + 1)) = 1 }
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("grow 8 MB stack 11x with signals", |b| {
        unsafe {
            set_signal_handler(PreAllocatedStack::signal_handler);
        }
        b.iter_batched(
            || PreAllocatedStack::new(8 * 1024 * 1024).unwrap(),
            |stack| {
                let bottom = stack.bottom();
                stack.give_to_signal();
                unsafe {
                    *(bottom.sub(4 * 1024 + 1)) = 1;
                    *(bottom.sub(8 * 1024 + 1)) = 1;
                    *(bottom.sub(16 * 1024 + 1)) = 1;
                    *(bottom.sub(32 * 1024 + 1)) = 1;
                    *(bottom.sub(64 * 1024 + 1)) = 1;
                    *(bottom.sub(128 * 1024 + 1)) = 1;
                    *(bottom.sub(256 * 1024 + 1)) = 1;
                    *(bottom.sub(512 * 1024 + 1)) = 1;
                    *(bottom.sub(1024 * 1024 + 1)) = 1;
                    *(bottom.sub(2 * 1024 * 1024 + 1)) = 1;
                    *(bottom.sub(8 * 1024 * 1024)) = 1; // Last accessible address
                }
            },
            BatchSize::SmallInput,
        )
    });

    c.bench_function("reference recursive 1 MB stack fill", |b| {
        b.iter(|| rec(black_box(940)))
    });
}

criterion_group!(benches, stackpp);
criterion_main!(benches);

/// This function is used to have a reference benchmark on how long it takes to eat up 8Mb of stack.
/// 8 MB is only an assumption here, it's really hard to tell how much the compiler is actually going
/// to use here.
fn rec(n: u64) -> u64 {
    let x: [u64; 64] = black_box([1; 64]);
    if n < 1 {
        x[0]
    } else {
        rec(black_box(n - 1))
    }
}

#[cfg(target_family = "unix")]
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
        use winapi::um::minwinbase::*;
        let record = &*(*exception_info).ExceptionRecord;
        // If it's not an access violation let the next handler take care of it.
        if record.ExceptionCode != EXCEPTION_ACCESS_VIOLATION
        {
            return winapi::vc::excpt::EXCEPTION_CONTINUE_SEARCH;
        }

        let f = F.unwrap();
        f(exception_info);
        winapi::vc::excpt::EXCEPTION_CONTINUE_EXECUTION
    }

    if winapi::um::errhandlingapi::AddVectoredExceptionHandler(1, Some(helper_handler)).is_null() {
        panic!("failed to add exception handler: {}", Error::last_os_error());
    }
}