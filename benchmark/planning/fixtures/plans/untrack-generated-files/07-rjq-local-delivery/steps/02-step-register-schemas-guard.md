# Step: 02-step-register-schemas-guard

## Ownership

- Goal: `07-rjq-local-delivery`
- Work unit: `W38`
- Type: `test`

## Change target

- File: `tests/test-register-schemas.sh`
- Primary symbol or file scope: `rjq availability guard`
- Subscope: `N/A`

## Objective

§ 4.1
Add a command -v rjq guard at the top that fails with the bootstrap.sh instruction and the T70 release note instead of a bare command-not-found; confirm tests/test-rjq-active-references.sh's existing unavailability message names the same fix.

## Instructions

§ 5.1
In tests/test-register-schemas.sh add a command -v rjq guard before any rjq call that fails with a message naming bootstrap.sh and the T70 release note; keep the rest of the test untouched. Read tests/test-rjq-active-references.sh's unavailability path and confirm its message names the same fix; adjust only its wording if it does not.

## Acceptance criteria

§ 6.1
With rjq absent the guard fires with the named instruction (not command-not-found); with rjq on PATH the test behaves exactly as before; test-rjq-active-references.sh's message names the same fix.

## Handoff

§ 7.1
W31's clean-checkout suite run relies on the guard turning a missing tool into an actionable instruction.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
