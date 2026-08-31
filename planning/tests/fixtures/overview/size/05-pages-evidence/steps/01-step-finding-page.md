# Step: 01-step-finding-page

## Ownership

- Goal: `05-pages-evidence`
- Work unit: `W27`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/findings.rs`
- Primary symbol or file scope: `render_finding()`
- Subscope: `N/A`

## Objective

§ 4.1
Show a finding in full, since findings currently run under the adjacent panel and are cut at the column edge.

## Instructions

§ 5.1
Render the finding evidence, impact, observed contradiction and required correction, its status, the work unit it names as a link, and its gated fix-key claim where one exists. A finding with no owning unit states that rather than rendering an empty cell.

## Acceptance criteria

§ 6.1
No finding text is clipped, overlapped or truncated at any width tried; the named unit opens in one click; a blank work-unit cell reads as no owning unit. US-05, US-40 and US-41 apply.

## Handoff

§ 7.1
W33 renders the superseded view of the same findings; both read one derivation.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
