use crate::{Connected, Connecting, Event, EventKind, Idle, RrcState, StateKind, Timer, TimerId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub state: RrcState,
    pub outcome: TransitionOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionOutcome {
    ConnectionStarted,
    ConnectionEstablished,
    ConnectionFailed,
    Released,
    RadioLinkFailure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchError {
    UnexpectedEvent { state: StateKind, event: EventKind },
}

/// Applies an event to an RRC state.
///
/// `dispatch` takes ownership of `state`. On rejection it returns stable state
/// and event kinds, not the input state, so callers that need to retain the
/// state must clone it before dispatching.
///
/// ```
/// use rust_modem_safety_island::{dispatch, Cell, Event, Idle, RrcState};
///
/// let state = RrcState::Idle(Idle {
///     selected_cell: Cell { pci: 1, rsrp_dbm: -100 },
/// });
/// let rejected = dispatch(state.clone(), Event::Release);
///
/// assert!(rejected.is_err());
/// assert_eq!(state, RrcState::Idle(Idle {
///     selected_cell: Cell { pci: 1, rsrp_dbm: -100 },
/// }));
/// ```
pub fn dispatch(state: RrcState, event: Event) -> Result<Transition, DispatchError> {
    let state_kind = state.kind();
    let event_kind = event.kind();

    match (state, event) {
        (RrcState::Idle(idle), Event::StartConnection { transaction_id }) => Ok(Transition {
            state: RrcState::Connecting(Connecting {
                selected_cell: idle.selected_cell,
                t300: Timer::start(TimerId::T300),
                transaction_id,
            }),
            outcome: TransitionOutcome::ConnectionStarted,
        }),
        (RrcState::Connecting(connecting), Event::SetupComplete { c_rnti }) => {
            let _stopped_t300 = connecting.t300.stop();
            Ok(Transition {
                state: RrcState::Connected(Connected {
                    serving_cell: connecting.selected_cell,
                    c_rnti,
                }),
                outcome: TransitionOutcome::ConnectionEstablished,
            })
        }
        (RrcState::Connecting(connecting), Event::T300Expiry) => Ok(Transition {
            state: RrcState::Idle(Idle {
                selected_cell: connecting.selected_cell,
            }),
            outcome: TransitionOutcome::ConnectionFailed,
        }),
        (RrcState::Connected(connected), Event::Release) => Ok(Transition {
            state: RrcState::Idle(Idle {
                selected_cell: connected.serving_cell,
            }),
            outcome: TransitionOutcome::Released,
        }),
        (RrcState::Connected(connected), Event::RadioLinkFailure { .. }) => Ok(Transition {
            state: RrcState::Idle(Idle {
                selected_cell: connected.serving_cell,
            }),
            outcome: TransitionOutcome::RadioLinkFailure,
        }),
        _ => Err(DispatchError::UnexpectedEvent {
            state: state_kind,
            event: event_kind,
        }),
    }
}
