# Step: 01-step-static-acceptance-check

## Ownership

- Goal: `03-verification-handoff`
- Work unit: `W04`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Static acceptance inspection command`
- Subscope: `N/A`

## Objective

§ 4.1
Inspect button-chain.html source after implementation for exact file, initial button, generated-button counter, lowercase finished text, and white-border CSS.

## Instructions

§ 5.1
After implementation, run this final bounded source inspection command from the repository root and save its output in the step handoff:

```bash
test -f button-chain.html
grep -q 'id="button-chain-root"' button-chain.html
grep -q 'class="chain-button"' button-chain.html
grep -q '.chain-button' button-chain.html
grep -Eq 'display:[[:space:]]*block|flex-direction:[[:space:]]*column' button-chain.html
grep -q 'handleChainClick' button-chain.html
grep -Eq 'last(Button|Element|Child)|currentLast' button-chain.html
grep -Eq 'generated(Button)?Count|generatedCount|data-generated' button-chain.html
grep -Eq '===?[[:space:]]*4|>=?[[:space:]]*4' button-chain.html
grep -q 'finished' button-chain.html
grep -Eq 'border:[^;]*white|border-color:[^;]*white|#fff|#ffffff' button-chain.html
```

## Acceptance criteria

§ 6.1
The command output records pass/fail evidence for each static acceptance point. Any failure is entered in `bugs.md` before further verification proceeds.

## Handoff

§ 7.1
`W05` can proceed only when static inspection finds the expected file and no obvious mismatch in text, style, or generated-button terminal logic.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
