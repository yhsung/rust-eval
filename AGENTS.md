# Contributor rules

- Use Rust stable.
- Keep the implementation dependency-free unless an explicit approval permits a
  dependency.
- Follow test-driven development: add a failing automated test before changing
  production behavior, then make the smallest change that passes it.
- Before committing, run `cargo test`, `cargo fmt --check`, and
  `cargo clippy -- -D warnings`.
- Preserve explicit runtime handling of unexpected events.
- Preserve the no-wildcard match-arm policy in
  `examples/exhaustive_state_event.rs`; its compile-fail test demonstrates the
  compiler coverage this policy provides.
