# Verification: 06-step-bind-reviewer-session

## Automated tests

Run test-review-lifecycle.sh with wrong-session, wrong-mode, stale, duplicate, and cross-capsule approval fixtures; assert only the exact current Reviewer B session passes and every rejection emits a deterministic non-terminal state reason.
