#!/usr/bin/env bash
# MODE: PROD
# generate-postmortem.sh — thin oracle for the generate-postmortem Rust
# binary. There is no bash predecessor to preserve parity with (this tool
# was Rust-first from inception, per T145's own policy that new planning
# tooling is never new bash), so this file carries no reimplementation body:
# it either execs the compiled binary or fails clearly.
#
# Usage:
#   generate-postmortem.sh <plan-directory> [--output PATH] [--help]
#
# Exit codes: 66 if the compiled binary is not staged (run ./setup-dev-env.sh
# first); otherwise whatever the compiled binary itself exits with.

set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
gpm_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$gpm_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present generate-postmortem "$gpm_script_dir" "$@"
unset gpm_script_dir

printf 'generate-postmortem: compiled binary not found; run ./setup-dev-env.sh\n' >&2
exit 66
