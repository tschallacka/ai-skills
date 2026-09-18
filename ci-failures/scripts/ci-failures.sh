#!/usr/bin/env bash
# MODE: PROD
# ci-failures - what actually failed in a CI run or pipeline, from a run/
# pipeline id, a PR/MR number, or a branch. Works against GitHub (gh) and
# GitLab (glab), detecting which one this repository's remote calls for.
#
# Usage:
#   ci-failures.sh                  # the newest run/pipeline for the current branch
#   ci-failures.sh 33894205595      # a run/pipeline id
#   ci-failures.sh 47               # a PR/MR number
#   ci-failures.sh pr/47            # a PR/MR number, unambiguously
#   ci-failures.sh fix/some-branch  # the newest run/pipeline for a branch
#   ci-failures.sh <target> --raw DIR  # keep the whole log of each failing job
#   ci-failures.sh <target> --all   # every job, not only the failing ones
#
# It prints, per failing job, the lines that identify the failure: a suite's
# own `Failed:` summary, cargo's `test result:`, every panic with the lines
# after it, GitHub's `##[error]` annotations, and this repository's own FAIL/
# portability findings. `--raw DIR` additionally writes each job's full,
# de-escaped log to DIR so a long screen dump can be read whole.
#
# WHY THIS EXISTS. Reading a failing run by hand is six steps -- list the jobs,
# find the failing ids, fetch each log through the API, allow the escape
# sequences, strip the CR and the ANSI, then search for the interesting lines
# -- and on GitHub it was done eight times in one session before anyone wrote
# it down. None of that is specific to one repository or one forge, which is
# why this is a skill rather than a maintainer note: which API answers while a
# run is still going, that GitHub's raw-log endpoint refuses a body carrying
# colour codes without an explicit flag, that the codes then need stripping
# with a literal ESC because the escape form is a GNU sed extension, and which
# lines in a log actually identify a failure as opposed to merely mentioning
# one.
#
# FORGE DETECTION. The remote this repository's origin points at decides gh or
# glab, not a flag: a github.com remote uses gh, a gitlab.com or self-hosted
# GitLab remote uses glab, and CI_FAILURES_FORGE=gh|glab overrides the guess
# outright for a remote neither pattern recognises. Either way the tool chosen
# is named on the first line of output: a silent choice between two APIs with
# different failure vocabularies is exactly the kind of guess this script
# exists to make instead of a person, and the person reading the output still
# needs to know which guess was made.
#
# BOTH FORGES, ONE VOCABULARY. GitHub calls it a run containing jobs, with a
# logs endpoint; GitLab calls it a pipeline containing jobs, with a trace
# endpoint instead, and its statuses and failure markup are not GitHub's. The
# <target> forms above (a bare number, pr/N, a branch, nothing) mean the same
# thing on both: pr/N addresses a pull request on GitHub and a merge request
# on GitLab, and a bare number is read the same way, disambiguated by
# magnitude (see resolve_run below), on both.
#
# VERIFIED DIFFERENTLY. The gh path is exercised against this repository's own
# real GitHub Actions runs. The glab path is written against GitLab's
# documented REST API v4 (pipelines, jobs, trace) -- the same stable,
# versioned surface the gh path prefers over gh's own formatted subcommands,
# and for the same reason -- but this repository has no GitLab remote to run
# it against, so it is exercised in tests against a stubbed glab rather than a
# live one. Treat a first real run against a GitLab project as the one
# still-missing verification step, not as settled.
set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script takes no --plan-dir and does not
# hoist one, so there is no hoist ordering to preserve; placed immediately
# after both anchor lines above. ci-failures/ is a top-level skill directory,
# not under planning/, so the relative path to plan-core-lib.sh crosses two
# directory levels up from ci-failures/scripts.
cif_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cif_script_dir/../../planning/scripts/plan-core-lib.sh"
plan_exec_compiled_binary_if_present ci-failures "$cif_script_dir" "$@"
unset cif_script_dir

plan_die "ci-failures: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
