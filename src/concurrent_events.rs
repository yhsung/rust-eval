use std::{
    sync::{Barrier, LazyLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    Cell, DispatchError, Event, EventKind, Idle, RrcState, StateKind, TransitionOutcome, dispatch,
};

const DEMO_T300: Duration = Duration::from_millis(10);

pub struct RaceStep {
    pub event: EventKind,
    pub before: StateKind,
    pub after: StateKind,
    pub result: Result<TransitionOutcome, DispatchError>,
}

fn wait_until(deadline: Instant) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        thread::sleep(remaining);
    }
}

/// Runs one demo connection and reports both events in receive order.
pub fn race_events() -> [RaceStep; 2] {
    let idle = RrcState::Idle(Idle {
        selected_cell: Cell {
            pci: 42,
            rsrp_dbm: -95,
        },
    });
    let connecting = dispatch(idle, Event::StartConnection { transaction_id: 1 })
        .expect("start connection from Idle")
        .state;

    let (events, inbox) = mpsc::channel();
    let setup_sender = events.clone();
    let start = Barrier::new(2);
    let deadline = LazyLock::new(|| Instant::now() + DEMO_T300);
    let wait_for_deadline = || {
        start.wait();
        wait_until(*deadline);
    };
    thread::scope(|scope| {
        scope.spawn(move || {
            wait_for_deadline();
            setup_sender
                .send(Event::SetupComplete { c_rnti: 0x1234 })
                .expect("queue setup completion");
        });
        scope.spawn(move || {
            wait_for_deadline();
            events.send(Event::T300Expiry).expect("queue expiry");
        });

        // Only this receiver owns the state; the producers only move events.
        let event = inbox.recv().expect("receive first event");
        let before = connecting.kind();
        let kind = event.kind();
        let first = dispatch(connecting, event).expect("either event is valid while Connecting");
        let first_step = RaceStep {
            event: kind,
            before,
            after: first.state.kind(),
            result: Ok(first.outcome),
        };
        let event = inbox.recv().expect("receive second event");
        let kind = event.kind();
        let before = first_step.after;
        let mut after = before;
        let result = dispatch(first.state, event).map(|transition| {
            after = transition.state.kind();
            transition.outcome
        });
        [
            first_step,
            RaceStep {
                event: kind,
                before,
                after,
                result,
            },
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::{DEMO_T300, Instant, wait_until};

    #[test]
    fn scheduled_event_does_not_fire_before_its_deadline() {
        let deadline = Instant::now() + DEMO_T300;
        wait_until(deadline);
        assert!(Instant::now() >= deadline);
    }
}
