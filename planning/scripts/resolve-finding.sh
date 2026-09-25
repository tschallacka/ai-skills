#!/usr/bin/env bash
# MODE: PROD
# resolve-finding.sh — close one adversarial-review finding: set its status,
# record its fix claim, and refuse a claim the minting session is making about
# its own keys.
#
# The three steps were previously three commands, and doing them by hand across
# eight review cycles produced two defects that the gate could not see (T54):
#
#   * A whole cycle's findings were reasoned about, remediated and cited by id
#     in six work units while never becoming rows, because the status flip was
#     done by regenerating the findings CSV *from* the table. Nine findings that
#     no row carried could not be minted, claimed, or reported as unclaimed —
#     verify-fix-keys iterates the pairs it finds, so it printed passed.
#   * Re-gating a finding onto a different unit left the old claim behind, and
#     verify-fix-keys reported four such rows as ignored rather than verified,
#     so fixes.md no longer said unambiguously which unit resolved them.
#
# Both are impossible through this command: it edits the row that exists rather
# than rewriting the table, and it removes a superseded claim for the same
# finding before recording the new one.
#
# Usage:
#   resolve-finding.sh [--plan-dir] <plan-directory> <AR-NN> [--status STATUS]
#                      [--claimed-by ID]
#   resolve-finding.sh --help
#
# The key comes from the plan's own fix-keys.json; the finding must already be
# gated on a work unit, because an ungated finding has no key to claim.
#
# Exit codes: 64 bad invocation, 65 the finding is absent or ungated,
# 66 the plan or its review file is missing, 70 self-certification refused.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script's own plan_hoist_plan_dir call
# runs after both of the anchor lines above, not between them, so the
# standard after-both-anchors placement is already correct here too.
rf_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rf_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present resolve-finding "$rf_script_dir" "$@"
unset rf_script_dir

plan_die "resolve-finding: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
