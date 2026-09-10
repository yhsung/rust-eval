use rust_modem_safety_island::{
    dispatch, Cell, Connected, Event, Idle, RlfCause, RrcState, TransitionOutcome,
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
        Event::StartConnection { transaction_id: 7 },
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
fn setup_complete_consumes_connecting_t300_before_connected() {
    let cell = Cell {
        pci: 123,
        rsrp_dbm: -85,
    };
    let connecting = dispatch(
        RrcState::Idle(Idle {
            selected_cell: cell.clone(),
        }),
        Event::StartConnection { transaction_id: 7 },
    )
    .unwrap()
    .state;

    let result = dispatch(connecting, Event::SetupComplete { c_rnti: 0x1234 }).unwrap();

    assert_eq!(result.outcome, TransitionOutcome::ConnectionEstablished);
    assert_eq!(
        result.state,
        RrcState::Connected(Connected {
            serving_cell: cell,
            c_rnti: 0x1234,
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
        Event::StartConnection { transaction_id: 8 },
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
        }),
        Event::Release,
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
fn radio_link_failure_returns_connected_serving_cell_to_idle() {
    let cell = Cell {
        pci: 126,
        rsrp_dbm: -88,
    };
    let result = dispatch(
        RrcState::Connected(Connected {
            serving_cell: cell.clone(),
            c_rnti: 0x1234,
        }),
        Event::RadioLinkFailure {
            cause: RlfCause::OutOfSync,
            serving_cell: cell.clone(),
        },
    )
    .unwrap();

    assert_eq!(result.outcome, TransitionOutcome::RadioLinkFailure);
    assert_eq!(
        result.state,
        RrcState::Idle(Idle {
            selected_cell: cell,
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
    let error = dispatch(state, Event::Release).unwrap_err();

    assert_eq!(
        format!("{error:?}"),
        "UnexpectedEvent { state: Idle, event: Release }"
    );
}
