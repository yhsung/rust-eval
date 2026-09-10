use rust_modem_safety_island::{DispatchError, EventKind, StateKind, TransitionOutcome, run_all};

#[test]
fn all_four_named_scenarios_are_available() {
    let names: Vec<_> = run_all()
        .into_iter()
        .map(|scenario| scenario.name)
        .collect();

    assert_eq!(
        names,
        [
            "establish-and-release",
            "t300-expiry",
            "radio-link-failure",
            "unexpected-event",
        ]
    );
}

#[test]
fn scenarios_retain_terminal_outcomes_and_the_unexpected_event_error() {
    let scenarios = run_all();

    assert_eq!(
        scenarios[0]
            .steps
            .last()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .outcome,
        TransitionOutcome::Released
    );
    assert_eq!(
        scenarios[1]
            .steps
            .last()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .outcome,
        TransitionOutcome::ConnectionFailed
    );
    assert_eq!(
        scenarios[2]
            .steps
            .last()
            .unwrap()
            .result
            .as_ref()
            .unwrap()
            .outcome,
        TransitionOutcome::RadioLinkFailure
    );
    assert_eq!(
        scenarios[3].steps.last().unwrap().result,
        Err(DispatchError::UnexpectedEvent {
            state: StateKind::Idle,
            event: EventKind::Release,
        })
    );
}
