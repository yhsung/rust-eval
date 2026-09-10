# Rust Modem Safety Island PoC Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a dependency-free Rust CLI that demonstrates compiler-enforced RRC state-machine and ownership invariants.

**Architecture:** Keep protocol domain types in `model`, transition decisions in
`machine`, and repeatable demonstrations in `scenarios`. The binary renders
scenario records without owning protocol logic. A separate compile-fail test
proves that an exhaustive State x Event matcher catches newly added events.

**Tech Stack:** Rust stable edition 2024; Cargo built-in test, fmt, and clippy; Bash for the compile-fail test; no external crates.

**Spec:** `docs/superpowers/specs/2026-09-10-rust-modem-safety-island-design.md`

## Global Constraints

- Do not add third-party dependencies.
- This is a teaching PoC, not a 3GPP-conformant modem, real-time system, or `no_std` port.
- Runtime transition logic must return an explicit unexpected-event error; it must not silently wildcard-ignore an unsupported event.
- `Connecting` exclusively owns T300; `Connected` must not contain it.
- The compile-fail fixture must deliberately avoid a wildcard match arm.
- All production behavior begins with a failing automated test.

---

### Task 1: Create the domain model and package skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/model.rs`
- Create: `tests/model.rs`

**Interfaces:**
- Produces: `Cell { pub pci: u16, pub rsrp_dbm: i16 }`, `Timer { pub id: TimerId, pub is_running: bool }`, `TimerId::T300`, `Idle { pub selected_cell: Cell }`, `Connecting { pub selected_cell: Cell, pub t300: Timer, pub transaction_id: u8 }`, `Connected { pub serving_cell: Cell, pub c_rnti: u16 }`, `RrcState`, `Event`, `EventKind`, and `RlfCause`.
- Consumed by: `machine::dispatch` in Task 2 and scenario helpers in Task 3.

- [ ] **Step 1: Write the failing model test**

```rust
use rust_modem_safety_island::{Cell, Event, EventKind, RlfCause, Timer, TimerId};

#[test]
fn typed_events_keep_their_payload_and_kind() {
    let event = Event::RadioLinkFailure {
        cause: RlfCause::OutOfSync,
        serving_cell: Cell { pci: 123, rsrp_dbm: -85 },
    };

    assert_eq!(event.kind(), EventKind::RadioLinkFailure);
    assert_eq!(Timer::start(TimerId::T300).is_running, true);
    assert_eq!(format!("{event:?}"), "RadioLinkFailure { cause: OutOfSync, serving_cell: Cell { pci: 123, rsrp_dbm: -85 } }");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test model`

Expected: FAIL because the Cargo package and public model types do not exist.

- [ ] **Step 3: Create the minimum public domain model**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell { pub pci: u16, pub rsrp_dbm: i16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerId { T300 }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timer { pub id: TimerId, pub is_running: bool }

impl Timer {
    pub fn start(id: TimerId) -> Self { Self { id, is_running: true } }
    pub fn stop(mut self) -> Self { self.is_running = false; self }
}
```

Define the three state payload structs and `RrcState::{Idle, Connecting,
Connected}`. Define `Event::{StartConnection { transaction_id }, SetupComplete
{ c_rnti }, T300Expiry, Release, RadioLinkFailure { cause, serving_cell }}` and
an `EventKind` enum. Implement `Event::kind(&self) -> EventKind` by exhaustive
matching. Re-export these types from `src/lib.rs`.

- [ ] **Step 4: Run the focused test and full test suite**

Run: `cargo test --test model && cargo test`

Expected: PASS; the typed event preserves its payload and identifies its kind.

- [ ] **Step 5: Commit the completed domain model**

```bash
git add Cargo.toml src/lib.rs src/model.rs tests/model.rs
git commit -m "feat: add typed rrc domain model"
```

### Task 2: Implement explicit protocol transitions

**Files:**
- Create: `src/machine.rs`
- Modify: `src/lib.rs`
- Create: `tests/machine.rs`

**Interfaces:**
- Consumes: all Task 1 model types.
- Produces: `pub fn dispatch(state: RrcState, event: Event) -> Result<Transition, DispatchError>`, `Transition { pub state: RrcState, pub outcome: TransitionOutcome }`, `TransitionOutcome`, and `DispatchError::UnexpectedEvent { state: StateKind, event: EventKind }`.
- Consumed by: Task 3 scenarios and its binary.

- [ ] **Step 1: Write failing transition tests**

```rust
use rust_modem_safety_island::{dispatch, Cell, Event, Idle, RrcState, TransitionOutcome};

#[test]
fn setup_complete_consumes_connecting_t300_before_connected() {
    let cell = Cell { pci: 123, rsrp_dbm: -85 };
    let connecting = dispatch(RrcState::Idle(Idle { selected_cell: cell.clone() }), Event::StartConnection { transaction_id: 7 }).unwrap().state;
    let result = dispatch(connecting, Event::SetupComplete { c_rnti: 0x1234 }).unwrap();

    assert_eq!(result.outcome, TransitionOutcome::ConnectionEstablished);
    assert_eq!(result.state, RrcState::Connected(rust_modem_safety_island::Connected { serving_cell: cell, c_rnti: 0x1234 }));
}

#[test]
fn release_in_idle_is_rejected_with_stable_kinds() {
    let state = RrcState::Idle(Idle { selected_cell: Cell { pci: 1, rsrp_dbm: -100 } });
    let error = dispatch(state, Event::Release).unwrap_err();
    assert_eq!(format!("{error:?}"), "UnexpectedEvent { state: Idle, event: Release }");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test machine`

Expected: FAIL because `dispatch`, transition output, and dispatch errors do not exist.

- [ ] **Step 3: Implement minimal explicit transitions**

```rust
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
            state: RrcState::Connected(Connected { serving_cell: connecting.selected_cell, c_rnti }),
            outcome: TransitionOutcome::ConnectionEstablished,
        })
    }
    // T300 expiry, release, RLF, then unsupported combinations below.
}
```

Implement `Connecting + T300Expiry -> Idle`, `Connected + Release -> Idle`,
and `Connected + RadioLinkFailure -> Idle`. For all other combinations, return
`DispatchError::UnexpectedEvent` using exhaustive `StateKind` and `EventKind`
mapping. Do not mutate or replace a rejected state.

- [ ] **Step 4: Expand test coverage and verify green**

Add tests for T300 expiry, release, and RLF that assert the next `Idle` state
preserves the appropriate selected/serving cell. Run:

```bash
cargo test --test machine
cargo test
```

Expected: PASS; all five supported transitions and unsupported-event rejection are verified.

- [ ] **Step 5: Commit the transition engine**

```bash
git add src/lib.rs src/machine.rs tests/machine.rs
git commit -m "feat: add explicit rrc transitions"
```

### Task 3: Add repeatable scenarios and the demonstration CLI

**Files:**
- Create: `src/scenarios.rs`
- Create: `src/main.rs`
- Modify: `src/lib.rs`
- Create: `tests/scenarios.rs`
- Create: `tests/cli.rs`

**Interfaces:**
- Consumes: `dispatch`, `Transition`, `DispatchError`, and Task 1 model types.
- Produces: `pub fn run_all() -> Vec<ScenarioResult>` where `ScenarioResult { pub name: &'static str, pub steps: Vec<ScenarioStep> }`; `ScenarioStep` contains the event description and either a `Transition` or `DispatchError`.
- Consumed by: `main` and integration tests.

- [ ] **Step 1: Write failing scenario and CLI tests**

```rust
use rust_modem_safety_island::run_all;

#[test]
fn all_four_named_scenarios_are_available() {
    let names: Vec<_> = run_all().into_iter().map(|scenario| scenario.name).collect();
    assert_eq!(names, ["establish-and-release", "t300-expiry", "radio-link-failure", "unexpected-event"]);
}
```

```rust
#[test]
fn cli_prints_all_scenario_names() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rust-modem-safety-island"))
        .output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("establish-and-release"));
    assert!(stdout.contains("unexpected-event"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test scenarios --test cli`

Expected: FAIL because neither the scenario API nor the executable exists.

- [ ] **Step 3: Implement scenario data and a thin binary**

Implement the four sequences described in the spec with a shared initial cell.
`run_all` must retain the explicit `DispatchError` as the final result of the
unexpected-event scenario. In `main`, call `run_all()` and print each scenario
name plus its step outcomes using `Debug` formatting. Do not introduce stdout
formatting into `machine`.

- [ ] **Step 4: Verify the public demonstration**

Run:

```bash
cargo test --test scenarios --test cli
cargo run
```

Expected: all tests PASS; stdout contains all four scenario names, an RLF path,
and an unexpected-event result.

- [ ] **Step 5: Commit scenario support**

```bash
git add src/lib.rs src/scenarios.rs src/main.rs tests/scenarios.rs tests/cli.rs
git commit -m "feat: add rrc poc scenarios"
```

### Task 4: Demonstrate compile-time coverage and document contributor rules

**Files:**
- Create: `examples/exhaustive_state_event.rs`
- Create: `tests/compile_fail.rs`
- Create: `README.md`
- Create: `AGENTS.md`

**Interfaces:**
- Consumes: Rust compiler and the standalone exhaustive-match fixture.
- Produces: a tested compile-time coverage demonstration and project operating documentation.

- [ ] **Step 1: Write the failing compile-fail and documentation assertions**

```rust
#[test]
fn adding_an_event_to_the_exhaustive_fixture_is_a_compile_error() {
    let fixture = include_str!("../examples/exhaustive_state_event.rs");
    let changed = fixture.replacen("    Release,", "    Release,\n    MeasurementReport,", 1);
    // Write changed source and a minimal Cargo.toml in a unique temp directory,
    // run `cargo check`, then assert `!status.success()` and stderr contains
    // "non-exhaustive patterns".
}
```

Add a test that reads `README.md` and checks it contains `cargo run`, `cargo
test`, `Safety Island`, and `not 3GPP-compliant`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test compile_fail`

Expected: FAIL because the fixture and test harness do not exist.

- [ ] **Step 3: Implement the exhaustive fixture, verification, and docs**

The fixture defines a three-variant state enum, four-variant event enum, and
a `match (state, event)` with one arm for every combination and no wildcard.
The compile-fail test writes the modified fixture to `src/main.rs` in a
uniquely named directory below `std::env::temp_dir()`, creates a dependency-free
manifest, runs `cargo check --quiet`, checks compiler stderr, and removes only
that unique temp directory afterward.

README must cover the PoC's purpose, safety-island framing, architecture,
scenario list, run/test/lint commands, and non-goals. `AGENTS.md` must require
Rust stable, dependency-free implementation unless explicitly approved, TDD,
`cargo fmt --check`, `cargo clippy -- -D warnings`, and preservation of the
no-wildcard policy in the compile-fail fixture.

- [ ] **Step 4: Run full quality verification**

Run:

```bash
cargo test
cargo fmt --check
cargo clippy -- -D warnings
cargo run
```

Expected: all commands succeed; `cargo run` visibly renders all four scenarios.

- [ ] **Step 5: Commit the teaching and documentation layer**

```bash
git add examples/exhaustive_state_event.rs tests/compile_fail.rs README.md AGENTS.md
git commit -m "docs: explain compiler checked modem invariants"
```

### Task 5: Final repository verification and GitHub handoff

**Files:**
- Verify: all repository files created by Tasks 1–4

**Interfaces:**
- Consumes: compiled crate, tests, quality checks, and configured `origin` remote.
- Produces: verified local commit history ready to push to `yhsung/rust-eval`.

- [ ] **Step 1: Inspect the final working tree and diff**

Run:

```bash
git status --short --branch
git log --oneline --decorate -5
git diff main --check
```

Expected: clean working tree; focused commits; no whitespace errors.

- [ ] **Step 2: Re-run the complete verification gate**

Run:

```bash
cargo test
cargo fmt --check
cargo clippy -- -D warnings
cargo run
```

Expected: all checks succeed and the CLI lists four named scenarios.

- [ ] **Step 3: Push the verified history**

Run: `git push -u origin main`

Expected: `main` is created or updated at `github.com/yhsung/rust-eval`.

- [ ] **Step 4: Report delivery evidence**

State the final commit SHA, GitHub branch, each verification command and its
result, and the intentionally excluded production-modem scope.
