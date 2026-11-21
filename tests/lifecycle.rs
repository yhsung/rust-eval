use rust_modem_safety_island::{
    Cell, DispatchError, Event, EventKind, Idle, RlfCause, RrcState, StateKind, TimerId,
    Transition, TransitionOutcome, dispatch,
};

fn cell(pci: u16) -> Cell {
    Cell { pci, rsrp_dbm: -90 }
}

fn step(state: RrcState, event: Event) -> Transition {
    dispatch(state, event).unwrap()
}

fn connecting() -> RrcState {
    step(
        RrcState::Idle(Idle {
            selected_cell: cell(1),
        }),
        Event::RrcSetupRequest { transaction_id: 1 },
    )
    .state
}

fn connected() -> RrcState {
    let setup = step(connecting(), Event::RrcSetup { c_rnti: 7 });
    step(setup.state, Event::RrcSetupComplete).state
}

#[test]
fn setup_stops_t300_and_completion_is_one_shot() {
    let setup = step(connecting(), Event::RrcSetup { c_rnti: 7 });
    assert_eq!(setup.state.kind(), StateKind::Connected);
    assert_eq!(
        dispatch(setup.state.clone(), Event::T300Expiry),
        Err(DispatchError::UnexpectedEvent {
            state: StateKind::Connected,
            event: EventKind::T300Expiry,
        })
    );
    let complete = step(setup.state, Event::RrcSetupComplete);
    assert_eq!(complete.outcome, TransitionOutcome::SetupCompleted);
    assert_eq!(
        dispatch(complete.state, Event::RrcSetupComplete),
        Err(DispatchError::UnexpectedEvent {
            state: StateKind::Connected,
            event: EventKind::RrcSetupComplete,
        })
    );
    let rejected = step(connecting(), Event::RrcReject);
    assert_eq!(
        (rejected.state.kind(), rejected.outcome),
        (StateKind::Idle, TransitionOutcome::ConnectionRejected)
    );
}

#[test]
fn suspend_resume_restores_context_and_timeout_discards_it() {
    let inactive = step(connected(), Event::RrcRelease { suspend: true }).state;
    assert_eq!(inactive.kind(), StateKind::Inactive);
    let resuming = step(inactive, Event::RrcResumeRequest).state;
    let RrcState::Resuming(procedure) = &resuming else {
        panic!("expected resume")
    };
    assert_eq!(procedure.t319.id, TimerId::T319);
    let abandoned = step(resuming.clone(), Event::T319Expiry);
    assert_eq!(abandoned.state.kind(), StateKind::Idle);
    let resumed = step(resuming, Event::RrcResume);
    let completed = step(resumed.state, Event::RrcResumeComplete);
    let RrcState::Connected(link) = completed.state else {
        panic!("expected connection")
    };
    assert_eq!((link.serving_cell, link.c_rnti), (cell(1), 7));
}

#[test]
fn reestablishment_uses_the_selected_cell_and_rejects_stale_t311() {
    let selecting = step(
        connected(),
        Event::RadioLinkFailure {
            cause: RlfCause::OutOfSync,
            serving_cell: cell(1),
        },
    )
    .state;
    let RrcState::SelectingCell(procedure) = &selecting else {
        panic!("expected cell selection")
    };
    assert_eq!(procedure.t311.id, TimerId::T311);
    assert_eq!(
        step(selecting.clone(), Event::T311Expiry).state.kind(),
        StateKind::Idle
    );
    let requesting = step(
        selecting,
        Event::RrcReestablishmentRequest { cell: cell(2) },
    )
    .state;
    let RrcState::Reestablishing(procedure) = &requesting else {
        panic!("expected reestablishment")
    };
    assert_eq!(procedure.t301.id, TimerId::T301);
    assert!(matches!(
        dispatch(requesting.clone(), Event::T311Expiry),
        Err(DispatchError::UnexpectedEvent { .. })
    ));
    let expired = step(requesting.clone(), Event::T301Expiry);
    assert_eq!(
        expired.state,
        RrcState::Idle(Idle {
            selected_cell: cell(2)
        })
    );
    let restored = step(requesting, Event::RrcReestablishment { c_rnti: 8 });
    let completed = step(restored.state, Event::RrcReestablishmentComplete);
    let released = step(completed.state, Event::RrcRelease { suspend: false });
    assert_eq!(
        released.state,
        RrcState::Idle(Idle {
            selected_cell: cell(2)
        })
    );
}
