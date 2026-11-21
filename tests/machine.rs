use rust_modem_safety_island::{
    Cell, Completion, Connected, Event, Idle, RlfCause, RrcState, SelectingCell, Timer, TimerId,
    TransitionOutcome, dispatch,
};

#[test]
fn start_connection_enters_connecting_with_t300() {
    let cell = Cell {
        pci: 123,
        rsrp_dbm: -85,
    };

    let result = dispatch(
        RrcState::Idle(Idle {
            selected_cell: cell.clone(),
        }),
        Event::RrcSetupRequest { transaction_id: 7 },
    )
    .unwrap();

    assert_eq!(result.outcome, TransitionOutcome::ConnectionStarted);
    assert_eq!(
        result.state,
        RrcState::Connecting(rust_modem_safety_island::Connecting {
            selected_cell: cell,
            t300: rust_modem_safety_island::Timer::start(rust_modem_safety_island::TimerId::T300),
            transaction_id: 7,
        })
    );
}

#[test]
fn rrc_setup_consumes_connecting_t300_before_connected() {
    let cell = Cell {
        pci: 123,
        rsrp_dbm: -85,
    };
    let connecting = dispatch(
        RrcState::Idle(Idle {
            selected_cell: cell.clone(),
        }),
        Event::RrcSetupRequest { transaction_id: 7 },
    )
    .unwrap()
    .state;

    let result = dispatch(connecting, Event::RrcSetup { c_rnti: 0x1234 }).unwrap();

    assert_eq!(result.outcome, TransitionOutcome::ConnectionEstablished);
    assert_eq!(
        result.state,
        RrcState::Connected(Connected {
            serving_cell: cell,
            c_rnti: 0x1234,
            pending_completion: Some(Completion::Setup),
        })
    );
}

#[test]
fn t300_expiry_returns_to_idle_with_selected_cell() {
    let cell = Cell {
        pci: 124,
        rsrp_dbm: -86,
    };
    let connecting = dispatch(
        RrcState::Idle(Idle {
            selected_cell: cell.clone(),
        }),
        Event::RrcSetupRequest { transaction_id: 8 },
    )
    .unwrap()
    .state;

    let result = dispatch(connecting, Event::T300Expiry).unwrap();

    assert_eq!(result.outcome, TransitionOutcome::ConnectionFailed);
    assert_eq!(
        result.state,
        RrcState::Idle(Idle {
            selected_cell: cell,
        })
    );
}

#[test]
fn release_returns_connected_serving_cell_to_idle() {
    let cell = Cell {
        pci: 125,
        rsrp_dbm: -87,
    };
    let result = dispatch(
        RrcState::Connected(Connected {
            serving_cell: cell.clone(),
            c_rnti: 0x1234,
            pending_completion: None,
        }),
        Event::RrcRelease { suspend: false },
    )
    .unwrap();

    assert_eq!(result.outcome, TransitionOutcome::Released);
    assert_eq!(
        result.state,
        RrcState::Idle(Idle {
            selected_cell: cell,
        })
    );
}

#[test]
fn radio_link_failure_starts_cell_selection_from_the_serving_cell() {
    let serving_cell = Cell {
        pci: 126,
        rsrp_dbm: -88,
    };
    let reported_cell = Cell {
        pci: 127,
        rsrp_dbm: -109,
    };
    let result = dispatch(
        RrcState::Connected(Connected {
            serving_cell: serving_cell.clone(),
            c_rnti: 0x1234,
            pending_completion: None,
        }),
        Event::RadioLinkFailure {
            cause: RlfCause::OutOfSync,
            serving_cell: reported_cell,
        },
    )
    .unwrap();

    assert_eq!(result.outcome, TransitionOutcome::RadioLinkFailure);
    assert_eq!(
        result.state,
        RrcState::SelectingCell(SelectingCell {
            previous_cell: serving_cell,
            t311: Timer::start(TimerId::T311),
        })
    );
}

#[test]
fn release_in_idle_is_rejected_with_stable_kinds() {
    let state = RrcState::Idle(Idle {
        selected_cell: Cell {
            pci: 1,
            rsrp_dbm: -100,
        },
    });
    let error = dispatch(state.clone(), Event::RrcRelease { suspend: false }).unwrap_err();

    assert_eq!(
        format!("{error:?}"),
        "UnexpectedEvent { state: Idle, event: RrcRelease }"
    );
    assert_eq!(
        state,
        RrcState::Idle(Idle {
            selected_cell: Cell {
                pci: 1,
                rsrp_dbm: -100,
            },
        })
    );
}
