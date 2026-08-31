# Step: 07-step-verify-shell

## Ownership

- Goal: `02-page-shell`
- Work unit: `W13`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `artifact works from disk, served and deep-linked`
- Subscope: `N/A`

## Objective

§ 4.1
Prove the one artifact genuinely works in all three ways it claims: from disk, served, and deep-linked.

## Instructions

§ 5.1
Open the artifact from the filesystem and navigate two pages deep. Serve it and repeat. Deep-link straight to a unit page, then walk back through the stack, then open a peek without navigating. Record the interaction and the observed result for each.

## Acceptance criteria

§ 6.1
US-02, US-03, US-16 and US-33 pass with recorded interactions; any feature genuinely requiring the server states its unavailability from disk rather than failing silently.

## Handoff

§ 7.1
The recorded evidence is the baseline the mode and live-update stories build on.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
