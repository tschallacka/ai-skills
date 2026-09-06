# Verification: 01-step-bootstrap-rjq

## Automated tests

§ 2.1
Run bootstrap.sh on a clean checkout without rjq: builds into the gitignored planning/bin path and prints a working PATH line; with rjq on PATH it is a no-op exiting 0; hide cargo to expect exit 69 naming the T70 release note.