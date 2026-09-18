#!/usr/bin/env bash
# MODE: PROD
# mint-fix-keys.sh — derive a per-(finding, work-unit) SHA-256 fix key for
# every gated findings row in adversarial-review.md and record the derived keys
# in fix-keys.json beside the review file.
#
# The secret itself is never written into the plan: it lives in the session
# secret dir under the planning scratch dir (see W12 session lifecycle). The
# plan only ever holds the derived keys plus the session id that names the
# secret dir.
#
# Usage:
#   mint-fix-keys.sh [--plan-dir] <plan-directory>
#   mint-fix-keys.sh --help
#
# Exit codes: 65 = a gated findings row has non-conforming ids; 69 = no SHA-256
 # implementation (the plan-crypt binary, sha256sum or shasum) or no OS random
 # source is available.

set -euo pipefail

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. Placed before this script's own
# plan_hoist_plan_dir call below: that call rewrites a --plan-dir flag into a
# positional argument, and the compiled binary must receive the caller's true
# original argv, not the already-hoisted form. Per AR-11, the compiled binary
# itself was extended to parse --plan-dir/--plan-dir= directly (src/mint-fix-keys/src/main.rs).
mfk_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$mfk_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present mint-fix-keys "$mfk_script_dir" "$@"
unset mfk_script_dir

plan_die "mint-fix-keys: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
