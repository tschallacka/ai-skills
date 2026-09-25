#!/usr/bin/env bash
# MODE: PROD
# plan-mutate.sh — the single dispatcher for every durable plan mutation.
#
# Each subcommand exec's the one helper that owns that mutation, so the protocol
# has exactly one entry point and direct edits to .plans stay prohibited. Two
# subcommands are implemented inline instead of dispatched — see the note above
# add_progress_step().
#
# Usage:
#   plan-mutate.sh <subcommand> [arguments...]
#   plan-mutate.sh --help
#
# Exit codes: whatever the dispatched helper returns; 64 for a bad subcommand.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
pm_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pm_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-mutate "$pm_script_dir" "$@"
unset pm_script_dir

plan_die "plan-mutate: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
