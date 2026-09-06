# Verification: 01-step-verify-bootstrap-suite

## Automated tests

§ 2.1
git archive HEAD into TMPDIR, run the bootstrap once, then ./run-tests.sh under the resource wrapper to green; assert git ls-files names none of the four classes; record the archived commit id.