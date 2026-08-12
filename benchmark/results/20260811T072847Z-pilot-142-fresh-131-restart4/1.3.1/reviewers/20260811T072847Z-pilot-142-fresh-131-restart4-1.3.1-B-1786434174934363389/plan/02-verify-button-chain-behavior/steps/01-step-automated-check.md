# Step: 01-step-automated-check

## Ownership

- Goal: `02-verify-button-chain-behavior`
- Work unit: `W04`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `static-and-simulated HTML behavior check`
- Subscope: `N/A`

## Objective

§ 4.1
Run a bounded local verification that inspects button-chain.html and simulates the click sequence to confirm initial count, exact one-button appends, fourth generated button clearing, exact finished text, and white border styling.

## Instructions

§ 5.1
After implementation, run a bounded node heredoc from the workspace root that reads ./button-chain.html with node:fs, asserts with node:assert/strict, executes page script in node:vm, and uses a small in-memory DOM harness declared inside the heredoc.

§ 5.2
The harness must model enough document, element, button, append, replace, textContent, class, and click behavior to exercise the page's own click handler without opening a browser or using injected browser events.

§ 5.3
Assert that the file starts with one button, each valid last-button click appends exactly one button, earlier buttons do not append, the fourth generated button clears previous content, the resulting text is exactly finished, and the completion style declares a visible white border.

§ 5.4
Paste the exact command, exit code, and assertion summary into 01-step-automated-check-testing.md before marking this step complete.

## Acceptance criteria

§ 6.1
The automated check exits successfully and reports all required behavior and styling assertions as passing.

§ 6.2
Failure output is captured and any required fix is planned before rerunning.

## Handoff

§ 7.1
W05 can proceed when W04 has passing automated evidence.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
