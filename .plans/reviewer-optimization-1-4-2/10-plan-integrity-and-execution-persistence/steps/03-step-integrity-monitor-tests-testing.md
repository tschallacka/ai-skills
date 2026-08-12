# Verification

Run `planning/tests/test-plan-integrity-and-monitor.sh` with its isolated temporary fixtures. Pass only when all mutation, continuation, bounded-retry, terminal-state, and interruption assertions pass and the test leaves no fixture artifacts in the repository or published result archive.
