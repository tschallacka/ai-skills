#!/usr/bin/env bash
# MODE: PROD
# add-adversarial-finding.sh — append one row to a plan's adversarial-review.md
# "## Findings" table.
#
# The row is appended after the LAST existing row of the Findings table (the
# section runs from "## Findings" to the next "## " heading or EOF), which is
# the one anchor every other consumer agrees on: create-adversarial-review.sh
# seeds the table, update-adversarial-review.sh rewrites exactly that span, and
# mint-fix-keys.sh / verify-fix-keys.sh / validate-plan.sh all read it bounded
# by "## Verdict". No narrative sentence is used as an anchor.
#
# Naming a work unit gates the finding behind the fix-key mechanism, so the
# keys are re-minted for the plan; without one the cell stays N/A (ungated).
#
# Usage:
#   add-adversarial-finding.sh [--plan-dir] <plan-directory> <AR-NN> <finding> <resolution>
#       [open|in-progress|resolved] [--status <s>] [--work-unit <WNN>]
#   add-adversarial-finding.sh --help

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call (AR-04): that call rewrites a --plan-dir flag into
# a positional argument, and the compiled binary must receive the caller's
# true original argv, not the already-hoisted form.
aaf_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$aaf_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-adversarial-finding "$aaf_bin_pref_script_dir" "$@"
unset aaf_bin_pref_script_dir

plan_die "add-adversarial-finding: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
