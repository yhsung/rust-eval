# Rust Modem Safety Island PoC — Design

## Purpose

Demonstrate that a small Rust protocol component can turn selected modem RRC
procedure invariants into compiler-checked types and explicit transition
coverage. This is a teaching and evaluation artifact, not a 3GPP-compliant
modem implementation.

## Scope

The executable models a simplified RRC connection procedure with these states:

- `Idle`, which owns a selected cell.
- `Connecting`, which owns a selected cell, a running T300 timer, and a
  transaction identifier.
- `Connected`, which owns a serving cell and a C-RNTI.

It accepts typed events for connection start, setup completion, T300 expiry,
release, and radio-link failure. Each event may include the data required to
process it, rather than an untyped ID plus payload pointer.

The repository will also include a state-transition coverage exercise: a
deliberately exhaustive matcher in a separate compile-fail example. Adding an
event to that example makes Rust identify every unhandled state/event pair.

## Non-goals

- Full 3GPP RRC, NAS, MAC, RLC, PDCP, ASN.1, or OTA decoding.
- `no_std`, an RTOS port, hardware drivers, DSP/PHY kernels, ISR code, or
  latency claims.
- C FFI and production IPC transports.
- Claiming that compilation proves conformance to a modem specification.

## Architecture

```text
Scenario input
    |
    v
typed Event ------------------- malformed/unexpected event --> audit result
    |
    v
RrcMachine::dispatch(RrcState, Event)
    |                         |
    | ownership move          | explicit transition decision
    v                         v
next RrcState            Transition record
```

`model` contains domain types (`Cell`, `Timer`, `Idle`, `Connecting`,
`Connected`, `RrcState`, and `Event`). `machine` owns the pure transition
function and returns a transition record including the next state and a
domain-level outcome. `scenarios` drives realistic event sequences for the CLI
and tests. The binary only formats scenario output; protocol decisions remain
testable without stdout capture.

`Connecting` is the only state containing `T300`; successful setup consumes
that state before constructing `Connected`. Therefore a connected state with a
live T300 cannot be constructed through the public state transition API.

## Error and Unexpected-event Policy

The runtime PoC explicitly rejects unsupported state/event combinations with
`DispatchError::UnexpectedEvent`, retaining the current state in the caller.
It does not silently ignore a protocol input. Each such result includes stable
state and event kind names suitable for logging or an eventual IPC audit path.

The compile-fail example takes the stricter approach: it intentionally contains
no wildcard arm, so adding a variant requires an explicit decision for each
state/event combination.

## Demonstrations and Tests

The CLI will run four named scenarios:

1. `establish-and-release`: `Idle -> Connecting -> Connected -> Idle` and
   validates T300 completion semantics.
2. `t300-expiry`: returns from `Connecting` to `Idle`.
3. `radio-link-failure`: returns from `Connected` to `Idle` while preserving
   the serving cell as the newly selected cell.
4. `unexpected-event`: shows that `Release` in `Idle` is rejected rather than
   mutating state.

Unit tests assert each transition's state data and outcome. An integration test
runs the binary and checks the scenario labels. A shell-based compile-fail
verification copies the exhaustive example into a temporary Cargo crate, adds
a new event variant, and asserts `cargo check` fails with a non-exhaustive
match diagnostic. No third-party crate is required.

## Repository and Delivery

The initial repository contains `Cargo.toml`, source and test files, a concise
README, the compile-fail example, and `AGENTS.md`. `AGENTS.md` directs future
contributors to preserve the PoC's safety-island scope, test-first workflow,
and explicit coverage policy. The `origin` remote is
`https://github.com/yhsung/rust-eval.git`; changes are committed locally and
pushed only as a final delivery step after verification.

## Acceptance Criteria

- `cargo run` displays all four scenarios and their outcomes.
- `cargo test` validates successful, timeout, release, RLF, and unexpected
  event behavior.
- `cargo test --test compile_fail` proves that extending an exhaustive event
  enum without covering every state/event pair fails compilation.
- `cargo clippy -- -D warnings` and `cargo fmt --check` succeed.
- README accurately describes scope and commands, without production or 3GPP
  conformance claims.
