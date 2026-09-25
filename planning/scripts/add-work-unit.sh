#!/usr/bin/env bash
# MODE: PROD
# add-work-unit.sh — add one work unit to a plan: an inventory row, its atomic
# step file, and its `§ 9.N` entry in the owning goal's "Owned work units".
#
# All three land together or none does: every input is validated before the
# first write, and the inventory row, the step file and the goal edit are staged
# in temp files that a single EXIT trap removes.
#
# Usage:
#   add-work-unit.sh [--plan-dir] <plan-directory> [--repo-root DIR] --id <WNN> --type <type> --file <path|N/A> \
#       --scope <scope> --subscope <subscope|N/A> --change <intended change> \
#       --depends-on <WNN,…|—> --goal <NN-name> --step <NN-step-name>
#   add-work-unit.sh --help

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call (AR-08): that call rewrites a --plan-dir flag into
# a positional argument, and the compiled binary must receive the caller's
# true original argv, not the already-hoisted form.
awu_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$awu_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present add-work-unit "$awu_bin_pref_script_dir" "$@"
unset awu_bin_pref_script_dir

plan_die "add-work-unit: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
