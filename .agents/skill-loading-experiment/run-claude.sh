#!/usr/bin/env bash
# MODE: DEV
# One replicate of the claude-code load-boundary experiment.
#
# Two phases, so the recall question can only be answered from context:
#   1. claude -p is told to load the file; we capture its session_id.
#   2. the file is MOVED AWAY, then the same session is resumed and asked
#      which token sits at each marker line. A re-read is now impossible
#      rather than merely forbidden, which is the whole point of the control.
#
# Phase 1 has two loading paths, selected by $MODE:
#   read    - the file is on disk under --add-dir and the prompt orders a read
#   adddir  - the directory is added but the file is never named; measures
#             whether reachability alone puts anything in context
#
# Emits, on stdout, one TSV row per marker line: replicate, mode, line,
# expected token, whether the answer carried it.
set -euo pipefail

MODE="${MODE:-read}"
MODEL="${MODEL:-sonnet}"
REP="${REP:-1}"
SRC="${SRC:?SRC=path to the generated marker file}"
MANIFEST="${MANIFEST:?MANIFEST=path to the marker manifest tsv}"
OUTDIR="${OUTDIR:?OUTDIR=where to write raw evidence}"

mkdir -p "$OUTDIR"
OUTDIR="$(cd "$OUTDIR" && pwd)"
MANIFEST="$(cd "$(dirname "$MANIFEST")" && pwd)/$(basename "$MANIFEST")"
ws="$(mktemp -d "${TMPDIR:-/tmp}/skill-loading-claude-XXXXXX")"
target="$ws/SKILL.md"
cp "$SRC" "$target"
# Run inside the workspace so each replicate's transcript lands in its own
# ~/.claude/projects/<escaped-cwd>/ tree, well away from any live session's.
cd "$ws"

case "$MODE" in
    read)
        phase1="Load the skill file $target into your context now, in full, from
its first line to its last line. Do not summarise and do not stop early. When
the whole file is loaded, reply with the single word LOADED."
        ;;
    adddir)
        phase1="The directory $ws has been added to your workspace. Reply with
the single word LOADED."
        ;;
    *) printf 'unknown MODE: %s\n' "$MODE" >&2; exit 64 ;;
esac

p1="$OUTDIR/rep${REP}-${MODE}-phase1.json"
claude -p --output-format=json \
    --add-dir "$ws" \
    --permission-mode acceptEdits \
    --model "$MODEL" \
    "$phase1" >"$p1" 2>"$OUTDIR/rep${REP}-${MODE}-phase1.err" || true

sid="$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$p1" | sed -n '1p')"
if [ -z "$sid" ]; then
    printf 'no session_id in %s\n' "$p1" >&2
    exit 1
fi
printf 'session_id=%s\n' "$sid" >"$OUTDIR/rep${REP}-${MODE}-session.txt"

# The control: make a re-read physically impossible before asking anything.
moved="${ws}-moved-away.md"
mv "$target" "$moved"

lines="$(awk 'NR>1 {printf "%s ", $1}' "$MANIFEST")"
phase2="From the skill file you loaded earlier, report the VERIFICATION TOKEN
that appears on each of these line numbers: $lines
Answer with one line per line number, formatted exactly as: <line> <token>
If you do not have a line's token in your context, write '<line> ABSENT'.
Do not guess and do not attempt to open any file."

p2="$OUTDIR/rep${REP}-${MODE}-phase2.json"
claude -p --output-format=json \
    --resume "$sid" \
    --add-dir "$ws" \
    --permission-mode acceptEdits \
    --model "$MODEL" \
    "$phase2" >"$p2" 2>"$OUTDIR/rep${REP}-${MODE}-phase2.err" || true

answer="$OUTDIR/rep${REP}-${MODE}-answer.txt"
python3 -c '
import json, sys
raw = open(sys.argv[1], encoding="utf-8").read()
try:
    doc = json.loads(raw)
    print(doc.get("result", ""))
except Exception:
    print(raw)
' "$p2" >"$answer"

# Score: a marker counts as recalled only if its exact token text appears.
python3 - "$MANIFEST" "$answer" "$REP" "$MODE" "$MODEL" <<'PY'
import sys
manifest, answer, rep, mode, model = sys.argv[1:6]
text = open(answer, encoding="utf-8").read()
with open(manifest, encoding="utf-8") as handle:
    next(handle)
    for row in handle:
        line, token = row.rstrip("\n").split("\t")
        got = "HIT" if token in text else "MISS"
        print(f"{rep}\t{mode}\t{model}\t{line}\t{token}\t{got}")
PY

# Copy this replicate's own transcript out as direct evidence of what was
# actually sent. Read-only, and matched by session id so no other session's
# transcript is ever touched.
transcript="$(find "${CLAUDE_PROJECTS_DIR:-$HOME/.claude/projects}" -type f -name "${sid}.jsonl" -print -quit 2>/dev/null || true)"
if [ -n "$transcript" ]; then
    cp "$transcript" "$OUTDIR/rep${REP}-${MODE}-transcript.jsonl"
fi

cd /
rm -rf "$ws" "$moved" 2>/dev/null || true
