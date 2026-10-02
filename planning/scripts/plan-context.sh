#!/usr/bin/env bash
# MODE: PROD
# plan-context.sh — bounded reader and freshness gate for one plan's documents.
#
# Owns the plan-context cache: `init` snapshots every plan document with its
# hash, `read` returns one PAGE of a document view under byte and record
# budgets, `check` reports which snapshotted documents drifted, `refresh`
# re-snapshots, and `checkpoint` records a phase state. Budgets are accounted in
# BYTES and bound each page, not the document.
#
# A page that withholds records returns `next_token`; feeding it back through
# `--token` resumes at the next record. The token carries the document's
# SHA-256 and the view it was minted for, so a token replayed against changed
# content is refused (65) instead of resuming into shifted records.
#
# Usage:
#   plan-context.sh init|read|check|refresh|checkpoint --plan-dir DIR [...]
#   plan-context.sh --help
#
# Exit codes: 2 bad invocation, 64 refused by the ROLE_ID reader allow-list,
# 65 stale --token (document or view no longer matches), 66 plan directory or
# document missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
pctx_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pctx_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-context "$pctx_script_dir" "$@"
unset pctx_script_dir

plan_die "plan-context: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
