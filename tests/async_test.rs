use async_wormhole::AsyncWormhole;

#[test]
fn async_yield() {
    let task = AsyncWormhole::new(|mut yielder| {
        println!("The println function blows up the stack more than 4Kb.");
        let x = yielder.async_suspend(async { 5 });
        assert_eq!(x, 5);
        let y = yielder.async_suspend(async { true });
        assert_eq!(y, true);
        42
    })
    .unwrap();
    let output = futures::executor::block_on(task);
    assert_eq!(output, 42);
}
