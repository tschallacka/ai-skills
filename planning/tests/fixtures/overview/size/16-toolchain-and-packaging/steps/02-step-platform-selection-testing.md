# Verification: 02-step-platform-selection

## Automated tests

§ 2.1
Run the same installer selection path with PLAN_OVERVIEW_TEST_MODE=1 and explicit OS and architecture inputs for all supported target keys plus an unsupported input. Assert normalized key selected binaries.tsv path and W18 unavailable notice.