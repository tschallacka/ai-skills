# Working context: 20-blinded-seeded-defect-oracle

## Protocol decisions

- The seeder creates the defective copy but does not grade the review.
- Worker, iterative reviewer, fresh reviewer, and analyzer processes receive no
  plaintext defect map or encryption key.
- The independent oracle decrypts only after terminal target evidence exists.
- The durable report contains classifications and hashes, never the key or
  plaintext mapping.

## Handoff

- Outcome: encrypted seeding, target routing, independent grading, redacted
  report publication, and private-material cleanup are implemented. The live
  benchmark path is opt-in through `BLINDED_ORACLE_SPEC` and fails closed when
  terminal Reviewer B evidence or grading is unavailable.
- Files: `benchmark/planning/seed-blinded-defects.sh`,
  `benchmark/planning/setup-benchmark.sh`,
  `benchmark/planning/review-oracle.sh`, and
  `benchmark/planning/grade-blinded-run.sh`.
- Verification: blinded seeding/grading regression and shell syntax checks
  pass; live pilot archives still require a new seeded run before release
  adoption can be evaluated.
- Caveat: existing archives remain immutable and contain no genuine
  mode-by-mode blinded oracle reports.
