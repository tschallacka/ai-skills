# Testing companion: 01-step-static-acceptance-check

## Command verification

Run one bounded command after future implementation:

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

Record pass/fail evidence for file existence, one initial button, vertical below-button layout, current-last-button guard, generated-button terminal count, exact lowercase `finished`, document clear behavior, and white-border completion styling.

## Pass/fail criteria

Pass only when the command output records every acceptance point as present. Fail if any acceptance point is missing or ambiguous; record the failure in `bugs.md` before browser verification.
