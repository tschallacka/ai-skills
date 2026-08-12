# Verification: 02-step-contract-tests

## Automated tests

- Run `bash planning/tests/test-plan-env.sh`.
- Run `planning/scripts/validate-plan.sh .plans/reviewer-optimization-1-4-2`.
- Pass only when malformed, unknown, stale, symlinked, weak-mode, and foreign-
  root manifests fail without executing injected content.
