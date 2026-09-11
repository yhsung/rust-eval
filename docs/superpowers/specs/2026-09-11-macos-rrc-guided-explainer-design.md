# macOS RRC Guided Explainer — Design

## Purpose

Add a dependency-free browser application that explains the Rust Modem Safety
Island PoC to modem/RAN engineers, embedded-software teams, and technical
decision makers. It must make typed state transitions, resource ownership, and
compiler-enforced coverage visible without claiming to be a production modem or
a second protocol implementation.

## Audience and Teaching Model

The application uses a guided step-through model. Each screen presents one
transition first, then lets the viewer reveal its technical and business
implications:

- **Protocol layer:** previous state, typed event, next state, and outcome.
- **Ownership layer:** which state owns T300 or cell context, and why that
  resource cannot exist in an invalid destination state.
- **Coverage layer:** how an exhaustive `State x Event` matcher forces a
  decision when an event is added.

The default narration is concise enough for a decision-maker presentation;
technical detail is available in-place for engineering discussion.

## Scope

Create a static browser app in `web/`:

- `web/index.html` provides the app shell and accessible semantic controls.
- `web/styles.css` provides a macOS-friendly presentation layout that fits a
  laptop screen and remains usable at narrow widths.
- `web/app.js` renders a finite, scripted set of teaching steps and updates the
  selected state, event, explanation, and coverage view.

The guided sequence contains:

1. `Idle + StartConnection -> Connecting` with T300 creation.
2. `Connecting + SetupComplete -> Connected` with T300 consumption.
3. `Connected + Release -> Idle` with serving-cell retention.
4. Branch demonstrations for `T300Expiry`, `RadioLinkFailure`, and an
   unsupported `Release` in `Idle`.

The coverage view displays the three states and the four events in
`examples/exhaustive_state_event.rs`. It then visualizes adding
`MeasurementReport` as unhandled cells. It must say that the visual is an
explanation of the real compile-fail test, not a browser compiler.

## Source of Truth and Synchronization

`src/machine.rs` remains the only protocol-transition authority. The browser
app is a fixed narrative representation of the transitions already covered by
Rust tests; it does not accept arbitrary payloads or replace `dispatch`.

When a supported transition, outcome, or stated invariant changes, the same
change must update `web/app.js`, browser tests, Rust tests, and README if its
scenario list changes. Browser labels must use the actual Rust names:
`Idle`, `Connecting`, `Connected`, `StartConnection`, `SetupComplete`,
`T300Expiry`, `Release`, and `RadioLinkFailure`.

## Interaction Design

- A visible progress indicator identifies the current guided step.
- Previous and next buttons traverse the primary flow.
- Branch buttons load T300 expiry, RLF, and unexpected-event demonstrations.
- A technical-detail control reveals state payloads, T300 ownership, and the
  explicit rejection contract without distracting from the default view.
- A coverage button switches to the State x Event table and event-addition
  demonstration.
- The active state and active step are visibly distinct and exposed using
  semantic labels and `aria-live` updates.

## AGENTS.md Consolidation

Rewrite `AGENTS.md` into short sections covering:

1. Project purpose and non-goals.
2. Repository map: Rust engine, browser explainer, compile-fail fixture, tests,
   and docs.
3. Required verification commands: `cargo test`, `cargo fmt --check`, and
   `cargo clippy -- -D warnings`; browser test command when applicable.
4. Domain invariants: `Connecting` owns T300, `Connected` has no T300,
   unexpected runtime events are explicit, and the fixture contains no wildcard
   match arm.
5. Browser synchronization and wording rules: do not call it a compiler,
   standards-conformant modem, production implementation, or timing proof.

## Testing

Use browser-platform APIs only; no npm dependency is permitted. `web/app.js`
must expose its scripted data and pure rendering-state helpers through a
testable module interface. A Node built-in test file will verify the guided
flow, branch labels, timer ownership messages, RLF and unexpected-event
explanations, and the coverage warning. The Rust suite continues to verify the
actual model and dispatcher. A static-content Rust test will check that
`AGENTS.md` contains the required browser synchronization and no-overclaim
rules.

## Non-goals

- Native macOS app packaging, signing, or installation.
- WASM, an HTTP backend, IPC, FFI, or a browser connection to a running modem.
- Arbitrary event injection, real timing, OTA decoding, or 3GPP conformance.
- Replacing existing Rust tests with browser tests.

## Acceptance Criteria

- Opening `web/index.html` in a macOS browser presents a guided RRC safety
  explanation with the named primary and branch flows.
- The app visibly explains that T300 exists only in `Connecting` and is absent
  from `Connected`.
- The coverage view distinguishes a visualization from the real Rust
  compile-fail check.
- Node's built-in test runner and the existing Rust verification commands pass
  with no added dependencies.
- `AGENTS.md` accurately describes the browser app, synchronization rules,
  required commands, and non-overclaim boundaries.
