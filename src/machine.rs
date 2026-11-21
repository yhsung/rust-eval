use crate::{
    Cell, Completion, Connected, Connecting, Event, EventKind, Idle, Reestablishing, Resuming,
    RrcState, SelectingCell, StateKind, Timer, TimerId,
};

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
    SetupCompleted,
    ConnectionRejected,
    Suspended,
    ResumeStarted,
    Resumed,
    ResumeCompleted,
    ResumeRejected,
    ResumeFailed,
    ReestablishmentStarted,
    Reestablished,
    ReestablishmentCompleted,
    ReestablishmentFailed,
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
/// let rejected = dispatch(state.clone(), Event::RrcRelease { suspend: false });
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
        (RrcState::Idle(idle), Event::RrcSetupRequest { transaction_id }) => Ok(Transition {
            state: RrcState::Connecting(Connecting {
                selected_cell: idle.selected_cell,
                t300: Timer::start(TimerId::T300),
                transaction_id,
            }),
            outcome: TransitionOutcome::ConnectionStarted,
        }),
        (RrcState::Connecting(connecting), Event::RrcSetup { c_rnti }) => {
            connecting.t300.stop();
            Ok(setup(connecting.selected_cell, c_rnti))
        }
        (RrcState::Connecting(connecting), Event::RrcReject) => {
            connecting.t300.stop();
            Ok(to_idle(
                connecting.selected_cell,
                TransitionOutcome::ConnectionRejected,
            ))
        }
        (RrcState::Connecting(connecting), Event::T300Expiry) => Ok(to_idle(
            connecting.selected_cell,
            TransitionOutcome::ConnectionFailed,
        )),
        (RrcState::Connected(mut link), Event::RrcSetupComplete)
            if link.pending_completion == Some(Completion::Setup) =>
        {
            link.pending_completion = None;
            Ok(Transition {
                state: RrcState::Connected(link),
                outcome: TransitionOutcome::SetupCompleted,
            })
        }
        (RrcState::Connected(mut link), Event::RrcResumeComplete)
            if link.pending_completion == Some(Completion::Resume) =>
        {
            link.pending_completion = None;
            Ok(Transition {
                state: RrcState::Connected(link),
                outcome: TransitionOutcome::ResumeCompleted,
            })
        }
        (RrcState::Connected(mut link), Event::RrcReestablishmentComplete)
            if link.pending_completion == Some(Completion::Reestablishment) =>
        {
            link.pending_completion = None;
            Ok(Transition {
                state: RrcState::Connected(link),
                outcome: TransitionOutcome::ReestablishmentCompleted,
            })
        }
        (RrcState::Connected(mut link), Event::RrcRelease { suspend: true }) => {
            link.pending_completion = None;
            Ok(Transition {
                state: RrcState::Inactive(link),
                outcome: TransitionOutcome::Suspended,
            })
        }
        (
            RrcState::Connected(link) | RrcState::Inactive(link),
            Event::RrcRelease { suspend: false },
        ) => Ok(to_idle(link.serving_cell, TransitionOutcome::Released)),
        (RrcState::Inactive(context), Event::RrcResumeRequest) => Ok(Transition {
            state: RrcState::Resuming(Resuming {
                context,
                t319: Timer::start(TimerId::T319),
            }),
            outcome: TransitionOutcome::ResumeStarted,
        }),
        (RrcState::Resuming(resuming), Event::RrcResume) => {
            resuming.t319.stop();
            let mut link = resuming.context;
            link.pending_completion = Some(Completion::Resume);
            Ok(Transition {
                state: RrcState::Connected(link),
                outcome: TransitionOutcome::Resumed,
            })
        }
        (RrcState::Resuming(resuming), Event::RrcReject) => {
            resuming.t319.stop();
            Ok(Transition {
                state: RrcState::Inactive(resuming.context),
                outcome: TransitionOutcome::ResumeRejected,
            })
        }
        (RrcState::Resuming(resuming), Event::T319Expiry) => Ok(to_idle(
            resuming.context.serving_cell,
            TransitionOutcome::ResumeFailed,
        )),
        (RrcState::Resuming(resuming), Event::RrcSetup { c_rnti }) => {
            resuming.t319.stop();
            Ok(setup(resuming.context.serving_cell, c_rnti))
        }
        (RrcState::Connected(link), Event::RadioLinkFailure { .. }) => Ok(Transition {
            state: RrcState::SelectingCell(SelectingCell {
                previous_cell: link.serving_cell,
                t311: Timer::start(TimerId::T311),
            }),
            outcome: TransitionOutcome::RadioLinkFailure,
        }),
        (RrcState::SelectingCell(selecting), Event::T311Expiry) => Ok(to_idle(
            selecting.previous_cell,
            TransitionOutcome::ReestablishmentFailed,
        )),
        (RrcState::SelectingCell(selecting), Event::RrcReestablishmentRequest { cell }) => {
            selecting.t311.stop();
            Ok(Transition {
                state: RrcState::Reestablishing(Reestablishing {
                    selected_cell: cell,
                    t301: Timer::start(TimerId::T301),
                }),
                outcome: TransitionOutcome::ReestablishmentStarted,
            })
        }
        (RrcState::Reestablishing(procedure), Event::T301Expiry) => Ok(to_idle(
            procedure.selected_cell,
            TransitionOutcome::ReestablishmentFailed,
        )),
        (RrcState::Reestablishing(procedure), Event::RrcReestablishment { c_rnti }) => {
            procedure.t301.stop();
            Ok(Transition {
                state: RrcState::Connected(Connected {
                    serving_cell: procedure.selected_cell,
                    c_rnti,
                    pending_completion: Some(Completion::Reestablishment),
                }),
                outcome: TransitionOutcome::Reestablished,
            })
        }
        (RrcState::Reestablishing(procedure), Event::RrcSetup { c_rnti }) => {
            procedure.t301.stop();
            Ok(setup(procedure.selected_cell, c_rnti))
        }
        _ => Err(DispatchError::UnexpectedEvent {
            state: state_kind,
            event: event_kind,
        }),
    }
}

fn setup(serving_cell: Cell, c_rnti: u16) -> Transition {
    Transition {
        state: RrcState::Connected(Connected {
            serving_cell,
            c_rnti,
            pending_completion: Some(Completion::Setup),
        }),
        outcome: TransitionOutcome::ConnectionEstablished,
    }
}

fn to_idle(selected_cell: Cell, outcome: TransitionOutcome) -> Transition {
    Transition {
        state: RrcState::Idle(Idle { selected_cell }),
        outcome,
    }
}
