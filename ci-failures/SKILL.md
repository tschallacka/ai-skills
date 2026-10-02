---
name: ci-failures
description: Use when a CI run or pipeline for this repository failed or is red, and you need to know which job and which line failed, from a run/pipeline id, a PR/MR number, or a branch. Not for triggering a new run, not for repositories with no CI at all.
---
<!-- MODE: PROD -->

# ci-failures

## Why this exists

Reading a failing run by hand is six steps: list the jobs, find the failing
ids, fetch each job's log through the API, allow the escape sequences the log
carries, strip the CR and the ANSI codes, then search the result for the
lines that actually identify the failure. None of that is specific to one
repository, so it belongs in a script rather than being redone by hand each
time a run goes red.

Run `scripts/ci-failures.sh` instead of reading a CI run by hand.

## How it works

```
scripts/ci-failures.sh                     # the newest run/pipeline for the current branch
scripts/ci-failures.sh 33894205595         # a run/pipeline id
scripts/ci-failures.sh 47                  # a PR/MR number
scripts/ci-failures.sh pr/47               # a PR/MR number, unambiguously
scripts/ci-failures.sh fix/some-branch     # the newest run/pipeline for a branch
scripts/ci-failures.sh <target> --raw DIR  # also write each failing job's full log to DIR
scripts/ci-failures.sh <target> --all      # every job, not only the failing ones
```

It prints, per failing job, the lines that identify the failure: a suite's
own `Failed:` summary, cargo's `test result:`, every panic with the lines
after it, GitHub's `##[error]` annotations, and a repository's own `FAIL`/
`portability:` findings. When nothing in a job's log matches those patterns,
it says so and points at `--raw` rather than staying silent.

A bare number is ambiguous between a run/pipeline id and a PR/MR number, so
it is disambiguated by magnitude: 9 or more digits reads as a run/pipeline
id, fewer reads as a PR/MR number. `pr/N` says which is meant when that guess
is wrong for a given repository. This is a heuristic, stated in the script's
own `--help`, not a guarantee.

## Which forge

The script reads `git remote get-url origin` to decide between GitHub (`gh`)
and GitLab (`glab`): a `github.com` remote uses `gh`, a `gitlab.com` remote
uses `glab`. A self-hosted remote (GitHub Enterprise, a private GitLab
instance) falls back to whichever of `gh`/`glab` is installed and already
authenticated against the repository. `CI_FAILURES_FORGE=gh` or
`CI_FAILURES_FORGE=glab` overrides the guess outright. Whichever forge is
used, the script names it on its first line of output — never a silent
choice, since the two APIs use different words for the same things (a run
vs. a pipeline) and different failure vocabularies.

`CI_FAILURES_REPO` overrides the repository/project the script talks to
(`owner/repo` for GitHub, `group/project` — or nested `group/subgroup/project`
— for GitLab), for when the current directory's origin remote is not the
right one to ask.

## Both forges, one vocabulary, verified differently

The GitHub path (`gh api`, never `gh --jq`, which embeds a second, hidden jq)
is exercised against this repository's own real GitHub Actions runs.

The GitLab path is written against GitLab's documented REST API v4
(`projects/:id/pipelines`, `.../jobs`, `.../jobs/:id/trace`,
`.../merge_requests/:iid`) via `glab api`, rather than `glab`'s own
higher-level subcommands, for the same reason the GitHub path prefers `gh
api` over `gh run view`'s own formatting: a documented, versioned surface
over a CLI's own output shape. There is no GitLab remote available to run it
against live, so it is exercised in this repository's own test suite against
a stubbed `glab` binary rather than a real one. Treat the first real run
against a GitLab project as the one still-missing verification step for that
path, not as settled — read its output with that in mind, and report back
anything that does not match what this document says it should do.

## What it needs

`bash` and `rjq` unconditionally. `gh`, `glab`, or both, depending on which
forge (or forges) this repository's remotes actually point at — see
`requires.tsv`.
