#!/usr/bin/env bash
# MODE: PROD
# remove-coverage.sh — remove one row from a plan's "## Definition-of-done
# coverage" table by its required-outcome cell. The sanctioned undo for
# add-coverage.sh (T17): an obsolete row whose work units still exist had no
# removal path short of the hand edit SKILL.md forbids.
#
# Per contract 9a the removal names what it discarded: each dropped row is
# printed on stderr after it is gone, with the work units it carried.
#
# Usage:
#   remove-coverage.sh [--plan-dir] <plan-directory> <required-outcome-or-proof>
#   remove-coverage.sh --help
#
# Exit codes: 64 bad invocation, 65 inventory or coverage section damaged,
# 66 plan directory, inventory, or matching row not found.

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
rcov_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rcov_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present remove-coverage "$rcov_script_dir" "$@"
unset rcov_script_dir

plan_die "remove-coverage: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
