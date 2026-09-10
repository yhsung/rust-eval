# Final Review Fix Report

## Scope

- Documented the ownership contract of `dispatch`: it consumes the input
  `RrcState`, and a caller that needs to retain state after rejection must
  clone it before dispatching.
- Strengthened the radio-link-failure regression test to use distinct connected
  serving and event-reported cells, proving that the next `Idle` state retains
  the connected serving cell.

## Files Changed

- `src/machine.rs`: public `dispatch` rustdoc and a compiled usage example.
- `tests/machine.rs`: caller-retention assertion and distinct-cell RLF test.

## Test-First Evidence

1. Updated `radio_link_failure_returns_connected_serving_cell_to_idle` before
   production documentation. The focused test initially passed because the
   existing implementation already retained the connected serving cell.
2. Mutated the RLF branch temporarily to use the event payload cell and ran:

   ```text
   rtk cargo test --test machine radio_link_failure_returns_connected_serving_cell_to_idle
   ```

   Result: failed as intended; actual `Idle` selected cell was PCI 127 / -109
   while the required connected serving cell was PCI 126 / -88.
3. Restored the correct branch, added the public rustdoc example, and verified
   the test and doctest below.

## Verification Commands and Results

```text
rtk cargo test --test machine
```

Result: 6 passed, 0 failed.

```text
rtk cargo test --doc
```

Result: 1 passed, 0 failed (the `dispatch` ownership example).

```text
rtk cargo test
```

Result: 13 passed across 8 suites, 0 failed.

```text
rtk cargo fmt --check
```

Result: exit 0, no formatting differences.

```text
rtk cargo clippy -- -D warnings
```

Result: no issues found.

```text
rtk git diff --check
```

Result: exit 0, no whitespace errors.

## Coverage by Finding

- Rejection ownership: `release_in_idle_is_rejected_with_stable_kinds` now
  passes a clone to `dispatch` and asserts the caller's original `RrcState`
  remains available; the `dispatch` rustdoc example compiles and asserts the
  same public usage.
- RLF cell retention:
  `radio_link_failure_returns_connected_serving_cell_to_idle` supplies distinct
  serving and reported cells, and asserts that `Idle.selected_cell` is the
  former. The temporary wrong-branch mutation failed this test.

## Self-Review

- No runtime transition behavior was changed.
- The documentation accurately states that rejection returns only stable kind
  metadata, so state retention is a caller responsibility.
- The RLF test can no longer pass if the transition accidentally selects the
  event payload cell.
- No dependencies were added; the unrelated untracked `.codebase-memory/`
  directory was left untouched.
