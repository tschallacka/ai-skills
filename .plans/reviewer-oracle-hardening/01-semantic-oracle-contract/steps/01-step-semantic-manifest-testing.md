# Testing: Goal 01 / Step 01

## Automated tests

Extend `benchmark/planning/tests/test-review-oracle.sh` with valid and invalid
semantic manifests, multiple mutations in one file, private-material access
checks, and schema validation. Run:

```bash
benchmark/planning/tests/test-review-oracle.sh
```

Pass requires deterministic seeding, final hashes, encrypted private material,
and fail-closed validation for missing semantic fields.
