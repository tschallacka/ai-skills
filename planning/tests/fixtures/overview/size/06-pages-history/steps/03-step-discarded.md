# Step: 03-step-discarded

## Ownership

- Goal: `06-pages-history`
- Work unit: `W34`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/history.rs`
- Primary symbol or file scope: `render_discarded()`
- Subscope: `N/A`

## Objective

§ 4.1
Make discarded work and its reasons visible, since a plan is judged partly on what it rejected.

## Instructions

§ 5.1
Render removed work units, rejected alternatives from the approach decisions, and corrected paragraphs with what the earlier version said. The reason appears beside the discard, never in a separate place. A discard with no recorded reason is shown as missing a reason rather than omitted from the list.

## Acceptance criteria

§ 6.1
A rejected alternative shows its rationale, a corrected paragraph shows both wordings, and a discard lacking a reason is listed and flagged. US-07 and US-45 apply. Omitting an incomplete entry to keep the page tidy is the specific failure this criterion forbids.

## Handoff

§ 7.1
This surface is what makes a stale-wording sweep self-documenting for a later reader.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
