# Step: 06-step-autoplay-availability

## Ownership

- Goal: `08-autoplay`
- Work unit: `W53`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/autoplay.rs`
- Primary symbol or file scope: autoplay_subject()
- Subscope: `N/A`

## Objective

§ 4.1
Autoplay's subject is decided by the mode. In implementing mode it follows the active step. In planning mode it follows the plan being built: units appearing, dependency edges forming. In complete mode there is no active subject, so autoplay is unavailable and states why rather than offering a control that cannot move. An earlier version of this row said autoplay exists in every mode, which adversarial findings AR-08 and AR-24 recorded as contradicting US-64 and plan 8.6.

## Instructions

§ 5.1
Select the autoplay subject from the derived mode: in implementing mode follow the active step; in planning mode follow the plan being built, meaning units appearing, edges forming and findings attaching. State the subject so a reader knows what they are watching. Complete mode has no active subject yet and is recorded as an open question rather than invented here.

## Acceptance criteria

§ 6.1
The stated subject matches the derived mode, planning mode animates plan construction rather than offering an inert toggle, and complete mode says plainly that nothing is being followed. US-64 applies, revised from an earlier reading that autoplay was meaningless outside execution.

## Handoff

§ 7.1
W56 provides the planning-mode animation; W50 provides the mode.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
