# Verification: 03-step-independent-oracle-report

## Automated tests

- Extend `benchmark/planning/tests/test-review-oracle.sh` with encrypted-map,
  role-separation, missing-evidence, hash-mismatch, and complete classification
  fixtures for iterative and fresh-review modes.
- Require fail-closed output when the oracle key/map is exposed or target
  evidence is incomplete.
