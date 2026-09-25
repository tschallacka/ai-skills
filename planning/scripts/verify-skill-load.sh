#!/usr/bin/env bash
# MODE: PROD
# verify-skill-load.sh — T86: the command that decides whether a part of the
# planning skill was actually read, not merely reported as read.
#
# Every generated planning/parts/part-N.md carries a load-sanity line at a
# position generate-skill-docs.sh chose from the part's own content, in its
# last fifth: `<!-- SKILL-LOAD-PROOF part=<N> token=<hex> -->`. This command
# reads that SAME file itself, from disk, and compares its OWN reading of the
# current token against the one the caller supplies. An instruction to
# report a token is claimable without having read that far; an argument
# this command independently verifies against the real file is not — that
# distinction is the whole reason the check is a command and not a line of
# prose asking the agent to say a number.
#
# A missing or mismatched token means one of: the part was not read that
# far (a harness silently truncated it — see
# .agents/knowledge/agent-read-limits.md), a stale token was recalled from a
# previous, since-regenerated version of the part, or the part name is wrong.
# Re-read the current part and try again with what it actually says.
#
# Usage:
#   verify-skill-load.sh --part <name> --token <hex> [<skill-directory>]
#   verify-skill-load.sh --help
#
# Exit codes: 64 = bad usage; 65 = the part carries no load-proof line at all
# (a stale generation, or generate-skill-docs.sh has not been run since the
# part was added); 66 = no such part file; 1 = the token does not match.
set -euo pipefail
export LC_ALL=C

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism. This script takes no --plan-dir and does not
# hoist one, so there is no hoist ordering to preserve; placed immediately
# after both anchor lines above.
vsl_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$vsl_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present verify-skill-load "$vsl_script_dir" "$@"
unset vsl_script_dir

plan_die "verify-skill-load: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
