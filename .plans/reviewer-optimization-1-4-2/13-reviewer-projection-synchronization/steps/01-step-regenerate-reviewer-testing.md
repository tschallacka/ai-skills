# Verification: 01-step-regenerate-reviewer

## Automated tests

- Run `planning/scripts/generate-reviewer.sh`.
- Compare the recorded projection hash with `sha256sum planning/SKILL.md`.
- Run `planning/scripts/validate-plan.sh .plans/reviewer-optimization-1-4-2`.
