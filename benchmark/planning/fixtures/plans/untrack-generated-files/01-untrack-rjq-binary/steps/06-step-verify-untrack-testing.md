# Verification: 06-step-verify-untrack

## Automated tests

§ 2.1
Execute the three probes in order and capture output: ls-files assertion, scratch install without the binary (expect exit 0 plus notice), npm pack --dry-run listing without planning/bin. Run the install probe under resource-limited-testing/scripts/limited-run.sh.