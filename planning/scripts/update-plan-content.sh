#!/usr/bin/env bash
# MODE: PROD
# update-plan-content.sh — edit the numbered prose of every plan document, and
# gate the review approval.
#
# Two jobs live here, and the second one is not obvious from the name:
#
#   1. Content editor. One flag per (document, granularity) pair rewrites a
#      titled paragraph, a whole section, a field, a table, or the decomposition
#      checkbox in plan-description.md, a goal.md, a step file, or
#      adversarial-review.md.
#   2. Approval gate. `--review-status <plan> approved` is the only path that
#      flips a review to approved. It refuses while any finding is still open,
#      verifies the fix keys through verify-fix-keys.sh as `${CLAIMED_BY:-<the
#      minting session>}`, and then destroys the session secret so the same keys
#      cannot be replayed. Export CLAIMED_BY with the fixer session that wrote
#      fixes.md: the default is the minting session, which the fix-key gate
#      refuses as self-certification. See the "Approval gate" banner below; do
#      not move that logic elsewhere.
#
# Usage:
#   update-plan-content.sh <flag> [--plan-dir] <plan-directory> [args…]     (see --help)
#   update-plan-content.sh --help
#
# Sections in order:
#   Usage and flag translation — the outer flag map, which rewrites "$@" into an
#     internal command plus canonical positionals.
#   Paragraph argument parsing — the repeated `-p N.N: content` form.
#   Command dispatch — one arm per internal command, including Approval gate.
#   Context invalidation — the post-mutation handoff marker.
#
# Exit codes: 64 bad invocation, 65 the document is in an unusable state,
# 66 a required plan file is missing.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism and why PLANNING_SKILL_ROOT is exported
# unconditionally.
upc_bin_pref_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$upc_bin_pref_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present update-plan-content "$upc_bin_pref_script_dir" "$@"
unset upc_bin_pref_script_dir

plan_die "update-plan-content: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
