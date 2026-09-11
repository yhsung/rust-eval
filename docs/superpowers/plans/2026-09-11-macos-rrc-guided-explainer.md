# macOS RRC Guided Explainer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a dependency-free macOS browser app that guides modem/RAN engineers and decision makers through the Rust PoC's RRC state, ownership, and compiler-coverage concepts.

**Architecture:** `web/app.js` owns a finite narrative data model and pure view-state helpers; it never implements arbitrary protocol dispatch. The browser shell renders that data with native browser APIs and can run from `file://`. Rust remains the protocol source of truth, while Node built-in tests verify the browser narrative and Rust tests verify contributor-rule text.

**Tech Stack:** Rust stable and Cargo; HTML/CSS/vanilla JavaScript; Node.js built-in `node:test` and `assert`; no third-party dependencies or build step.

**Spec:** `docs/superpowers/specs/2026-09-11-macos-rrc-guided-explainer-design.md`

## Global Constraints

- Keep Rust and browser implementation dependency-free; use no npm packages or build tools.
- The browser app is a fixed narrative representation, not a second protocol dispatcher or a production modem.
- Browser labels must exactly use `Idle`, `Connecting`, `Connected`, `StartConnection`, `SetupComplete`, `T300Expiry`, `Release`, and `RadioLinkFailure`.
- Explain that T300 exists only in `Connecting` and is absent from `Connected`.
- The coverage view must say it visualizes the real Rust compile-fail test; it must not claim the browser is a compiler.
- Preserve explicit runtime unexpected-event handling and no wildcard arm in `examples/exhaustive_state_event.rs`.
- Required final checks are `node --test web/app.test.js`, `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings`.

---

### Task 1: Create the testable narrative model

**Files:**
- Create: `web/app.js`
- Create: `web/app.test.js`

**Interfaces:**
- Produces: `RrcExplainer.primarySteps`, `RrcExplainer.branchSteps`, `RrcExplainer.coverage`, `RrcExplainer.initialView()`, `RrcExplainer.next(view)`, `RrcExplainer.previous(view)`, and `RrcExplainer.selectBranch(view, id)`.
- Consumed by: Task 2 browser renderer and Task 1 Node tests.

- [ ] **Step 1: Write failing Node narrative tests**

```js
const test = require("node:test");
const assert = require("node:assert/strict");
const explainer = require("./app.js");

test("primary narrative preserves Rust transition names and T300 ownership", () => {
  assert.deepEqual(
    explainer.primarySteps.map((step) => step.id),
    ["start-connection", "setup-complete", "release"],
  );
  assert.match(explainer.primarySteps[0].nextState.detail, /T300/);
  assert.match(explainer.primarySteps[1].nextState.detail, /no T300/);
  assert.equal(explainer.primarySteps[1].event.name, "SetupComplete");
});

test("branches retain RLF and unexpected-event teaching contracts", () => {
  assert.match(explainer.branchSteps.radioLinkFailure.explanation, /serving_cell/);
  assert.match(explainer.branchSteps.unexpectedEvent.explanation, /UnexpectedEvent/);
  assert.equal(explainer.selectBranch(explainer.initialView(), "t300Expiry").mode, "branch");
});

test("coverage warns that Rust performs the actual compiler check", () => {
  assert.equal(explainer.coverage.addedEvent, "MeasurementReport");
  assert.match(explainer.coverage.warning, /compile-fail test/);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `node --test web/app.test.js`

Expected: FAIL because `web/app.js` does not exist.

- [ ] **Step 3: Implement the narrative-only CommonJS/browser module**

```js
const RrcExplainer = (() => {
  const primarySteps = [
    {
      id: "start-connection",
      previousState: { name: "Idle", detail: "selected_cell" },
      event: { name: "StartConnection", payload: "transaction_id = 1" },
      nextState: { name: "Connecting", detail: "selected_cell · T300 · transaction_id" },
      outcome: "ConnectionStarted",
    },
  ];

  function initialView() { return { mode: "guided", stepIndex: 0, branchId: null, showTechnical: false }; }
  function selectBranch(view, id) { return { ...view, mode: "branch", branchId: id }; }

  return { primarySteps, branchSteps, coverage, initialView, next, previous, selectBranch };
})();

if (typeof module !== "undefined") module.exports = RrcExplainer;
if (typeof window !== "undefined") window.RrcExplainer = RrcExplainer;
```

Define all three primary transitions plus branch records `t300Expiry`,
`radioLinkFailure`, and `unexpectedEvent`. Each record must contain actual Rust
names, state/event/outcome labels, a concise engineering explanation, and a
decision-maker explanation. Define a `coverage` record with three states, four
fixture events, added event `MeasurementReport`, and a warning that names the
Rust compile-fail test. `next` and `previous` clamp the guided index; they do
not simulate arbitrary event dispatch.

- [ ] **Step 4: Run Node tests to verify green**

Run: `node --test web/app.test.js`

Expected: PASS; the primary flow, branch contracts, and coverage warning are asserted without a DOM or dependency.

- [ ] **Step 5: Commit the narrative model**

```bash
git add web/app.js web/app.test.js
git commit -m "feat: add rrc explainer narrative model"
```

### Task 2: Render the macOS browser guided experience

**Files:**
- Create: `web/index.html`
- Create: `web/styles.css`
- Modify: `web/app.js`
- Modify: `web/app.test.js`

**Interfaces:**
- Consumes: Task 1's `window.RrcExplainer` records and view-state helpers.
- Produces: `RrcExplainer.mount(document)` and a `file://` runnable interface with guided, branch, technical-detail, and coverage controls.
- Consumed by: manual macOS browser verification and Task 3 documentation.

- [ ] **Step 1: Write failing mount/render tests using a minimal fake document**

```js
test("mount renders the first guided transition and live result", () => {
  const root = fakeRoot();
  explainer.mount(fakeDocument(root));

  assert.match(root.innerHTML, /Idle/);
  assert.match(root.innerHTML, /StartConnection/);
  assert.match(root.innerHTML, /Connecting/);
  assert.match(root.innerHTML, /aria-live="polite"/);
});

test("coverage render names MeasurementReport and the Rust compile-fail test", () => {
  const root = fakeRoot();
  explainer.render(root, { ...explainer.initialView(), mode: "coverage" });
  assert.match(root.innerHTML, /MeasurementReport/);
  assert.match(root.innerHTML, /compile-fail test/);
});
```

Define `fakeRoot()` as `{ innerHTML: "", querySelector: () => null }` and a
minimal `fakeDocument(root)` returning that root for `getElementById`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `node --test web/app.test.js`

Expected: FAIL because `mount` and `render` do not exist.

- [ ] **Step 3: Build the static browser shell and renderer**

Create `web/index.html` with `<main id="app">`, a normal `<script src="app.js">`
tag (not an ES module, so direct `file://` opening works), and a no-JavaScript
fallback. Create `web/styles.css` with responsive CSS only; use system font
stacks, high-contrast active-state styling, visible focus styles, and no remote
assets.

Implement `mount(document)` to attach to `#app`, initialize `initialView()`,
and attach click handlers using event delegation. Implement `render(root, view)`
to render:

```html
<section aria-live="polite">
  <p>Step 1 of 3</p>
  <div class="transition"><article>Idle</article><span aria-hidden="true">→</span><article>Connecting</article></div>
  <p>Event: StartConnection</p>
</section>
```

Render Previous and Next controls, branch controls labeled `T300Expiry`,
`RadioLinkFailure`, and `Unexpected event`, a Show technical detail toggle,
and a Show compiler coverage control. Technical detail must visibly explain
T300 ownership. Coverage must show State x Event cells, mark
`MeasurementReport` unhandled in every state, and include the exact caution:
`This view explains the Rust compile-fail test; it does not compile Rust in the browser.`

At the bottom of `web/app.js`, only call `RrcExplainer.mount(document)` when
`typeof document !== "undefined"`; Node test imports must not manipulate a DOM.

- [ ] **Step 4: Verify browser logic and manual file opening**

Run:

```bash
node --test web/app.test.js
open web/index.html
```

Expected: Node tests PASS. In the macOS browser, the guided flow starts at
Idle, buttons change views, technical detail toggles, and coverage shows the
compiler caution.

- [ ] **Step 5: Commit the browser experience**

```bash
git add web/index.html web/styles.css web/app.js web/app.test.js
git commit -m "feat: add macos rrc guided explainer"
```

### Task 3: Consolidate contributor rules and verify synchronization wording

**Files:**
- Modify: `AGENTS.md`
- Create: `tests/contributor_rules.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: browser paths from Task 2 and Rust invariants from `src/model.rs`,
  `src/machine.rs`, and `examples/exhaustive_state_event.rs`.
- Produces: concise operating rules and an executable Rust test that guards their required wording.
- Consumed by: contributors and final verification.

- [ ] **Step 1: Write the failing contributor-rule test**

```rust
#[test]
fn agents_document_browser_sync_and_protocol_boundaries() {
    let agents = std::fs::read_to_string("AGENTS.md").unwrap();

    for required in [
        "web/",
        "cargo test",
        "node --test web/app.test.js",
        "Connecting owns T300",
        "Connected has no T300",
        "no-wildcard",
        "compile-fail",
        "source of truth",
        "not a compiler",
        "not 3GPP-compliant",
    ] {
        assert!(agents.contains(required), "AGENTS.md must contain {required}");
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test contributor_rules`

Expected: FAIL because `tests/contributor_rules.rs` does not exist.

- [ ] **Step 3: Consolidate AGENTS.md and update README launch instructions**

Rewrite `AGENTS.md` with concise headings: Purpose and non-goals; Repository
map; Required verification; Domain invariants; Browser explainer synchronization;
and Claim boundaries. Include all exact test strings above. State that
`src/machine.rs` is the source of truth; browser narrative changes must track
Rust event/state/outcome changes; and the browser must never claim it compiles
Rust or proves 3GPP conformance.

Update README with a `macOS browser explainer` section that directs the user to
run `open web/index.html`, lists the guided and branch demonstrations, and
identifies it as a visualization of the Rust PoC rather than a modem emulator.

- [ ] **Step 4: Verify documentation behavior and full Rust suite**

Run:

```bash
cargo test --test contributor_rules
cargo test
node --test web/app.test.js
```

Expected: all commands PASS; contributor requirements are protected by a real test and browser narrative tests remain green.

- [ ] **Step 5: Commit contributor documentation**

```bash
git add AGENTS.md README.md tests/contributor_rules.rs
git commit -m "docs: add browser explainer contributor rules"
```

### Task 4: Final cross-layer verification

**Files:**
- Verify: `web/index.html`, `web/styles.css`, `web/app.js`, `web/app.test.js`, `AGENTS.md`, `README.md`, Rust source, and all tests.

**Interfaces:**
- Consumes: the finished static browser narrative, contributor rules, and existing Rust PoC.
- Produces: a verified local branch ready for the configured GitHub remote.

- [ ] **Step 1: Inspect repository status and delivery diff**

Run:

```bash
git status --short --branch
git diff --check
git log --oneline --decorate -8
```

Expected: no uncommitted delivery files or whitespace errors; local brainstorming and index cache paths remain untracked and excluded from commits.

- [ ] **Step 2: Run all automated verification**

Run:

```bash
node --test web/app.test.js
cargo test
cargo fmt --check
cargo clippy -- -D warnings
```

Expected: all suites and quality checks PASS with no warnings.

- [ ] **Step 3: Manually verify the macOS presentation**

Run: `open web/index.html`

Expected: the page opens with the first guided step; keyboard and pointer
controls can reach each primary and branch view; technical detail and coverage
views are readable and include the compiler caution.

- [ ] **Step 4: Push after explicit authorization**

Run: `git push origin main`

Expected: verified commits are published to `github.com/yhsung/rust-eval`.
