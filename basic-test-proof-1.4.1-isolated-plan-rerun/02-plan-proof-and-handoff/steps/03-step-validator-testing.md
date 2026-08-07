# Testing: 03-step-validator

## Automated tests

- Run `planning/scripts/validate-plan.sh <plan-directory>` through the resource-limited wrapper; require exit 0 and seven work units across two goals.
- Run `planning/tests/test-plan-context.sh --audit-triggers` and `--benchmark` through the wrapper; require exit 0 and no correctness regression.
- Initialize the plan context snapshot and audit all entries without registering reads; require exit 0.
- Search only the new plan directory for `.html` or `.htm`; require no match.

## Process verification

- Read process state and confirm this proof started no browser, server, or driver. Do not terminate pre-existing processes.
