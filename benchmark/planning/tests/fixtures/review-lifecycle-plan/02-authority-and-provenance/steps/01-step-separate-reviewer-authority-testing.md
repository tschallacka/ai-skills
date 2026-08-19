# Verification: 01-step-separate-reviewer-authority

## Automated tests

Run lifecycle fixtures with A=true/B=false, A=false/B=true, missing B, and duplicate B approvals. Assert only B controls plan_approved and unauthorized A approval produces a named fail-closed reason.
