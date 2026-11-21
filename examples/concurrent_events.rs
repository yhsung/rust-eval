use std::{
    sync::{Barrier, LazyLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use rust_modem_safety_island::{
    Cell, DispatchError, Event, Idle, RrcState, TransitionOutcome, dispatch,
};

const DEMO_T300: Duration = Duration::from_millis(10);

fn wait_until(deadline: Instant) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        thread::sleep(remaining);
    }
}

fn race_events() -> (TransitionOutcome, Result<TransitionOutcome, DispatchError>) {
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
    let start = &start;
    let deadline = LazyLock::new(|| Instant::now() + DEMO_T300);
    // Start the demo clock once both producers are ready.
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

        // Only the receiver owns the state; the producers only move events.
        let first = dispatch(connecting, inbox.recv().expect("receive first event"))
            .expect("either event is valid while Connecting");
        let second = dispatch(first.state, inbox.recv().expect("receive second event"))
            .map(|transition| transition.outcome);
        (first.outcome, second)
    })
}

fn main() {
    const ROUNDS: usize = 100;
    let mut setup_wins = 0;
    let mut expiry_wins = 0;
    for _ in 0..ROUNDS {
        let (winner, loser) = race_events();
        let wins = match winner {
            TransitionOutcome::ConnectionEstablished => &mut setup_wins,
            TransitionOutcome::ConnectionFailed => &mut expiry_wins,
            other => panic!("unexpected outcome: {other:?}"),
        };
        if *wins == 0 {
            println!("{winner:?}; second event: {loser:?}");
        }
        *wins += 1;
    }
    println!("{ROUNDS} rounds: SetupComplete first {setup_wins}, T300Expiry first {expiry_wins}");
}

#[cfg(test)]
mod tests {
    use super::race_events;
    use rust_modem_safety_island::{DispatchError, EventKind, StateKind, TransitionOutcome};

    #[test]
    fn scheduled_event_does_not_fire_before_its_deadline() {
        use super::wait_until;
        use std::time::{Duration, Instant};

        let deadline = Instant::now() + Duration::from_millis(10);
        wait_until(deadline);
        assert!(Instant::now() >= deadline);
    }

    #[test]
    fn either_winner_transitions_and_the_loser_is_rejected() {
        for _ in 0..16 {
            let (winner, loser) = race_events();
            let (state, event) = match winner {
                TransitionOutcome::ConnectionEstablished => {
                    (StateKind::Connected, EventKind::T300Expiry)
                }
                TransitionOutcome::ConnectionFailed => (StateKind::Idle, EventKind::SetupComplete),
                other => panic!("unexpected outcome: {other:?}"),
            };
            assert_eq!(loser, Err(DispatchError::UnexpectedEvent { state, event }));
        }
    }
}
