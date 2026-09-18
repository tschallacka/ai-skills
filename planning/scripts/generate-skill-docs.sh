#!/usr/bin/env bash
# MODE: DEV
# generate-skill-docs.sh — build SKILL.md and its parts from skill-source.txt.
#
# A maintainer-only tool, like skill-source.txt itself: an installed skill
# already carries the generated SKILL.md/parts it needs to be USED, and
# regenerating them is only ever done by someone editing this repository.
#
# planning/skill-source.txt is the authored content; SKILL.md and
# planning/parts/part-N.md are GENERATED from it, the way install.sh is
# generated from installer/src/ — same reason: a file over roughly 25,000
# tokens is silently truncated by at least one harness this repo runs under
# (see .agents/knowledge/agent-read-limits.md), and planning/SKILL.md alone
# was well past that (T87).
#
# skill-source.txt marks each part with a comment pair:
#   <!-- SKILL_SECTION:START <name> targets=<target,...> -->
#   ...
#   <!-- SKILL_SECTION:END <name> -->
# Every byte between them is copied verbatim into every listed target. A
# target may be named by more than one section (they concatenate, in source
# order), and a section may list more than one target — neither is used by
# planning today, but the mechanism does not assume a single 1:1 split.
#
# Generated files stay COMMITTED and SHIPPED: unlike install.sh, a skill has
# no build step at the point it is consumed, so there is nothing to run this
# at. Use --check to catch a source edit that was not followed by a rebuild.
#
# T86: every part also gets a load-sanity line — see plant_load_proof below —
# at a random position, planted in the same pass so the split and the load
# check are one generator rather than two retrofitted onto each other.
#
# generate-reviewer.sh is a separate, narrower generator over the same
# source (REVIEWER.md is not committed, unlike these targets, and its own
# REVIEWER_SECTION markers serve a different purpose: an excerpt for review,
# not a readable part). This script does not produce REVIEWER.md.
#
# Usage:
#   generate-skill-docs.sh [--check] [<skill-directory>]
#   generate-skill-docs.sh --help
#
# Exit codes: 64 = bad usage; 65 = a listed part had no matching section, or a
# section is empty; 66 = skill-source.txt is missing; 69 = no SHA-256
# implementation (the plan-crypt binary, sha256sum or shasum) for the T86
# token, which is derived from each part's own content — see plant_load_proof.
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
gsd_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$gsd_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present generate-skill-docs "$gsd_script_dir" "$@"
unset gsd_script_dir

plan_die "generate-skill-docs: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
