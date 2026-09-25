#!/usr/bin/env bash
# MODE: DEV
# verify-both-shells.sh — run the suite on the working tree under both shells.
#
# Verifies the WORKING TREE, not HEAD, in a linked worktree away from the repo, so
# editing can continue here while it runs. Both legs matter: the local bash, and
# the bash 3.2 floor CODE-STYLE.md section 1 declares, because stock macOS ships
# 3.2 and a bash-4 construct is invisible under a newer shell.
#
# Usage:
#   ./verify-both-shells.sh            # both legs
#   ./verify-both-shells.sh --keep     # keep the logs and the worktree on failure
#   ./verify-both-shells.sh --help
#
# A linked worktree rather than a clone: it shares the object store, so every ref
# is present. blast-radius.sh resolves a merge base against master, and a clone
# has only origin/master -- which made an earlier version of this report a failure
# of itself as a failure of the code. --detach because the branch is checked out
# in the main repo, and it lives in TMPDIR, never under the repo: a worktree
# inside it gets picked up by the filesystem scans and lands machine-specific
# paths in generated artifacts, which is how PORTABILITY.md was once polluted.
#
# Never rewrite this file while a run is in flight: bash reads its source
# incrementally, so a mid-run edit makes it execute a fragment of the new text
# as commands -- the "been: command not found" of B17 came from exactly that.
set -uo pipefail

src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed immediately after src is computed and
# BEFORE the case "${1:-}" argument-parsing block -- as early as structurally
# possible, letting even --help reach the compiled binary when present
# (unlike blast-radius.sh in goal 18, where --help was structurally
# unreachable through the compiled binary; this script's own -h|--help exit
# sits AFTER this point, so there is no such obstruction here). This script
# already declares set -uo pipefail above (deliberately WITHOUT -e, so both
# shell legs and the overlay loop tolerate individual command failures
# without aborting the whole harness); sourcing plan-core-lib.sh would
# otherwise silently add -e back on the fall-through path, so it is forced
# back off immediately below. verify-both-shells.sh lives at the repository
# root itself, one level shallower than planning/scripts, so the relative
# path to plan-core-lib.sh crosses one directory level down, matching every
# prior goal's own precedent. Nothing before this point consumes "$@" via
# shift, so it is safe to forward unmodified.
vb_script_dir="$src"
# plan-core-lib.sh is generated (gitignored) by build-plan-libs.sh, so it does
# not exist on a genuinely fresh checkout that has never bootstrapped -- guard
# the source+exec on it already being present, unconditionally falling
# through to this script's own bash implementation when it is not, matching
# B346's fix for build-plan-libs.sh's own self-referential case.
if [ -f "$vb_script_dir/planning/scripts/plan-core-lib.sh" ]; then
    source "$vb_script_dir/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present verify-both-shells "$vb_script_dir" "$@"
fi
unset vb_script_dir
set +e
set -uo pipefail

printf '%s: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it\n' "verify-both-shells" >&2
exit 69
