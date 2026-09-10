fn main() {
    for scenario in rust_modem_safety_island::run_all() {
        println!("{}", scenario.name);
        for step in scenario.steps {
            println!("  {}: {:?}", step.event, step.result);
        }
    }
}
