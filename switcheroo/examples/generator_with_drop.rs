use switcheroo::Generator;
use switcheroo::{stack::*, Yielder};

struct DropMarker {}

impl Drop for DropMarker {
    fn drop(&mut self) {
        println!("Dropped");
    }
}

fn main() {
    let stack = EightMbStack::new().unwrap();
    let mut add_one = Generator::new(stack, |yielder: &Yielder<i32, i32>, mut input| {
        let _marker = DropMarker {};
        input = yielder.suspend(input + 1);
        input = yielder.suspend(input + 1);
        input = yielder.suspend(input + 1);
        yielder.suspend(input + 1);
    });

    assert_eq!(add_one.resume(2), Some(3));
    assert_eq!(add_one.resume(2), Some(3));
    assert_eq!(add_one.resume(127), Some(128));
    // assert_eq!(add_one.resume(0), Some(1));
    assert_eq!(add_one.finished(), false);
}
