# Step: 01-step-separate-reviewer-authority

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W05`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `reviewer approval state calculation`
- Subscope: `N/A`

## Objective

§ 4.1
Use exactly one authoritative Reviewer B terminal approval, reject Reviewer A overall approvals as protocol violations, and distinguish missing, duplicate, conflicting, and rejected final approvals.

## Instructions

§ 5.1
Select approval candidates by lifecycle role only; exact session, capsule, mode, and freshness equality are exclusively W15 responsibilities. Treat Reviewer A approval attempts as protocol violations or handoffs, never as final approval. Emit separate reasons for missing, duplicate, conflicting, rejected, and unauthorized approvals without binding an artifact to a session here.

## Acceptance criteria

§ 6.1
A true/B false cannot create an undifferentiated conflict; only a candidate from Reviewer B can proceed to W15; missing or duplicate B candidates fail closed with explicit reasons, and W05 performs no exact session/capsule comparison.

## Handoff

§ 7.1
W06 receives one selected B artifact and W07 receives deterministic state reason names.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
