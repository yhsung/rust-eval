use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn adding_an_event_to_the_exhaustive_fixture_is_a_compile_error() {
    let fixture = include_str!("../examples/exhaustive_state_event.rs");
    let changed = fixture.replacen("    Release,", "    Release,\n    MeasurementReport,", 1);
    let temporary_project = unique_temporary_project();
    {
        fs::create_dir_all(temporary_project.path.join("src")).unwrap();
        fs::write(
            temporary_project.path.join("Cargo.toml"),
            "[package]\nname = \"exhaustive-state-event-check\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(temporary_project.path.join("src/main.rs"), changed).unwrap();

        let output = Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(&temporary_project.path)
            .output()
            .unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();

        assert!(
            !output.status.success(),
            "adding an event must make the exhaustive match fail to compile"
        );
        assert!(
            stderr.contains("non-exhaustive patterns"),
            "expected Rust's exhaustive-match diagnostic, got:\n{stderr}"
        );
    }
}

#[test]
fn readme_documents_the_runnable_safety_island_workflow() {
    let readme = fs::read_to_string("README.md").unwrap();

    assert!(readme.contains("cargo run"));
    assert!(readme.contains("cargo test"));
    assert!(readme.contains("Safety Island"));
    assert!(readme.contains("not 3GPP-compliant"));

    let output = Command::new(env!("CARGO_BIN_EXE_rust-modem-safety-island"))
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    for scenario in [
        "establish-and-release",
        "t300-expiry",
        "radio-link-failure",
        "unexpected-event",
    ] {
        assert!(
            readme.contains(scenario),
            "README must document the {scenario} scenario"
        );
        assert!(
            stdout.contains(scenario),
            "the documented {scenario} scenario must run"
        );
    }
}

struct TemporaryProject {
    path: PathBuf,
}

impl Drop for TemporaryProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn unique_temporary_project() -> TemporaryProject {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);

    TemporaryProject {
        path: env::temp_dir().join(format!(
            "rust-modem-safety-island-compile-fail-{}-{nanos}-{sequence}",
            std::process::id()
        )),
    }
}
