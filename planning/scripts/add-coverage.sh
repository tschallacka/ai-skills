#!/usr/bin/env bash
# MODE: PROD
# add-coverage.sh — add (or with --replace, replace) one row in a plan's
# "## Definition-of-done coverage" table, linking a required outcome or proof to
# the work units that deliver it.
#
# The row is inserted immediately above the "## Work units" heading, so coverage
# rows accumulate in the order they were added. --replace collapses every row
# carrying the same outcome into one at the position of the first match, and adds
# the row when no such outcome exists yet.
#
# Usage:
#   add-coverage.sh [--plan-dir] <plan-directory> <required-outcome-or-proof> <WNN[,WNN...]> <notes> [--replace]
#   add-coverage.sh --help

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call (AR-05): that call rewrites a --plan-dir flag into
# a positional argument, and the compiled binary must receive the caller's
# true original argv, not the already-hoisted form.
acov_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$acov_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-coverage "$acov_bin_pref_script_dir" "$@"
unset acov_bin_pref_script_dir

plan_die "add-coverage: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
