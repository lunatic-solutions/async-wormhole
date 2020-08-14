use std::io::Error;

use stackpp::*;
use stackpp::utils::set_signal_handler;

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
#[cfg(target_family = "unix")]
fn grow_1x_8kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(8 * 1024)?; // 8 KB
    stack.grow()?;
    Ok(())
}

#[test]
#[cfg(target_family = "unix")]
fn grow_2x_16kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(16 * 1024)?; // 16 KB
    stack.grow()?;
    stack.grow()?;
    Ok(())
}

#[test]
#[cfg(target_family = "unix")]
fn fail_on_4x_grow_32kb_stack() -> Result<(), Error> {
    let mut stack = PreAllocatedStack::new(32 * 1024)?; // 32 KB
    stack.grow()?;
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
#[cfg(target_family = "unix")]
fn trigger_signal_and_grow_stack_outside_first_4kb() -> Result<(), Error> {
    // For this to work on Windows our actual stack pointer would need to be at the offending mamory location, so
    // Windows could automatically move the guard page. 
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
