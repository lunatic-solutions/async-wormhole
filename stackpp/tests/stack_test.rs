use std::io::Error;

use stackpp::*;

#[test]
fn create_8mb_stack() -> Result<(), Error> {
    EightMbStack::new()?; // 1 MB
    Ok(())
}

#[test]
fn create_120tb_of_stacks() {
    let mut stacks = vec![];
    for _i in 0..(15_000_000) {
        let stack = EightMbStack::new();
        assert!(stack.is_ok());
        stacks.push(stack);
    }
}