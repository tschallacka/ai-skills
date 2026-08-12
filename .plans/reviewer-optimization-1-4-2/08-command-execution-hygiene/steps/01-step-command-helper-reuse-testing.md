# Verification

Run a representative long monitoring command twice through a temporary executable helper. Pass only when the helper:

- exists under the run-specific `/tmp` scope and is executable;
- accepts explicit run/result/poll arguments with safe quoting;
- produces bounded, concise output for each invocation;
- performs only the documented read-only inspection or other explicitly scoped action;
- has its path, arguments, output limit, and cleanup status recorded in evidence; and
- is absent from the repository and published result archive after cleanup when it is run-specific.

If no repeated long command occurs in a run, record `not needed` with the reason and verify that no temporary helper was left behind.
