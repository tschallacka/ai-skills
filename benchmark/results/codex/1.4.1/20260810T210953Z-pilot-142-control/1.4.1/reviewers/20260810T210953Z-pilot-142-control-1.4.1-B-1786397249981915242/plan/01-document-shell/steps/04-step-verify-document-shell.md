# Step: 04-step-verify-document-shell

## Ownership

- Goal: `01-document-shell`
- Work unit: `W07`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Document shell source inspection`
- Subscope: `N/A`

## Objective

§ 4.1
Verify the future document shell has one initial button, completion styling, and vertical chain-button layout before behavior work begins.

## Instructions

§ 5.1
After the future document-shell steps are implemented, run this bounded command from the repository root and save its output in the step handoff:

```bash
test -f button-chain.html
grep -q 'id="button-chain-root"' button-chain.html
grep -q 'class="chain-button"' button-chain.html
grep -q '.completion-state' button-chain.html
grep -Eq 'border:[^;]*white|border-color:[^;]*white|#fff|#ffffff' button-chain.html
grep -q '.chain-button' button-chain.html
grep -Eq 'display:[[:space:]]*block|flex-direction:[[:space:]]*column' button-chain.html
```

## Acceptance criteria

§ 6.1
The inspection output records pass/fail evidence for the one-button initial state, terminal style selector, and vertical button-stack style. Any failure blocks `W03` until the plan is revised with explicit investigation and fix work units or the failing step is redone within its owned target.

## Handoff

§ 7.1
`W03` can rely on the document shell, initial button, terminal style, and below-placement style being present.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
