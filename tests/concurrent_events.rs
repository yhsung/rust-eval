use rust_modem_safety_island::{
    DispatchError, EventKind, StateKind, TransitionOutcome, race_events,
};

#[test]
fn trace_reports_the_received_order_and_preserves_state_on_rejection() {
    let [first, second] = race_events();
    assert_eq!(first.before, StateKind::Connecting);
    let (after, outcome, late) = match first.event {
        EventKind::SetupComplete => (
            StateKind::Connected,
            TransitionOutcome::ConnectionEstablished,
            EventKind::T300Expiry,
        ),
        EventKind::T300Expiry => (
            StateKind::Idle,
            TransitionOutcome::ConnectionFailed,
            EventKind::SetupComplete,
        ),
        other => panic!("unexpected first event: {other:?}"),
    };
    assert_eq!((first.after, first.result), (after, Ok(outcome)));
    assert_eq!(
        (second.event, second.before, second.after),
        (late, after, after)
    );
    assert_eq!(
        second.result,
        Err(DispatchError::UnexpectedEvent {
            state: after,
            event: late
        })
    );
}
