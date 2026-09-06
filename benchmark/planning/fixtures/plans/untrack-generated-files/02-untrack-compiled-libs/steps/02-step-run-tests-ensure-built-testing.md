# Verification: 02-step-run-tests-ensure-built

## Automated tests

§ 2.1
Remove the five libs and run ./run-tests.sh: they are rebuilt and the suite proceeds; with libs present, mtimes are unchanged (no rebuild); inject a syntax error into a lib function file and expect run-tests to fail with the compiler message.