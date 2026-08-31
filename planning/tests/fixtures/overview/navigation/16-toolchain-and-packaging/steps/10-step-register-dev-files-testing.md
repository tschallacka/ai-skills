# Verification: 10-step-register-dev-files

## Automated tests

§ 2.1
Stage every new file, then run tests/test-skill-files-manifest.sh and record that no tracked file is unaccounted. Then prove the run was not vacuous: unstage one fixture file, re-run, and confirm the gate no longer mentions it — a gate that reports the same either way is reading a stale tracked set. Re-stage and confirm the account is complete again. This ordering is the point of the step: the same gate passed vacuously in this repository on the day this unit was written, because it was run before git add.