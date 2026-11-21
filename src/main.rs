fn main() {
    for scenario in rust_modem_safety_island::run_all() {
        println!("{}", scenario.name);
        for step in scenario.steps {
            let (after, cm_after) = match &step.result {
                Ok(transition) => (transition.state.kind(), transition.state.cm_state()),
                Err(_) => (step.before, step.cm_before),
            };
            println!(
                "  {}: {:?}/CM-{:?} -> {:?}/CM-{:?}: {:?}",
                step.event, step.before, step.cm_before, after, cm_after, step.result
            );
        }
    }
}
