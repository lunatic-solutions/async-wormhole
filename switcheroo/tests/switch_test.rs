use switcheroo::Generator;
use stackpp::*;

use stackpp::utils::set_signal_handler;

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
    blow_stack.resume(70);
}

fn rec(n: u64) -> u64 {
  let x: [u64; 64] = [1; 64];
  if n < 1 {
      x[0]
  } else {
      rec(n - 1)
  }
}
