use rust_modem_safety_island::{TransitionOutcome, race_events};

fn main() {
    const ROUNDS: usize = 100;
    let mut setup_wins = 0;
    let mut expiry_wins = 0;
    for _ in 0..ROUNDS {
        let [first, second] = race_events();
        let winner = first.result.expect("first event transitions");
        let wins = match winner {
            TransitionOutcome::ConnectionEstablished => &mut setup_wins,
            TransitionOutcome::ConnectionFailed => &mut expiry_wins,
            other => panic!("unexpected outcome: {other:?}"),
        };
        if *wins == 0 {
            println!("{winner:?}; second event: {:?}", second.result);
        }
        *wins += 1;
    }
    println!("{ROUNDS} rounds: SetupComplete first {setup_wins}, T300Expiry first {expiry_wins}");
}
