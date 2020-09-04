# Switcheroo

> This library is heavily inspired by https://github.com/edef1c/libfringe.

> **Currently only works in Rust nightly.**

Switcheroo provides lightweight context switches in Rust. It runs on Windows, MacOs and Linux (x64 & AArch64).

## Example

```rust
use switcheroo::stack::*;
use switcheroo::Generator;

fn  main() {
    let stack = EightMbStack::new().unwrap();
    let  mut add_one = Generator::new(stack, |yielder, mut input| {
        loop {
            if input ==  0 {
                break;
            }
            input = yielder.suspend(input +  1);
        }
    });

    assert_eq!(add_one.resume(2), Some(3));
    assert_eq!(add_one.resume(127), Some(128));
    assert_eq!(add_one.resume(0), None);
    assert_eq!(add_one.resume(0), None);
}
```

## Performance

On my Macbook Pro 15" (Late 2013) each context switch is comparable to a function call (sub-nanosecond).

## Developer Experience

Switcheroo **tries** hard to not let the context switching disturb default Rust behaviour on panics. The displayed backtrace should stretch across the context switch boundary.

> There are known bugs on Windows, where this doesn't perfectly work because of how Rust handles naked functions on windows: [#75897](https://github.com/rust-lang/rust/issues/75897)

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
