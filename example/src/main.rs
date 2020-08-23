use async_wormhole::AsyncWormhole;
fn main() {
    let task = AsyncWormhole::new(|mut yielder| {
        let x = yielder.async_suspend(async { 5 });
        assert_eq!(x, 5);
        panic!("Will a longer panic also fail. What about a really long one.");
        let y = yielder.async_suspend(async { true });
        assert_eq!(y, true);
        42
    })
    .unwrap();

    let output = futures::executor::block_on(task);
    assert_eq!(output.unwrap(), 42);
}
