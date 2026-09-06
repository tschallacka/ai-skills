# Verification: 02-step-register-schemas-guard

## Automated tests

§ 2.1
Unset rjq and run tests/test-register-schemas.sh: the guard fires naming bootstrap.sh and T70 (not command-not-found); with rjq on PATH the test passes unchanged; confirm the test-rjq-active-references.sh message names the same fix.