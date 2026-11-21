# Rust Modem Safety Island PoC

This teaching PoC shows a small Rust "Safety Island" around a
modem-facing connection procedure. It makes state, events, timer ownership, and
unexpected events explicit so that contributors can inspect and exercise the
invariants.

It is an educational model, not production modem software. It is **not 3GPP-compliant**,
does not model radio protocol interoperability, and makes no
real-time, latency, scheduling, persistence, hardware, or safety-certification
claims.

## Architecture

The model has three NR operating states: `Idle`, `Inactive`, and `Connected`.
It also exposes four procedure phases: `Connecting` (T300), `Resuming` (T319),
`SelectingCell` (T311), and `Reestablishing` (T301). Each phase owns its timer.
`RRCSetup` stops T300 immediately; `RRCSetupComplete` is a separate step in
`Connected`. Resume and reestablishment likewise have separate completion
events. Unexpected input returns `UnexpectedEvent` without a transition.

`examples/exhaustive_state_event.rs` is independent from the runtime model. It
deliberately matches every state/event combination without a wildcard arm. The
compile-fail test injects one new event and asks Rust to compile it, proving the
compiler reports an uncovered combination.

## Scenarios

The CLI runs 13 repeatable scenarios:

- Establishment: `establish-and-release`, `setup-rejected`, `t300-expiry`.
- Inactive and resume: `suspend-and-resume`, `resume-rejected`,
  `resume-timeout`, `resume-fallback`.
- Recovery: `radio-link-failure`, `reestablish`, `cell-selection-timeout`,
  `reestablishment-timeout`, `reestablishment-fallback`.
- Error handling: `unexpected-event`.

Invalid runtime inputs remain observable
errors rather than silently falling through.

## Concurrency example

Run `cargo run --example concurrent_events` to see two producer threads send
`RRCSetup` and `T300Expiry` through a channel. Only the receiver owns the RRC
state. Both producers share a monotonic start instant. The CLI runs 10 rounds
with both events scheduled at 100 ms:

- `RRCSetup` first establishes the connection; the late expiry is rejected.
- `T300Expiry` first fails the connection; the late setup is rejected.

Counts depend on scheduling. The demo intentionally delivers both events,
even when a real stack would cancel the timer after receiving `RRCSetup`.
Run `cargo test --test concurrent_events` to verify the trace contract.

### macOS GUI

Run `cargo run --bin rrc-gui` on Rust stable. Click **執行 / Run** to start one
fresh race. Select a T300 value from the eight standard base values
(`100`, `200`, `300`, `400`, `600`, `1000`, `1500`, `2000` ms;
3GPP TS 38.331, `UE-TimersAndConstants.t300`). The selection stays fixed
across runs until changed. Each round uniformly samples an integer
`RRCSetup` delay within ±50 ms of the selected T300 and shows that delay.
This distribution is a demo input, not a 3GPP requirement. The window lists
the two received events in order, each before/after state, and why the late
event was rejected. The state diagram replays the observed route: mint marks
accepted transitions and amber marks the rejected event without a state change.
Replay pacing is for display only; it does not change the race. The runtime
model uses only the standard library; the GUI uses `eframe`.

Use **Procedure** to switch from the live T300 race to a lifecycle scenario.
**回放 / Replay** reveals the state path and dispatch outcome one step at a time.
The scenario timers are modeled as explicit expiry events, not real timers.

**Stochastic Run** generates a new trajectory from the entered unsigned
64-bit seed and the selected T300. Reusing both inputs reproduces the same
events and simulated timestamps. The run stops on return to `Idle` or after
24 events. The trace uses a simulated clock; replay speed is independent.
T301 and T319 are fixed at 100 ms, T311 at 1000 ms. Candidate responses and
cell selection are sampled uniformly from 50% to 150% of their deadline;
expiry wins a tie. After a completed connection, release, suspend, and radio
link failure are equally likely, with a uniformly sampled 20–200 ms wait.
These distributions are demo inputs, not measurements or 3GPP requirements.

## Run and verify

Use Rust stable:

```sh
cargo run
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

`cargo run` prints each scenario and each transition result. The test suite
includes the compiler-coverage demonstration as well as runtime transition and
CLI checks.

## Non-goals

This PoC is not a complete modem stack, a standards implementation, a timing
model, a hardware abstraction, or evidence of functional-safety compliance.
It omits access barring and T302, security and bearer procedures, and actual
cell-selection logic. The recovery scenarios assume eligibility for
reestablishment; the suitable cell is supplied as an input.
