<!-- MODE: PROD -->
# ci-failures

**What actually failed, not the whole log.**

A CI run went red. This skill turns "go read the run" into one command: it
resolves a run/pipeline from an id, a PR/MR number, or a branch, then prints
just the lines that identify each failing job's failure — a panic, a test
suite's own `Failed:` line, a `##[error]` annotation — instead of the whole
scrollback.

## Quick start

> Run's red — what failed?

Behind the scenes:

```sh
scripts/ci-failures.sh pr/47
```

## What you get

- **One command, either forge.** Detects GitHub vs. GitLab from the git
  remote and says which it picked, so the choice is never silent.
- **The failure, not the log.** Panics, test-suite failure summaries, and
  CI-level error annotations, extracted — with `--raw DIR` available when the
  extract comes up empty and the full log is needed.
- **The same target vocabulary on both forges.** A run/pipeline id, a PR/MR
  number, `pr/N`, a branch, or nothing (the current branch).

Read SKILL.md for the forge-detection rules, the run/pipeline disambiguation
heuristic, and what is and is not verified on the GitLab side.
