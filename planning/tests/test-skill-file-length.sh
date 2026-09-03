#!/usr/bin/env bash
# MODE: DEV
# test-skill-file-length — a skill a reading agent cannot finish is ratcheted.
#
# Usage: test-skill-file-length.sh
#
# Claude Code returns a PREFIX of a file at roughly 25,000 tokens and says
# nothing: no notice in the tool result, no notice anywhere. A skill past that
# is partly read and reads as fully read, and the agent cannot tell you — one
# asked why content was missing answered that it had "declined to load the rest,
# since it's repetitive filler" when a single unbounded Read had handed it 1,078
# of 1,502 lines. Measured 2026-09-03 across three harnesses; the numbers and
# the method are in .agents/knowledge/agent-read-limits.md.
#
# BYTES ARE THE PROXY, and the budget is deliberately below the observed cut.
# planning/SKILL.md is 89,860 bytes and 2 of 3 real runs received 977 of its
# 1,502 lines, so the cut fell near 58 KB for that prose-and-code mix — about
# 2.3 bytes per token. Denser content tokenises worse and would cut sooner, so
# the budget assumes 2 bytes per token: 25,000 tokens, 50,000 bytes.
#
# Ratcheted, not a hard cap, for the same reason as the function-length ratchet:
# planning/SKILL.md is over TODAY and splitting it is T87. A hard cap would
# block every commit until that lands. The COUNT of over-budget skills may
# shrink and never grow, so a new one fails here at the moment it is added.
# When T87 lands, lower CAP to 0 in the same commit. Never raise it.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

BUDGET=50000
CAP=1

note_fail() { printf 'skill-file-length: %s\n' "$1" >&2; t_record "$1"; }

# benchmark/results is archived agent output, not a live skill, and every other
# scan in this suite excludes it for the same reason.
over=0
listing=""
while IFS= read -r f; do
    [ -n "$f" ] || continue
    [ -f "$root/$f" ] || continue
    size="$(wc -c <"$root/$f" | tr -d ' ')"
    [ "$size" -gt "$BUDGET" ] || continue
    over=$((over + 1))
    listing="$listing  $f  $size bytes (budget $BUDGET, over by $((size - BUDGET)))
"
done <<EOF
$(git -C "$root" ls-files '*/SKILL.md' | awk '!/^benchmark\/results\//')
EOF

if [ "$over" -gt "$CAP" ]; then
    note_fail "$over skill file(s) exceed the $BUDGET byte read budget; the ratchet allows $CAP"
    printf '%s' "$listing" >&2
    cat >&2 <<'RESOLVE'

  WHAT THIS MEANS
    An agent told to read this skill gets a PREFIX of it and is told nothing.
    The skill's later sections are silently absent from the agent's context,
    and the agent will act as though it read them.

  HOW TO RESOLVE, in the order worth trying

    1. SPLIT IT, and make the split explicit.
       Keep SKILL.md as a short index that points at part files, each well
       under the budget. This is what T87 does for planning/SKILL.md: the
       authored content becomes a source file, and a short SKILL.md plus its
       parts are GENERATED from it, the way install.sh is generated from
       installer/src/. Reuse that machinery -- the REVIEWER_SECTION markers,
       generate-reviewer.sh, and the hash test-reviewer-projection.sh pins --
       rather than inventing a second scheme.

    2. MOVE REFERENCE MATERIAL OUT.
       Contracts, schemas and long examples belong in references/ or docs/,
       read on demand by the task that needs them. A skill is an index and a
       contract, not a manual (MAINTAINER.md 1.2).

    3. CUT WHAT THE READER DOES NOT NEED.
       Journey prose, rationale that belongs in a commit message, and worked
       examples that a reference file can hold.

  WHAT NOT TO DO
    Do not raise BUDGET or CAP in this file to make the failure go away. The
    budget is a measured property of the reading agent, not a style preference,
    and raising it does not make the agent read further -- it only stops this
    test telling you that it will not.

  THE MEASUREMENT
    .agents/knowledge/agent-read-limits.md records the caps per harness, how
    they were measured, and the controls that make the numbers trustworthy.
RESOLVE
elif [ "$over" -lt "$CAP" ]; then
    note_fail "only $over skill file(s) are over budget but CAP is $CAP; lower CAP to $over in this commit so the ratchet cannot drift back up"
    printf '%s' "$listing" >&2
else
    printf 'skill-file-length: %s of %s skill file(s) over the %s byte budget, at the cap\n' \
        "$over" "$(git -C "$root" ls-files '*/SKILL.md' | awk '!/^benchmark\/results\//' | wc -l | tr -d ' ')" "$BUDGET"
    printf '%s' "$listing"
fi

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-skill-file-length: PASS'
