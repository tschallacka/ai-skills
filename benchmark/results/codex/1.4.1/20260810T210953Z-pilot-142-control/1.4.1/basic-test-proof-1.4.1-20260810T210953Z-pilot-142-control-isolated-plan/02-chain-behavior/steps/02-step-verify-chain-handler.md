# Step: 02-step-verify-chain-handler

## Ownership

- Goal: `02-chain-behavior`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Chain behavior source inspection`
- Subscope: `N/A`

## Objective

§ 4.1
Verify the future click handler accepts only the current last button and treats the fourth generated button as terminal before final browser proof.

## Instructions

§ 5.1
After the future handler is implemented, run this bounded command from the repository root and save its output in the step handoff:

```bash
test -f button-chain.html
grep -q 'handleChainClick' button-chain.html
grep -Eq 'last(Button|Element|Child)|currentLast' button-chain.html
grep -Eq 'generated(Button)?Count|generatedCount|data-generated' button-chain.html
grep -Eq '===?[[:space:]]*4|>=?[[:space:]]*4' button-chain.html
grep -q 'finished' button-chain.html
grep -q 'completion-state' button-chain.html
grep -Eq 'replaceChildren|document\\.body\\.innerHTML|textContent[[:space:]]*=' button-chain.html
```

The command recipe checks for the named handler, a current-last-button guard, generated-button state, fourth-generated terminal branch, exact terminal text, completion styling, and document replacement.

## Acceptance criteria

§ 6.1
The inspection output records pass/fail evidence for the current-last guard, one-button append operation, fourth-generated terminal condition, exact terminal text, and document clear operation.

## Handoff

§ 7.1
`W04` and `W05` can rely on the handler source having passed local proof before final acceptance inspection and browser story execution.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
