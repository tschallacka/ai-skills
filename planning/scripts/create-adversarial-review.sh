#!/usr/bin/env bash
# MODE: PROD
# create-adversarial-review.sh — seed a plan's adversarial-review.md with the
# Review scope, the 5-column Findings table, and a pending Verdict.
#
# The seeded AR-01 row is the empty state of the table, and the Findings table
# itself is the insertion anchor every consumer agrees on: add-adversarial-finding.sh
# appends after its last row, update-adversarial-review.sh rewrites the span from
# "## Findings" to "## Verdict", and mint-fix-keys.sh reads the same span.
#
# Usage:
#   create-adversarial-review.sh [--plan-dir] <plan-directory>
#   create-adversarial-review.sh --help

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
car_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$car_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present create-adversarial-review "$car_script_dir" "$@"
unset car_script_dir

plan_die "create-adversarial-review: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
