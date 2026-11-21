use rust_modem_safety_island::{DispatchError, EventKind, StateKind, TransitionOutcome, run_all};

#[test]
fn named_scenarios_are_available() {
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
            "setup-rejected",
            "suspend-and-resume",
            "resume-rejected",
            "resume-timeout",
            "resume-fallback",
            "reestablish",
            "cell-selection-timeout",
            "reestablishment-timeout",
            "reestablishment-fallback",
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
            event: EventKind::RrcRelease,
        })
    );
}

#[test]
fn lifecycle_scenarios_reach_their_expected_terminal_states() {
    let cases = [
        (
            "setup-rejected",
            StateKind::Idle,
            TransitionOutcome::ConnectionRejected,
        ),
        (
            "suspend-and-resume",
            StateKind::Idle,
            TransitionOutcome::Released,
        ),
        (
            "resume-rejected",
            StateKind::Inactive,
            TransitionOutcome::ResumeRejected,
        ),
        (
            "resume-timeout",
            StateKind::Idle,
            TransitionOutcome::ResumeFailed,
        ),
        (
            "resume-fallback",
            StateKind::Connected,
            TransitionOutcome::SetupCompleted,
        ),
        ("reestablish", StateKind::Idle, TransitionOutcome::Released),
        (
            "cell-selection-timeout",
            StateKind::Idle,
            TransitionOutcome::ReestablishmentFailed,
        ),
        (
            "reestablishment-timeout",
            StateKind::Idle,
            TransitionOutcome::ReestablishmentFailed,
        ),
        (
            "reestablishment-fallback",
            StateKind::Connected,
            TransitionOutcome::SetupCompleted,
        ),
    ];
    let scenarios = run_all();
    for (name, state, outcome) in cases {
        let scenario = scenarios
            .iter()
            .find(|scenario| scenario.name == name)
            .unwrap();
        for step in &scenario.steps {
            assert!(step.result.is_ok(), "{name}: {}", step.event);
        }
        let terminal = scenario.steps.last().unwrap().result.as_ref().unwrap();
        assert_eq!(
            (terminal.state.kind(), terminal.outcome),
            (state, outcome),
            "{name}"
        );
    }
}
