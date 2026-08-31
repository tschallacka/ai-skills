# Step: 05-step-availability-notice

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W18`
- Type: `source`

## Change target

- File: `installer/src/20-runtime-tools.sh`
- Primary symbol or file scope: overview availability notice branch
- Subscope: `N/A`

## Objective

§ 4.1
Own only the unavailable-platform notice branch in installer/src/20-runtime-tools.sh; W104 owns artifact selection and placement in the same file.

## Instructions

§ 5.1
Own only the unavailable-platform notice branch in installer/src/20-runtime-tools.sh. When W104's host selection finds no supported pair, print that the plan overview is unavailable and name the detected platform; do not change selection or placement here.

## Acceptance criteria

§ 6.1
On a host with no matching artifact the message names the platform and the unavailable feature, the exit status is deliberate rather than incidental, and the remaining skill files install normally. US-50 covers the runtime equivalent.

## Handoff

§ 7.1
W20 verifies this on a simulated platform with no artifact.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
