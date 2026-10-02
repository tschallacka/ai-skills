#!/usr/bin/env bash
# MODE: PROD
# register-read.sh — the read side of the two registers: show one entry, list a
# filtered set, report what is open or what changed, count, or name the next id.
#
# The writers went through helpers from the start; reading did not, so every
# caller wrote its own rjq at the call site and every caller could get it wrong.
# Two did in one session (T60): reading .todos instead of .tasks reported a
# register as nearly empty while an open task sat in the other array, and a
# hand-rolled next-id crashed on the existing id T1e, which reg_next_id has
# always handled. rjq stays the reading language (T41); it is written once here,
# where a test can hold it, instead of once per caller.
#
# Usage:
#   register-read.sh <bug|todo> show <ID>
#   register-read.sh <bug|todo> list [--status S] [--priority P] [--surface TEXT] [--parent ID]
#   register-read.sh <bug|todo> report [--since ISO8601]
#   register-read.sh <bug|todo> count [--status S]
#   register-read.sh <bug|todo> next-id
#   register-read.sh --help
#
# The register file comes from BUGS_JSON / TODO_JSON, or --file PATH.
#
# Exit codes: 64 bad invocation, 66 register missing, 69 rjq missing, 1 no match
# for show.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
rr_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rr_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present register-read "$rr_script_dir" "$@"
unset rr_script_dir

plan_die "register-read: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
