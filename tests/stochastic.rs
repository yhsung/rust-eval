use rust_modem_safety_island::{StateKind, stochastic_run};

#[test]
fn seed_replays_the_same_timed_path() {
    let run = stochastic_run(42, 100);
    assert_eq!(run, stochastic_run(42, 100));
    assert_eq!(run.steps_ms[0], 0);
    assert!(run.steps_ms.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(run.steps_ms.len(), run.scenario.steps.len());
    assert_ne!(run, stochastic_run(43, 100));
}

#[test]
fn trajectories_use_fixed_timer_deadlines_and_reach_multiple_procedures() {
    let timers = [
        ("T300Expiry", 200),
        ("T301Expiry", 100),
        ("T311Expiry", 1000),
        ("T319Expiry", 100),
    ];
    let mut observed = [false; 4];
    let mut visited = [false; 3];
    for seed in 0..256 {
        let run = stochastic_run(seed, 200);
        assert!((2..=24).contains(&run.scenario.steps.len()));
        let mut state = StateKind::Idle;
        for (i, step) in run.scenario.steps.iter().enumerate() {
            assert_eq!(step.before, state);
            state = step.result.as_ref().unwrap().state.kind();
            for (j, (event, ms)) in timers.iter().enumerate() {
                if step.event == *event {
                    assert_eq!(run.steps_ms[i] - run.steps_ms[i - 1], *ms);
                    observed[j] = true;
                }
            }
            visited[0] |= state == StateKind::Inactive;
            visited[1] |= state == StateKind::Resuming;
            visited[2] |= state == StateKind::Reestablishing;
        }
        assert!(state == StateKind::Idle || run.scenario.steps.len() == 24);
    }
    assert_eq!(observed, [true; 4]);
    assert_eq!(visited, [true; 3]);
}
