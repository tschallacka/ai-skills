#!/usr/bin/env bash
# MODE: DEV
# ci-test-scope.sh — decide which shell tests and crate tests a CI run has to
# execute.
#
# Prints:
#   scope=full|selective
#   reason=<one line saying why>
#   tests=<space-separated repo-relative test paths and crate dirs to run,
#          meaningful only when scope=selective; empty under scope=full,
#          where nothing is filtered and everything runs>
#
# THE DEFAULT IS ALWAYS full: every
# branch that cannot prove a smaller scope correct returns full, including
# every error path. A selector that narrows when it is confused is worse than
# none, because the green tick then means "we did not look" while reading as
# "we looked".
#
# SELECTION. Each test may declare what it covers with a COVERS marker within
# the first few header lines (right after the shebang and the MODE marker), a
# comment line reading:
#
#   COVERS: <path> <path> ...
#
# A changed path "hits" a COVERS entry when it equals the entry or begins with
# "<entry>/" — a directory entry covers everything under it, a file entry
# covers only itself. A test with NO COVERS marker is UNDECLARED, and an
# undeclared test ALWAYS runs: selection only ever narrows a test that opted
# in, on grounds that test itself stated, never a test nobody has annotated
# yet. That is what keeps marking the rest of the suite a pure optimisation
# rather than a hazard — an unmarked test costs nothing in speed but nothing
# in safety either.
#
# The canonical test/crate list comes from `run-tests.sh --list-only`, not a
# second copy of its suites array here: two lists of "what counts as a test"
# drift, and this selector deciding what NOT to run is exactly the place a
# stale list would fail silently.
#
# Usage:
#   ci-test-scope.sh [--base REF] [--files-from FILE]
#   ci-test-scope.sh --push-to BRANCH
#   ci-test-scope.sh --help
#
#   --base REF        what to diff against (default: origin/master, then master)
#   --files-from FILE  read the change set from FILE instead of git; one path
#                      per line. For tests, so every branch is reachable
#                      without inventing commits.
#   --push-to BRANCH  this run is a push to BRANCH, not a pull request: decide
#                     full and stop (a push to master has an empty diff
#                     against itself, and selection is a pull-request
#                     feature).
#
# Exit codes: 0 always, unless usage is wrong (64).

set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed immediately after repo_root is
# computed and BEFORE run_tests=/the rest of this script's own variable
# defaults -- as early as structurally possible, and safe since nothing
# before this point consumes "$@". Matching goal 21's own precedent for
# ci-scope.sh: this script already computes its own repo_root for its own
# real use (the run_tests= line below), so the wiring call reuses that
# existing variable rather than computing a second, differently-named one.
# This script already declares set -uo pipefail above (deliberately WITHOUT
# -e), so it is forced back off immediately below, matching this plan's own
# established fix for that class of caller.
# plan-core-lib.sh is generated (gitignored) by build-plan-libs.sh, so it does
# not exist on a genuinely fresh checkout -- guard the source+exec on it
# already being present, unconditionally falling through to this script's own
# bash implementation when it is not, matching B346's fix for
# build-plan-libs.sh's own self-referential case.
if [ -f "$repo_root/planning/scripts/plan-core-lib.sh" ]; then
    source "$repo_root/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present ci-test-scope "$repo_root" "$@"
fi
set +e
set -uo pipefail

run_tests="$repo_root/run-tests.sh"
base_ref=""
files_from=""
push_to=""

usage() {
    awk 'NR > 1 && /^#/ && !/^# ?(MODE|PACKAGE):/{ sub(/^# ?/, ""); print } /^set -uo/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || usage; base_ref="$2"; shift 2 ;;
        --files-from) [ "$#" -ge 2 ] || usage; files_from="$2"; shift 2 ;;
        --push-to) [ "$#" -ge 2 ] || usage; push_to="$2"; shift 2 ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done

decide() { # <scope> <reason> [tests...]
    local scope="$1" reason="$2"; shift 2
    printf 'scope=%s\n' "$scope"
    printf 'reason=%s\n' "$reason"
    printf 'tests=%s\n' "$*"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        {
            printf 'scope=%s\n' "$scope"
            printf 'reason=%s\n' "$reason"
            printf 'tests=%s\n' "$*"
        } >> "$GITHUB_OUTPUT"
    fi
    exit 0
}

# Reached only once the wiring block above has already fallen through (no
# compiled ci-test-scope binary found) -- no decision logic is left to compute
# one.
decide full "ci-test-scope binary not found; run ./setup-dev-env.sh to build it"
