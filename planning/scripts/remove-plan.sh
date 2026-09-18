#!/usr/bin/env bash
# MODE: PROD
# remove-plan.sh — remove a plan directory and reconcile the plans-root git
# history.
#
# Usage:
#   remove-plan.sh [--plan-dir] <plan-directory>
#   remove-plan.sh --help
#
# Removes the plan directory (plan-description.md must exist, so a stray path
# is not destroyed). When the enclosing plans root then holds no other plan
# directories, the root's own git history (created by create-plan.sh when the
# root is git-excluded or outside any repo) is cleared so a discarded
# initiative does not leave stale history behind. The root directory itself
# and its .env manifest are preserved; the next create-plan.sh re-initializes
# the history.
#
# Exit codes: 64 bad invocation or not a plan directory, 66 no such directory.

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call below: that call rewrites a --plan-dir flag into a
# positional argument, and the compiled binary must receive the caller's true
# original argv, not the already-hoisted form.
rp_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rp_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present remove-plan "$rp_script_dir" "$@"
unset rp_script_dir

plan_die "remove-plan: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
