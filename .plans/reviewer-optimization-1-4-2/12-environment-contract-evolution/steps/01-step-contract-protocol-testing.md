# Verification: 01-step-contract-protocol

## Automated tests

- Run `bash planning/tests/test-plan-env.sh`.
- Run `bash planning/tests/test-installer-manifest.sh`.
- Run `planning/scripts/validate-plan.sh .plans/reviewer-optimization-1-4-2`.
- Pass only when the skill and plan both state that backward-compatible
  aliases, adapters, legacy modes, and inferred defaults are prohibited.
