# Working context: 12-environment-contract-evolution

## Handoff

- Outcome: the planning skill now requires coordinated manifest schema
  migrations and explicitly forbids backward-compatible aliases, adapters,
  legacy modes, and inferred defaults.
- Verification: focused manifest tests and plan validation pass after the
  protocol is recorded.
- Caveat: future schema changes must update producer, consumers, tests,
  package inventory, and fresh adversarial evidence in one plan mutation.
