#!/usr/bin/env bash
# MODE: DEV
# Deterministic, repeatable test runner for the whole repo.
#
# Runs every test under tests/, planning/tests/ and benchmark/planning/tests/ in a
# fixed sorted order, each under the resource-limited wrapper, and reports a
# stable summary. The order and per-test result are the same on every run on
# the same host, so CI or a maintainer sees identical output.
#
# Usage: run-tests.sh [--verbose] [--select-file FILE] [--shard I/N]
#        run-tests.sh --list-only
#
# --list-only        print the discovered shell tests and crates, one
#                    repo-relative path per line, sorted, then exit -- no lock,
#                    no bootstrap, nothing run. This is the canonical list
#                    .github/ci-test-scope.sh (T116) reads rather than keeping
#                    a second copy of the suites array that could drift.
# --select-file FILE  FILE holds one repo-relative test path or crate dir per
#                    line (the same shape --list-only prints); only listed
#                    items run, everything else is skipped. Absent: every
#                    discovered item runs, exactly today's behaviour --
#                    selection is opt-in, never a silent default.
# --shard I/N        run only item I of every N, 0-indexed, chosen by
#                    deterministic position in the SORTED, post-selection work
#                    list (shell tests then crates) -- not per-suite, so one
#                    shard is never planning/tests' 84% of the work while
#                    another idles. The same I/N against the same inputs picks
#                    the same items every time, so a CI failure on shard 2/4
#                    reproduces locally with the identical flag. Absent: every
#                    item runs, exactly today's behaviour.
#
# One run at a time, machine-wide: the runner holds /tmp/ai-skills-run-tests.lock
# and refuses to start (exit 75) while another run really holds it. A lock whose
# recorded pid is gone -- or belongs to something that is not a suite run -- is
# stale and gets reused, so a killed run never wedges the next one.
# AI_SKILLS_ALLOW_CONCURRENT=1 bypasses it. --list-only takes no lock at all --
# it runs nothing, so it cannot collide with anything the lock protects.
#
# --shard is a CI-only concept: each CI runner is its own machine, so N shards
# is N machines each holding their own lock. Locally the lock still serialises
# a whole (possibly --select-file'd or --shard'd) run against any other, which
# is correct -- see the one-verification-at-a-time hazard where a second
# concurrent run deletes the first's worktree.
#
# A failing test's full output is always printed — it is the only diagnostic the
# runner has, and truncating it to the last 20 lines hid the failing assertion.
# `--verbose` additionally prints the output of tests that passed.
#
# `set -uo pipefail` deliberately omits `-e` (the sanctioned exception in
# CODE-STYLE.md §2): a failing test must not abort the loop before the summary
# is printed. Each test's status is captured explicitly instead.
#
# Suite paths are resolved against the repo root, so the runner works from any
# working directory.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script takes no --plan-dir and does not
# hoist one, so there is no hoist ordering to preserve; placed immediately
# after repo_root is computed (the only genuine prerequisite: the relative
# path to plan-core-lib.sh needs it) and before wrapper is resolved, before
# the AI_SKILLS_RESOURCE_LIMIT case statement, and before anything else this
# script does -- as early as structurally possible, matching pre-push-check.sh's
# own goal-14 precedent exactly, since the compiled binary re-derives the
# resource-limit/wrapper selection itself and needs nothing bash would
# otherwise compute first. run-tests.sh lives at the repository root itself,
# one level shallower than ci-failures/scripts, so the relative path to
# plan-core-lib.sh crosses one directory level down, not the two
# ci-failures.sh crosses upward.
#
# RUN_TESTS_BASH exports the bash interpreter THIS invocation is actually
# running under (bash's own $BASH), so the compiled binary can propagate the
# same interpreter into every child test process exactly as run_one's own
# "$BASH" "$t" invocation does -- required for bash32-run-tests (which
# re-execs this very script under a specific bash 3.2 binary) to keep testing
# every individual script under bash 3.2, not only the top-level runner.
RUN_TESTS_BASH="$BASH"
export RUN_TESTS_BASH
rt_script_dir="$repo_root"
source "$rt_script_dir/planning/scripts/plan-core-lib.sh"
plan_exec_compiled_binary_if_present run-tests "$rt_script_dir" "$@"
unset rt_script_dir

printf 'run-tests: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it\n' >&2
exit 69
