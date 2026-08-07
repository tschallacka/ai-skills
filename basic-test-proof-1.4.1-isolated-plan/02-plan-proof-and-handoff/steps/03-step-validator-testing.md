# Testing companion: 03-step-validator

## Automated verification

Run `planning/scripts/validate-plan.sh basic-test-proof-1.4.1-isolated-plan` from the repository root after review approval. Run the required plan-context audit and benchmark through the resource-limited testing wrapper. Inspect only the new plan directory for prohibited HTML or implementation artifacts.

## Pass/fail criteria

Pass when structural validation reports seven work units across two goals, context checks succeed, no prohibited artifact exists in the new plan directory, and no browser, server, or driver was started by this proof.
