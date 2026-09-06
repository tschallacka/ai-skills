# Verification: 01-step-ignore-bin-dir

## Automated tests

§ 2.1
Assert git ls-files planning/bin is empty; git check-ignore -v names the new rule for the triple path; the stale tracked-artifact comment is gone from .gitignore.