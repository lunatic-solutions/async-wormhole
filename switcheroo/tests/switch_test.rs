use switcheroo::stack::*;
use switcheroo::Generator;

#[test]
fn switch_stack() {
    let stack = EightMbStack::new().unwrap();
    let mut add_one = Generator::new(stack, |yielder, mut input| {
        println!("Sometimes println doesn't touch all pages on windows");
        loop {
            if input == 0 {
                break;
            }
            input = yielder.suspend(input + 1);
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
    let stack = EightMbStack::new().unwrap();
    let mut blow_stack = Generator::new(stack, |yielder, input| {
        rec(input);
        yielder.suspend(Some(0));
    });
    // This will use 7 Mb of stack, more than the first 4 Kb commited memory on Windows
    blow_stack.resume(7_000);
}

// Uses 1 Kb per iteration
fn rec(n: u64) -> u8 {
    let x: [u8; 1024] = [1; 1024];
    if n < 1 {
        x[0]
    } else {
        rec(n - 1)
    }
}

#[test]
#[should_panic]
fn panic_on_different_stack() {
    let stack = EightMbStack::new().unwrap();
    let mut add_one = Generator::new(stack, |_yielder, mut _input| {
        panic!("Ups");
    });
    let _: u32 = add_one.resume(0).unwrap();
}
