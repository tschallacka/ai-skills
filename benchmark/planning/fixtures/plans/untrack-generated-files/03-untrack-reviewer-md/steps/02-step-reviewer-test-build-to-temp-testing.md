# Verification: 02-step-reviewer-test-build-to-temp

## Automated tests

§ 2.1
Run planning/tests/test-reviewer-projection.sh with REVIEWER.md removed from the tree (pass); perturb the SKILL.md reviewer section and expect the SHA assertion to fail; two consecutive fresh runs are byte-identical.