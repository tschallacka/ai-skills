#!/usr/bin/env bash
# MODE: PROD
# verify-fix-keys.sh — check every claim in fixes.md against the derived fix
# keys in fix-keys.json. Run by the approval gate before a gated review is
# marked approved; also runnable standalone.
#
# Usage:
#   The plan directory may be given positionally or as --plan-dir <path>.
#   verify-fix-keys.sh [--plan-dir] <plan-directory> [--claimed-by <id>]
#   verify-fix-keys.sh --help
#
# A plan without fix-keys.json is ungated and passes without verification.
# A gated plan must hold, per gated (finding, work-unit) pair, a fixes.md claim
# line with exactly three tab-separated fields (finding id, work unit, key) and
# the key must match SHA-256 over (secret)(session_id|finding|work unit),
# secret first. Keys minted under the retired HMAC scheme do not verify.
# Mismatched or unclaimed pairs fail; well-formed claims for pairs that are not
# gated are ignored with a warning.
#
# Optional --claimed-by <id> names the session that recorded the claims; when it
# equals the session recorded as minted_by in fix-keys.json, the run FAILS: a
# fixer that minted and then claimed its own keys is self-certifying, which the
# gate refuses rather than reports. The approval gate always passes the flag.
# A harness whose roles share one derived session id (subagent reviewers under
# a coordinator) mints with MINTED_BY=<reviewer identity>, so the recorded
# minter is the role and honest claims never collide with it.

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
vfk_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$vfk_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present verify-fix-keys "$vfk_script_dir" "$@"
unset vfk_script_dir

plan_die "verify-fix-keys: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
