use rust_modem_safety_island::{
    Cell, CmState, Event, Idle, RlfCause, RrcState, StateKind, dispatch,
};

fn cell(pci: u16) -> Cell {
    Cell { pci, rsrp_dbm: -90 }
}

fn idle() -> RrcState {
    RrcState::Idle(Idle {
        selected_cell: cell(1),
    })
}

fn step(state: RrcState, event: Event) -> RrcState {
    dispatch(state, event).unwrap().state
}

#[test]
fn rrc_maps_to_simplified_cm_state() {
    assert_eq!(idle().cm_state(), CmState::Idle);

    let connecting = step(idle(), Event::RrcSetupRequest { transaction_id: 1 });
    assert_eq!(connecting.kind(), StateKind::Connecting);
    assert_eq!(connecting.cm_state(), CmState::Idle);

    let established = step(connecting, Event::RrcSetup { c_rnti: 7 });
    assert_eq!(established.cm_state(), CmState::Connected);

    let completed = step(established, Event::RrcSetupComplete);
    assert_eq!(completed.cm_state(), CmState::Connected);
}

#[test]
fn suspend_and_radio_failure_retain_cm_connected_until_idle() {
    let connected = step(
        step(idle(), Event::RrcSetupRequest { transaction_id: 1 }),
        Event::RrcSetup { c_rnti: 7 },
    );
    let connected = step(connected, Event::RrcSetupComplete);

    let inactive = step(connected.clone(), Event::RrcRelease { suspend: true });
    assert_eq!(inactive.kind(), StateKind::Inactive);
    assert_eq!(inactive.cm_state(), CmState::Connected);

    let resuming = step(inactive, Event::RrcResumeRequest);
    assert_eq!(resuming.cm_state(), CmState::Connected);
    let back_to_idle = step(resuming, Event::T319Expiry);
    assert_eq!(back_to_idle.kind(), StateKind::Idle);
    assert_eq!(back_to_idle.cm_state(), CmState::Idle);

    let selecting = step(
        connected,
        Event::RadioLinkFailure {
            cause: RlfCause::OutOfSync,
            serving_cell: cell(1),
        },
    );
    assert_eq!(selecting.kind(), StateKind::SelectingCell);
    assert_eq!(selecting.cm_state(), CmState::Connected);
    let released = step(selecting, Event::T311Expiry);
    assert_eq!(released.cm_state(), CmState::Idle);
}

#[test]
fn scenario_steps_carry_the_cm_state_before_each_event() {
    let scenarios = rust_modem_safety_island::run_all();
    let establish = scenarios
        .iter()
        .find(|scenario| scenario.name == "establish-and-release")
        .expect("establish-and-release scenario exists");

    assert_eq!(establish.steps[0].cm_before, CmState::Idle);
    assert_eq!(establish.steps[1].cm_before, CmState::Idle);
    assert_eq!(establish.steps[2].cm_before, CmState::Connected);
}
