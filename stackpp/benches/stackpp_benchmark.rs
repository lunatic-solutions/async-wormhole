use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

use stackpp::pre_allocated_stack::PreAllocatedStack;
use stackpp::Stack;

use std::io::Error;

fn stackpp(c: &mut Criterion) {
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
                stack.give_to_signal(); // Around 650ns on my i7-4850HQ (Macbook Pro)
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
                stack.give_to_signal(); // Around 650ns on my i7-4850HQ (Macbook Pro)
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

    c.bench_function("reference recursive 8 MB stack fill", |b| {
        b.iter(|| rec(black_box(7_910)))
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
