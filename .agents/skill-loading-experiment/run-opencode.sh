#!/usr/bin/env bash
# MODE: DEV
# One replicate of the opencode load-boundary experiment.
#
# Same two-phase control as the claude driver: load, MOVE the file away, then
# resume the session (`opencode run -s <id>`) and quiz. A correct answer can
# only have come from context.
#
# MODE selects the loading path, and the two are genuinely different
# mechanisms rather than two phrasings:
#   attach - the file is handed over as an `-f` attachment, the way
#            benchmark/planning/runtime/opencode/agent.sh loads a capsule
#   read   - no attachment; the prompt tells the agent to open the file itself
#
# Emits one TSV row per marker line: replicate, mode, model, line, expected
# token, HIT or MISS.
set -euo pipefail

MODE="${MODE:-attach}"
MODEL="${MODEL:-opencode/claude-sonnet-4-5}"
REP="${REP:-1}"
SRC="${SRC:?SRC=path to the generated marker file}"
MANIFEST="${MANIFEST:?MANIFEST=path to the marker manifest tsv}"
OUTDIR="${OUTDIR:?OUTDIR=where to write raw evidence}"

mkdir -p "$OUTDIR"
OUTDIR="$(cd "$OUTDIR" && pwd)"
MANIFEST="$(cd "$(dirname "$MANIFEST")" && pwd)/$(basename "$MANIFEST")"

ws="$(mktemp -d "${TMPDIR:-/tmp}/skill-loading-oc-XXXXXX")"
target="$ws/SKILL.md"
cp "$SRC" "$target"

tag="rep${REP}-${MODE}"
p1="$OUTDIR/${tag}-phase1.json"

if [ "$MODE" = attach ]; then
    opencode run --format json --dir "$ws" --model "$MODEL" \
        -f "$target" --auto \
        "The attached skill file is now in your context. Reply with the single word LOADED." \
        >"$p1" 2>"$OUTDIR/${tag}-phase1.err" || true
else
    opencode run --format json --dir "$ws" --model "$MODEL" --auto \
        "Read the skill at $target and follow it. Reply LOADED when ready." \
        >"$p1" 2>"$OUTDIR/${tag}-phase1.err" || true
fi

sid="$(sed -nE 's/.*"sessionID"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$p1" | sed -n '1p')"
if [ -z "$sid" ]; then
    printf 'no sessionID in %s\n' "$p1" >&2
    exit 1
fi
printf 'session_id=%s\n' "$sid" >"$OUTDIR/${tag}-session.txt"

# The control: a re-read is now impossible, not merely forbidden.
moved="${ws}-moved-away.md"
mv "$target" "$moved"

lines="$(awk 'NR>1 {printf "%s ", $1}' "$MANIFEST")"
p2="$OUTDIR/${tag}-phase2.json"
opencode run --format json --dir "$ws" --model "$MODEL" -s "$sid" --auto \
    "From the skill file you loaded earlier, report the VERIFICATION TOKEN on each
of these line numbers: $lines
Answer one line per line number, formatted exactly as: <line> <token>
If a line's token is not in your context, write '<line> ABSENT'. Do not guess
and do not open any file." \
    >"$p2" 2>"$OUTDIR/${tag}-phase2.err" || true

python3 - "$MANIFEST" "$p2" "$REP" "$MODE" "$MODEL" <<'PY'
import sys
manifest, answer, rep, mode, model = sys.argv[1:6]
text = open(answer, encoding="utf-8", errors="replace").read()
with open(manifest, encoding="utf-8") as handle:
    next(handle)
    for row in handle:
        line, token = row.rstrip("\n").split("\t")
        print(f"{rep}\t{mode}\t{model}\t{line}\t{token}\t{'HIT' if token in text else 'MISS'}")
PY

rm -rf "$ws" "$moved" 2>/dev/null || true
