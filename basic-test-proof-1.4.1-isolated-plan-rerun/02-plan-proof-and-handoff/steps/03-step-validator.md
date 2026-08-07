# Step: 03-step-validator

## Ownership

- Goal: `02-plan-proof-and-handoff`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `planning readiness and isolation validation`
- Subscope: `N/A`

## Objective

§ 4.1
Run the structural plan validator and bounded planning context checks after review, then confirm the new plan directory contains no HTML and no prohibited browser/server/driver was started by this proof.

## Instructions

§ 5.1
After the separate sequential adversarial pass has no open findings, synchronize review status, run validate-plan.sh under the resource-limited wrapper, then run test-plan-context.sh --audit-triggers and --benchmark under the wrapper. Initialize a bounded context snapshot and run a non-registering all-entry check. Audit only this new plan directory for .html or .htm files and read process state to ensure this proof started no browser, server, or driver. Do not open any HTML path or terminate pre-existing processes.

## Acceptance criteria

§ 6.1
All bounded commands exit 0; validate-plan.sh reports seven work units across two goals; context audit and benchmark recommend continuation without correctness regression; snapshot initialization and all-entry check pass; the new directory contains no HTML or HTM file; and no prohibited process was started by this proof. Any failure remains unresolved and prevents W06 completion.

## Handoff

§ 7.1
W06 receives exact command outcomes, limitation disclosures, artifact inventory, and next-executor state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
