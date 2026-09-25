#!/usr/bin/env bash
# MODE: PROD
# create-plan-progress.sh — generate a plan's top-level progress.md: one row per
# goal directory holding a goal.md, each carrying that goal's definition of done.
#
# Row order is the goal directories in byte order (export LC_ALL=C above, so the
# generated order is the same for every developer on every locale). Refuses to
# overwrite an existing tracker (73); rebuild-plan-progress.sh refreshes one.
#
# Usage:
#   create-plan-progress.sh [--plan-dir] <plan-directory>
#   create-plan-progress.sh --help

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
cpp_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$cpp_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-plan-progress "$cpp_script_dir" "$@"
unset cpp_script_dir

plan_die "create-plan-progress: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
