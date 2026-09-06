# Verification: 02-step-portability-test-temp

## Automated tests

§ 2.1
Run planning/tests/test-portability-contract.sh without PORTABILITY.md (pass by regeneration); add a bogus PORTABILITY() marker id to a script and expect the sweep to fail; without rjq on PATH expect UNCONFIGURED, not failure.