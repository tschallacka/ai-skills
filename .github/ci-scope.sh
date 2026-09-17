#!/usr/bin/env bash
# MODE: DEV
# ci-scope.sh — decide how much of the workspace a CI run has to build.
#
# Prints three lines, and writes the same to $GITHUB_OUTPUT when it is set:
#
#   scope=full|selective|none
#   reason=<one line saying why>
#   crates=<space-separated crate names, empty unless scope=selective>
#
# full       build and test every workspace member on every target
# selective  build and test only the named crates
# none       no crate needs building; the shell and packaging gates still run
#
# THE DEFAULT IS ALWAYS full. Every branch that cannot prove a smaller scope is
# correct returns full, including every error path: no merge base, a shallow
# clone, cargo metadata failing, an unreadable change set. A selector that
# narrows the run when it is confused is worse than no selector, because the
# green tick then means "we did not look" while reading as "we looked".
#
# Selection is crate changes UNIONED WITH THEIR DEPENDENTS. Building only the
# changed crate is wrong: chat-proto has two dependents, planning-core has
# twenty-four, and a library change that compiles in isolation can still break
# every consumer. The reverse edges come from cargo metadata, so the graph is
# the real one rather than a list someone maintains by hand.
#
# Usage:
#   ci-scope.sh [--base REF] [--files-from FILE] [--threshold N]
#   ci-scope.sh --push-to BRANCH
#   ci-scope.sh --help
#
#   --base REF        what to diff against (default: origin/master, then master)
#   --files-from FILE  read the change set from FILE instead of git; one path
#                      per line. For tests, so every branch is reachable
#                      without inventing commits.
#   --threshold N     override the derived threshold (see below)
#   --push-to BRANCH  this run is a push to BRANCH, not a pull request: decide
#                     full and stop. On a push to master, HEAD *is*
#                     origin/master, so the merge base is HEAD and the diff is
#                     empty -- the selector reported `scope=none` and master
#                     went green having compiled nothing. Selection is a pull
#                     request feature; an integration branch stays exhaustive.
#
# THE THRESHOLD IS DERIVED, NOT A CONSTANT. A closure bigger than a quarter of
# the workspace goes full: ceil(members / 4), with a floor of 5 so a small
# workspace does not end up with a threshold of 1.
#
# A fixed number would rot. When this was written the workspace had 78 members
# and the closure sizes were 25, 13, 10, 10, 7, 6, 5, 3, then 2 for eleven more
# libraries and 1 for the 69 leaf crates. Any hand-picked value between 10 and
# 13 behaved identically, and so did anything from 13 to 24 — so the number
# looked meaningful while being arbitrary inside a gap, and would have silently
# changed meaning as crates were added.
#
# The obvious alternative is to find the widest gap in that distribution and
# split there, which is what a person does by eye. It is rejected deliberately:
# the widest gap MOVES. One new crate with a mid-sized closure relocates it, and
# the policy flips with no edit and no announcement. A ratio is monotonic — it
# only ever moves when the workspace size moves, and it moves predictably.
#
# The ratio is a cap on RISK as much as on cost. Pure economics would put the
# crossover much higher, since the fixed overhead of a run is paid either way;
# but a large closure means a broad change, and a broad change is exactly where
# selection is most likely to miss a path cargo cannot see — a shell script, a
# generated library, a packaged file. Capping well below the cost crossover
# buys back that uncertainty.
#
# Exit codes: 0 always, unless usage is wrong (64). A decision is not a failure.

set -uo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed immediately after repo_root is
# computed and BEFORE the rest of this script's own variable defaults -- as
# early as structurally possible, and safe since nothing before this point
# consumes "$@". Unlike ci-subjects.sh (goal 20), this script already
# computes its own repo_root for its own real use (the git/cargo-metadata
# work below), so the wiring call reuses that existing variable rather than
# computing a second, differently-named one. This script already declares
# set -uo pipefail above (deliberately WITHOUT -e), so it is forced back off
# immediately below, matching this plan's own established fix for that class
# of caller.
# plan-core-lib.sh is generated (gitignored) by build-plan-libs.sh, so it does
# not exist on a genuinely fresh checkout -- guard the source+exec on it
# already being present, unconditionally falling through to this script's own
# bash implementation when it is not, matching B346's fix for
# build-plan-libs.sh's own self-referential case.
if [ -f "$repo_root/planning/scripts/plan-core-lib.sh" ]; then
    source "$repo_root/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present ci-scope "$repo_root" "$@"
fi
set +e
set -uo pipefail

base_ref=""
files_from=""
push_to=""
# Empty means derive it from the workspace size once cargo metadata is read.
threshold="${CI_SCOPE_THRESHOLD:-}"
threshold_source="derived"
# ceil(members / CI_SCOPE_DIVISOR), never below CI_SCOPE_FLOOR.
divisor="${CI_SCOPE_DIVISOR:-4}"
floor="${CI_SCOPE_FLOOR:-5}"

usage() {
    awk 'NR > 1 && /^#/ && !/^# ?(MODE|PACKAGE):/{ sub(/^# ?/, ""); print } /^set -uo/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || usage; base_ref="$2"; shift 2 ;;
        --files-from) [ "$#" -ge 2 ] || usage; files_from="$2"; shift 2 ;;
        --push-to) [ "$#" -ge 2 ] || usage; push_to="$2"; shift 2 ;;
        --threshold) [ "$#" -ge 2 ] || usage; threshold="$2"; threshold_source="given"; shift 2 ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
done

# An unusable override is discarded rather than honoured: a threshold of
# "abc" must not become 0 and silently make every run full, nor become huge and
# silently make every run selective. Empty means derive.
case "$threshold" in
    '')       threshold_source="derived" ;;
    *[!0-9]*) threshold=""; threshold_source="derived (ignored an unusable override)" ;;
    *)        [ "$threshold_source" = "given" ] || threshold_source="CI_SCOPE_THRESHOLD" ;;
esac
case "$divisor" in ''|*[!0-9]*|0) divisor=4 ;; esac
case "$floor" in ''|*[!0-9]*) floor=5 ;; esac

decide() { # <scope> <reason> [crates...]
    local scope="$1" reason="$2"; shift 2
    printf 'scope=%s\n' "$scope"
    printf 'reason=%s\n' "$reason"
    printf 'crates=%s\n' "$*"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        {
            printf 'scope=%s\n' "$scope"
            printf 'reason=%s\n' "$reason"
            printf 'crates=%s\n' "$*"
        } >> "$GITHUB_OUTPUT"
    fi
    exit 0
}

# Reached only once the wiring block above has already fallen through (no
# compiled ci-scope binary found) -- no decision logic is left to compute one.
decide full "ci-scope binary not found; run ./setup-dev-env.sh to build it"
