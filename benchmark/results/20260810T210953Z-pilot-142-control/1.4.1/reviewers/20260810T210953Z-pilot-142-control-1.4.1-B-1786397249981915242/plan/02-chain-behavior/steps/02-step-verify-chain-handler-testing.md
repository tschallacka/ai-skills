# Testing companion: 02-step-verify-chain-handler

## Command verification

After future behavior implementation, run:

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

Record the command exit status and any failing predicate in the step handoff.

## Pass/fail criteria

Pass when the command evidence confirms the handler can satisfy the full click contract before browser proof. Fail if terminal logic is off by one, generated count is conflated with total button count, non-last clicks can append, or terminal rendering is wrong.
