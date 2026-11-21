#[test]
fn cli_prints_all_scenario_names() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-modem-safety-island"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(stdout.contains("establish-and-release"));
    assert!(stdout.contains("t300-expiry"));
    assert!(stdout.contains("radio-link-failure"));
    assert!(stdout.contains("unexpected-event"));
    assert!(stdout.contains("suspend-and-resume"));
    assert!(stdout.contains("reestablishment-fallback"));
    assert!(stdout.contains("RRCReestablishmentComplete"));
    assert!(stdout.contains("RadioLinkFailure"));
    assert!(stdout.contains("UnexpectedEvent"));
}
