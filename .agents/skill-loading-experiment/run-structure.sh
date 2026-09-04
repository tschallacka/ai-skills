#!/usr/bin/env bash
# MODE: DEV
# One replicate of a split-structure condition, on either agent.
#
# Phase 1 loads the skill directory. Phase 2 MOVES every part file away and
# then quizzes the resumed session on each part's deep content marker, so a
# hit can only have come from context.
#
# Two things are scored, and they are not the same question:
#   completeness - did phase 1 echo every part's LOAD TOKEN (verify structure)
#   retention    - did phase 2 recall each part's deep content marker
#
# Usage: AGENT=claude|opencode STRUCTURE=... SKILLDIR=... OUTDIR=... [MODEL=]
set -euo pipefail

AGENT="${AGENT:-claude}"
STRUCTURE="${STRUCTURE:?STRUCTURE=imperative|index|verify}"
SKILLDIR="${SKILLDIR:?SKILLDIR=directory holding SKILL.md and the parts}"
OUTDIR="${OUTDIR:?OUTDIR=where to write raw evidence}"
REP="${REP:-1}"

mkdir -p "$OUTDIR"
OUTDIR="$(cd "$OUTDIR" && pwd)"
SKILLDIR="$(cd "$SKILLDIR" && pwd)"

ws="$(mktemp -d "${TMPDIR:-/tmp}/skill-loading-struct-XXXXXX")"
cp "$SKILLDIR"/*.md "$ws/"
tag="${AGENT}-${STRUCTURE}-r${REP}"

phase1="Load the skill at $ws/SKILL.md and follow it. Reply when ready."

if [ "$AGENT" = claude ]; then
    MODEL="${MODEL:-sonnet}"
    claude -p --output-format=json --add-dir "$ws" \
        --permission-mode acceptEdits --model "$MODEL" "$phase1" \
        >"$OUTDIR/${tag}-phase1.json" 2>"$OUTDIR/${tag}-phase1.err" || true
    sid="$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        "$OUTDIR/${tag}-phase1.json" | sed -n '1p')"
else
    MODEL="${MODEL:-opencode/big-pickle}"
    opencode run --format json --dir "$ws" --model "$MODEL" --auto "$phase1" \
        >"$OUTDIR/${tag}-phase1.json" 2>"$OUTDIR/${tag}-phase1.err" || true
    sid="$(sed -nE 's/.*"sessionID"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' \
        "$OUTDIR/${tag}-phase1.json" | sed -n '1p')"
fi
printf '%s\n' "$sid" >"$OUTDIR/${tag}-session.txt"

# The control: the parts are gone before any question is asked.
moved="${ws}-moved"
mkdir -p "$moved"
mv "$ws"/part-*.md "$moved/"

phase2="For each part file of the skill you loaded, report the VERIFICATION TOKEN
it contains. One line per part, formatted exactly as: <part filename> <token>
If a part's token is not in your context, write '<part filename> ABSENT'. Do
not guess and do not open any file."

if [ "$AGENT" = claude ]; then
    claude -p --output-format=json --resume "$sid" --add-dir "$ws" \
        --permission-mode acceptEdits --model "$MODEL" "$phase2" \
        >"$OUTDIR/${tag}-phase2.json" 2>"$OUTDIR/${tag}-phase2.err" || true
else
    opencode run --format json --dir "$ws" --model "$MODEL" -s "$sid" --auto \
        "$phase2" >"$OUTDIR/${tag}-phase2.json" 2>"$OUTDIR/${tag}-phase2.err" || true
fi

python3 - "$SKILLDIR" "$OUTDIR/${tag}-phase1.json" "$OUTDIR/${tag}-phase2.json" \
    "$AGENT" "$STRUCTURE" "$MODEL" "$REP" <<'PY'
import sys

skilldir, p1, p2, agent, structure, model, rep = sys.argv[1:8]


def read(path):
    try:
        return open(path, encoding="utf-8", errors="replace").read()
    except OSError:
        return ""


def manifest(name):
    rows = {}
    with open(f"{skilldir}/{name}", encoding="utf-8") as handle:
        next(handle)
        for row in handle:
            part, token = row.rstrip("\n").split("\t")
            rows[part] = token
    return rows


phase1 = read(p1)
phase2 = read(p2)
loads = manifest("loads.tsv")
markers = manifest("markers.tsv")

for part in loads:
    echoed = "ECHOED" if loads[part] in phase1 else "NOT_ECHOED"
    recalled = "HIT" if markers[part] in phase2 else "MISS"
    print(f"{rep}\t{agent}\t{structure}\t{model}\t{part}\t{echoed}\t{recalled}")
PY

rm -rf "$ws" "$moved" 2>/dev/null || true
