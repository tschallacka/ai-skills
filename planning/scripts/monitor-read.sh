#!/usr/bin/env bash
# MODE: PROD
# monitor-read.sh — gated, bounded, paginated monitor reader (pull-on-exception).
#
# Willie (the end-user-facing monitor, maintainer persona) supervises personas
# by reading ONLY each subagent's bounded supervision frame (written by
# supervision-frame.sh), never raw logs. This reader serves those frames
# bounded and paginated so a green frame costs ~zero context and Willie pulls
# deeper only when a frame flags escalated/out-of-bounds/blocked.
#
# Usage:
#   monitor-read.sh show <frame-file>                      # bounded frame text
#   monitor-read.sh status <frame-file>                    # one-line: subagent,status
#   monitor-read.sh summary <dir>                          # all frames under <dir>
#   monitor-read.sh grants <grant-log> [--last N]          # grant log (case+command)
#   monitor-read.sh verify <frame-file>                    # fail-closed identity
#   monitor-read.sh --help
#
# Gating: Willie is the maintainer. Reading a frame requires ROLE_ID resolving
# to `maintainer`; non-maintainer callers are refused (fail closed). Budget is
# enforced per frame via supervision-frame.sh check.

# NOTE: `usage` below prints lines 1-20 of this file as the help text, so the
# docblock above MUST stay within the first 20 lines (CODE-STYLE.md section 2).
# Anything added here goes below this comment, never into the docblock.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
mr_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$mr_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present monitor-read "$mr_script_dir" "$@"
unset mr_script_dir

plan_die "monitor-read: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
