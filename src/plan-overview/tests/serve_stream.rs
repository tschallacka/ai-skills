// MODE: DEV
use plan_overview::serve::state_stream;
use std::time::Duration;

#[test]
fn subscribers_receive_current_state_and_each_update() {
    let stream = state_stream("first".into());
    let first = stream.subscribe();
    let second = stream.subscribe();
    assert_eq!(
        first.recv_timeout(Duration::from_millis(50)).unwrap(),
        "first"
    );
    assert_eq!(
        second.recv_timeout(Duration::from_millis(50)).unwrap(),
        "first"
    );
    stream.publish("second".into());
    assert_eq!(
        first.recv_timeout(Duration::from_millis(50)).unwrap(),
        "second"
    );
    assert_eq!(
        second.recv_timeout(Duration::from_millis(50)).unwrap(),
        "second"
    );
    assert_eq!(stream.current(), "second");
}
