#!/usr/bin/env bash
# MODE: PROD
# plan-env.sh — write and verify the .env manifests that pin a plan's paths.
#
# Two manifests exist: a global one at <plans-root>/.env naming the plans root
# and the installed planning skill, and a per-plan one at <plan-root>/.env
# naming every document path inside that plan. `check` is a gate: it refuses a
# manifest that is a symlink, is not mode 600, is not owned by the caller, has a
# duplicate/unexpected/missing key, or carries a value with shell metacharacters
# — only then is the manifest safe to source.
#
# Usage:
#   plan-env.sh write-global <plans-root> <planning-root>
#   plan-env.sh write-plan <plan-root> [plans-root] [snapshot-repo]
#   plan-env.sh check <plan-root> [plans-root]
#   plan-env.sh path global|plan <plan-root> [plans-root]
#   plan-env.sh print <plan-root> [plans-root]
#
# Exit codes: 64 bad invocation, 65 malformed manifest, 66 missing path or
# manifest, 69 no usable stat(1).
#
# Not sourced: this file is only ever exec'd, so the bare names `usage`, `die`
# and `absolute_path` cannot shadow a caller's functions. Every other name here
# is prefixed or file-local; keep it that way if this ever becomes a library.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
pe_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$pe_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present plan-env "$pe_script_dir" "$@"
unset pe_script_dir

plan_die "plan-env: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
