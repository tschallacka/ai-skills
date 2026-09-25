#!/usr/bin/env bash
# MODE: DEV
# blast-radius.sh — report what a change set touches beyond the files it edits.
#
# This is an integration-safety report, not a correctness check. It catches the
# mistakes that come from a file's couplings: a generated artifact left stale, a
# new registry that never ships, a branch whose base moved under it. It will not
# find a logic bug, and it is not a substitute for exercising the changed path.
#
# Four passes:
#   1. freshness  — generated artifacts whose sources moved (fails)
#   2. registry   — a new file under planning/ with no manifest row (fails for a
#                   runtime registry, warns for anything else)
#   3. drift      — commits that touched these files since the base ref, which
#                   is how a stale branch silently reverts someone else's fix
#   4. contracts  — couplings recorded in coupling.tsv that a human must honour
#
# Usage:
#   blast-radius.sh [--base <ref>] [<path> ...]
#   blast-radius.sh --help
#
# With no paths, the working tree's own changes are used (staged, unstaged and
# untracked). --base defaults to master and only affects the drift pass.
#
# Exit codes: 0 clean, 1 findings, 64 bad invocation, 69 not a git work tree.

set -euo pipefail
export LC_ALL=C

case "${1:-}" in
    -h|--help)
        awk 'NR == 1 { next }
             /^#/ {
                 sub(/^#[[:space:]]?/, "")
                 if ($0 ~ /^(MODE|PACKAGE):/) next
                 if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
                 print; next
             }
             { exit }' "$0"
        exit 0
        ;;
esac

# PORTABILITY(empty-array-setu): saved here, before the arg-parsing loop
# below consumes "$@" via shift, so the compiled-binary wiring further down
# (necessarily placed after repo_root is computed, which this script only
# resolves AFTER parsing args) can still exec with the caller's own argv
# exactly as received rather than an already-emptied "$@". Guarded the same
# way `paths` is below: an empty array's [@] expansion is an
# unbound-variable error under bash 3.2's own set -u semantics.
br_original_args=("$@")

base=master
paths=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || { printf '%s: --base needs a ref\n' "${0##*/}" >&2; exit 64; }
                base="$2"; shift 2 ;;
        --base=*) base="${1#--base=}"; shift ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
        *) paths+=("$1"); shift ;;
    esac
done

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    printf '%s: not a git work tree\n' "${0##*/}" >&2; exit 69
}

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed immediately after repo_root is
# computed and BEFORE the `cd "$repo_root"` below -- matching the placement
# already used in pre-push-check.sh, run-tests.sh, and setup-dev-env.sh
# (wiring block immediately after repo_root is computed, strictly before any
# subsequent cd), not merely "before registry is set" (which the cd below
# would also satisfy). This script already declares set -euo pipefail above,
# matching what sourcing plan-core-lib.sh itself wants, so no call-site
# set +e fix is needed here. blast-radius.sh lives at the repository root
# itself, one level shallower than planning/scripts, so the relative path to
# plan-core-lib.sh crosses one directory level down, matching
# generate-portability.sh/pre-push-check.sh/setup-dev-env.sh's own precedent.
#
# Forwards br_original_args, NOT "$@": unlike every other already-wired
# script, this one parses its own args (consuming "$@" via shift) BEFORE
# computing repo_root, so by this point "$@" is empty and would silently
# strip every argument from the compiled binary's own invocation.
br_script_dir="$repo_root"
# plan-core-lib.sh is generated (gitignored) by build-plan-libs.sh, so it does
# not exist on a genuinely fresh checkout -- guard the source+exec on it
# already being present, unconditionally falling through to this script's own
# bash implementation when it is not, matching B346's fix for
# build-plan-libs.sh's own self-referential case.
if [ -f "$br_script_dir/planning/scripts/plan-core-lib.sh" ]; then
    source "$br_script_dir/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present blast-radius "$br_script_dir" \
        ${br_original_args[@]+"${br_original_args[@]}"}
fi
unset br_script_dir br_original_args

printf '%s: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it\n' "blast-radius" >&2
exit 69
