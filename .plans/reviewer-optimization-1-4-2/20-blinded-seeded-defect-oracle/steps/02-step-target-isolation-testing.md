# Verification: 02-step-target-isolation

## Automated tests

- Run an isolated fixture with a sentinel key and map, then inspect target
  environment, arguments, capsule, access audit, and logs for exposure.
- Verify the oracle role cannot be the same session identity as the seeder or
  target reviewer and that interrupted targets produce a blocker.
