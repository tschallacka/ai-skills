# Step 06: static handoff

## Ownership
- Goal: `01-button-chain`
- Work unit: `W06`
- Type: `verification`

## Change target
- File: `N/A`
- Primary symbol or file scope: `tagged validator command`
- Subscope: `N/A`

## Objective
Run the one named tagged validator command for the durable plan structure.

## Instructions
1. Run `/tmp/20260810T121526Z-benchmark8/1.2.0/source/planning/scripts/validate-plan.sh` against this plan directory and hand the exact output and exit code to W07.

## Acceptance criteria
- The named validator exits zero and its complete output is available to W07.

## Handoff
- W07 owns the durable validator report; W09 and W10 own separate safety audits.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
