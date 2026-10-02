#!/usr/bin/env bash
# MODE: PROD
# role-context.sh — role-gated context reader (persona registry + scope docs).
#
# Given a role id or canonical name, print the .md documents that single role
# needs, concatenated with provenance headers and the role's voice preamble, so
# an agent gets a scoped payload instead of loading unrelated knowledge.
#
# Usage:
#   role-context.sh <role-id|name> [-p N|--page N] [--page-size BYTES]
#   role-context.sh --list                    # -l; identity-free (safe mode)
#   ROLE_ID=maintainer role-context.sh --paths <role-id|name>  # maintainer-only
#   role-context.sh --help
#
# Output is BYTE-budgeted and paginated: -p2 (or -p 2) prints the next page when
# a "more: ..." footer is shown; --page-size sets the per-page byte budget
# (default 12000). Every page is a deterministic slice; no TTY is needed.
#
# GATING: identity-aware and FAILS CLOSED. Any content read requires a ROLE_ID
# resolving to a registered persona; reads are restricted to the caller's own
# role (reviewer family mutual, maintainer may read all). --list is open.

set -euo pipefail
# LC_ALL=C also pins the page accounting to BYTES: under a UTF-8 locale ${#str}
# counts characters, which mis-bills the byte budget below for the multi-byte
# glyphs (§ 💤 ⏳ ✅ —) these documents are full of. Bytes everywhere, one unit.
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
rlc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rlc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present role-context "$rlc_script_dir" "$@"
unset rlc_script_dir

plan_die "role-context: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
