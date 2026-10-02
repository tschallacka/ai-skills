#!/usr/bin/env bash
# MODE: PROD
# supervision-frame.sh — bounded supervision-frame emitter + grant log.
#
# Every subagent under Willie's supervision ends by writing one bounded
# "supervision frame" (fixed shape, strict byte budget, footer-overwriting the
# previous frame) instead of pushing its raw log. Willie reads only the latest
# frame and pulls deeper only on exception. A frame must
# stay within FRAME_BUDGET bytes (default 2048); `write` refuses an over-budget
# frame and `check` enforces the same bound for the reader.
#
# Usage:
#   supervision-frame.sh write <frame-file> --subagent NAME --persona ID \
#       --status ok|blocked|escalated|out-of-bounds [--read-discipline ok|violated] \
#       [--wholesale-reads N] [--skill-loaded none|NAME] [--needs-escalation none|CASE] \
#       [--grant-requested none|COMMAND] [--verdict TEXT]
#   supervision-frame.sh grant <grant-log-file> <subagent> <persona> \
#       --case TEXT --command TEXT     # appends case + command, NEVER reasoning
#   supervision-frame.sh show  <frame-file>           # print the latest frame
#   supervision-frame.sh check <frame-file> <budget>  # exit 64 if over budget
#   supervision-frame.sh --help

# NOTE: `usage` below prints lines 1-20 of this file as the help text, so the
# docblock above MUST stay within the first 20 lines. Anything added here
# goes below this comment, never into the docblock.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# Exec into the compiled binary when one is present, falling through to the
# bash implementation otherwise.
sf_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$sf_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present supervision-frame "$sf_script_dir" "$@"
unset sf_script_dir

plan_die "supervision-frame: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
