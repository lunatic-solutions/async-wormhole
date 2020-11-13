use std::io::Error;

use switcheroo::stack::*;

#[test]
fn create_8_mb_stack() -> Result<(), Error> {
    EightMbStack::new()?;
    Ok(())
}

#[test]
fn create_300k_8_mb_stacks() {
    // Uses around 4 Gb of commited memory
    let mut stacks = vec![];
    for _i in 0..300_000 {
        let stack = EightMbStack::new();
        assert!(stack.is_ok());
        stacks.push(stack);
    }
}
