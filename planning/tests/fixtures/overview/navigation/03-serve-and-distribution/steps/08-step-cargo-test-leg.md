# Step: 08-step-cargo-test-leg

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W75`
- Type: `config`

## Change target

- File: `run-tests.sh`
- Primary symbol or file scope: `test discovery`
- Subscope: `N/A`

## Objective

§ 4.1
Discover and run the crate test suite alongside the shell tests, so a failing cargo test fails the repository gate. Today discovery is a single find for test-star.sh at line 87, which means no Rust test can ever redden the suite however many the plan declares.

## Instructions

§ 5.1
In run-tests.sh, alongside the existing discovery of test-star.sh files at line 87, discover and run the crate test suite, reporting each crate test as a case in the same summary as the shell tests. Do not replace the shell discovery; add to it. Where no Rust toolchain is present the leg reports itself unconfigured and the shell suite still runs, which is the same honest-degradation rule the repository already applies to the mermaid render check.  Read the refuse variable here, in run-tests.sh, and branch on it: set, a missing toolchain makes the leg exit non-zero and the suite fail; unset, it reports itself unconfigured and the shell suite still runs. W112 sets the variable in CI, but setting it there does nothing unless this file reads it, and an earlier version of this instruction attributed the whole mechanism to W112 while W112's change target is ci.yml alone — adversarial finding AR-51 recorded that an executor following it would implement only the degrading arm and then be unable to discharge this unit's own acceptance criteria.

## Acceptance criteria

§ 6.1
A failing crate test makes the whole suite exit non-zero and names the failing test. Proven by breaking one crate test deliberately: without this unit the suite stays green with a broken crate test, which is the defect. On a machine with no Rust toolchain the suite still runs the shell tests and states that the crate leg is unconfigured rather than failing or silently skipping. The strict arm is pinned here rather than described: with the refuse variable set and no toolchain on PATH, the leg exits non-zero and the suite fails, and with the variable unset in the same conditions it reports itself unconfigured and the shell suite still runs. Both are asserted, because adversarial finding AR-41 recorded that only the degrading half had a stated proof while the arm that protects the gate had none.

## Handoff

§ 7.1
W48 can rely on one command running both the shell and crate suites, so its recorded result covers the crate tests the earlier goals declare rather than only the shell ones.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
