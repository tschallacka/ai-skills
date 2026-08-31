# Step: 02-step-frame-sampler

## Ownership

- Goal: `12-adaptive-cinematics`
- Work unit: `W72`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/perf.js`
- Primary symbol or file scope: `frameSampler`
- Subscope: `N/A`

## Objective

§ 4.1
Measure sustained performance rather than reacting to one slow frame.

## Instructions

§ 5.1
Sample frame times over a rolling window during animation and report a sustained figure. One slow frame from an unrelated cause must not change the tier, and sampling must cost less than the effects it governs.

## Acceptance criteria

§ 6.1
A single outlier frame does not alter the reported figure, and a sustained change does within the stated window. The sampler overhead is recorded.

## Handoff

§ 7.1
W73 applies a tier from this figure and the table.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
