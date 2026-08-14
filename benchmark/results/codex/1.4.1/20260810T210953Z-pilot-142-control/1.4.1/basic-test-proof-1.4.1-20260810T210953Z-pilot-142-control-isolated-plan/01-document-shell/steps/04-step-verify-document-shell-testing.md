# Testing companion: 04-step-verify-document-shell

## Command verification

After future shell implementation, run:

```bash
test -f button-chain.html
grep -q 'id="button-chain-root"' button-chain.html
grep -q 'class="chain-button"' button-chain.html
grep -q '.completion-state' button-chain.html
grep -Eq 'border:[^;]*white|border-color:[^;]*white|#fff|#ffffff' button-chain.html
grep -q '.chain-button' button-chain.html
grep -Eq 'display:[[:space:]]*block|flex-direction:[[:space:]]*column' button-chain.html
```

Record the command exit status and any failing predicate in the step handoff.

## Pass/fail criteria

Pass when the command evidence confirms all shell, style, and layout prerequisites for behavior work. Fail if any prerequisite is missing, ambiguous, or implemented in an unrelated file.
