# Step: 04-step-test-autoplay

## Ownership

- Goal: `08-autoplay`
- Work unit: `W44`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/autoplay.rs`
- Primary symbol or file scope: active_state_and_subject_selection
- Subscope: `N/A`

## Objective

§ 4.1
Pin the active-state derivation and the autoplay subject together: one active state yields one tab, several yield one tab each, a state leaving the active set removes its tab, and the subject the mode selects is the one autoplay follows, including the complete-mode case where there is none. Fault-inject a state with no active step, and a complete plan, and require the no-subject result rather than a stale one.

## Instructions

§ 5.1
Assert one active state yields one tab, several yield one each, a state leaving the set removes its tab, and an empty set yields the explicit no-active-step result. Assert the subject selection alongside it: implementing mode follows the active step, planning mode follows the plan being built, and complete mode reports no subject. Fault-inject a state whose step is complete and require it not to appear as active, and fault-inject a complete plan and require the no-subject result rather than the last subject the page held. An earlier version of this step pinned the active-state derivation only, while a coverage row claimed it pinned the subject; adversarial findings AR-07 and AR-25 recorded that it did not.

## Acceptance criteria

§ 6.1
The test fails if a completed step is treated as active or if a departed state leaves a stale tab.

## Handoff

§ 7.1
W45 exercises the same lifecycle live in the browser.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
