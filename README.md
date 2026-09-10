# Rust Modem Safety Island PoC

This dependency-free teaching PoC shows a small Rust "Safety Island" around a
modem-facing connection procedure. It makes state, events, timer ownership, and
unexpected events explicit so that contributors can inspect and exercise the
invariants.

It is an educational model, not production modem software. It is **not 3GPP-compliant**,
does not model radio protocol interoperability, and makes no
real-time, latency, scheduling, persistence, hardware, or safety-certification
claims.

## Architecture

The model has three runtime states: `Idle`, `Connecting`, and `Connected`.
Typed events enter a single dispatcher, which returns either a typed transition
or an explicit `UnexpectedEvent` error. Only `Connecting` owns T300; setup
completion consumes that timer before entering `Connected`, while T300 expiry
returns to `Idle`.

`examples/exhaustive_state_event.rs` is independent from the runtime model. It
deliberately matches every state/event combination without a wildcard arm. The
compile-fail test injects one new event and asks Rust to compile it, proving the
compiler reports an uncovered combination.

## Scenarios

The CLI runs four repeatable scenarios:

- `establish-and-release`
- `t300-expiry`
- `radio-link-failure`
- `unexpected-event`

The last scenario is intentional: invalid runtime inputs remain observable
errors rather than silently falling through.

## Run and verify

Use Rust stable with no extra dependencies:

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
model, a hardware abstraction, or evidence of functional-safety compliance. It
exists to teach a narrow design technique: use types, explicit failures, and a
no-wildcard exhaustive fixture to keep safety-relevant changes visible.
