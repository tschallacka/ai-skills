# Step: 01-step-contract-protocol

## Ownership

- Goal: `12-environment-contract-evolution`
- Work unit: `W70`
- Type: `docs`

## Change target

- File: `planning/SKILL.md`
- Primary symbol or file scope: `environment contract evolution protocol`
- Subscope: `N/A`

## Objective

Document the mandatory no-backward-compatibility protocol for adding or
changing planning environment variables.

## Instructions

Update the planning skill and plan contract to require producer/consumer,
package, test, validator, and adversarial-review updates together. Require
fail-closed rejection of stale or mismatched manifests and removal of
superseded variables from allowlists and documentation.

## Acceptance criteria

The protocol is explicit, actionable, and contains no compatibility fallback.

## Handoff

W71 verifies the protocol and schema rejection behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
