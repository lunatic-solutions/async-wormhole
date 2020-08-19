use switcheroo::Generator;
use stackpp::*;

#[test]
fn switch_stack() {
    let stack = EightMbStack::new().unwrap();
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
    let stack = EightMbStack::new().unwrap();
    let mut blow_stack = Generator::new(stack, |yielder, input| {
        rec(input);
        yielder.suspend(Some(0));
    });
    // This will use more than the first 4Kb commited memory on Windows
    blow_stack.resume(10_000);
}

fn rec(n: u64) -> u64 {
  let x: [u64; 64] = [1; 64];
  if n < 1 {
      x[0]
  } else {
      rec(n - 1)
  }
}
