# Verification: 10-step-verify-clean-suite

## Automated tests

§ 2.1
git archive HEAD into a TMPDIR tree and run ./run-tests.sh under resource-limited-testing/scripts/limited-run.sh; capture the summary line and confirm the five libs exist afterwards and are ignored.