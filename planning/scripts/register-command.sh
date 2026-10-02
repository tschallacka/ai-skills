#!/usr/bin/env bash
# MODE: PROD
# register-command.sh — maintain a plan's command registry (commands.json).
#
# Every command literal in step instructions or testing companions must be
# registered here with its "when" context, so executors know when the command
# is appropriate and future plans do not copy it out of context. The registry
# is seeded empty by create-plan.sh; validate-plan.sh flags any unregistered
# command literal it finds.
#
# --list emits TSV (key, command, when) so a caller can parse it (§10).
#
# Usage:
#   register-command.sh [--plan-dir] <plan-directory> <key> <command> <when>
#   register-command.sh [--plan-dir] <plan-directory> --remove <key>
#   register-command.sh [--plan-dir] <plan-directory> --list
#   register-command.sh --help
#
# Requires rjq.
#
# Exit codes: 64 bad invocation, 66 no plan directory, 69 rjq unavailable.

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# Exec into the compiled binary when one is present, falling through to the
# bash implementation otherwise. Placed before this script's own
# plan_hoist_plan_dir call below: that call rewrites a --plan-dir flag into a
# positional argument, and the compiled binary must receive the caller's true
# original argv, not the already-hoisted form.
rc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present register-command "$rc_script_dir" "$@"
unset rc_script_dir

plan_die "register-command: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
