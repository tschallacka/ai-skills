# Verification: 04-step-tracked-binary-guard

## Automated tests

§ 2.1
Run tests/test-shipped-binaries.sh (pass); stage a dummy file under planning/bin, re-run, expect failure naming MAINTAINER.md section 2.15; unstage and re-run (pass).