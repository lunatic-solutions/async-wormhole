use std::io::Error;

use stackpp::*;

#[test]
fn create_8_mb_stack() -> Result<(), Error> {
    EightMbStack::new()?;
    Ok(())
}

#[test]
fn create_6_tb_of_stacks() {
    let mut stacks = vec![];
    for _i in 0..(800_000) {
        let stack = EightMbStack::new();
        assert!(stack.is_ok());
        stacks.push(stack.unwrap());
    }
}