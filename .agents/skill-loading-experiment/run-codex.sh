#!/usr/bin/env bash
# MODE: DEV
# One replicate of the codex load-boundary experiment.
#
# Same control as the other two drivers: load, MOVE the file away, then resume
# the session and quiz, so a correct answer can only have come from context.
# codex exec is one-shot, so phase 2 uses `codex exec resume --last`; that
# means replicates must run one at a time in a given codex home.
set -euo pipefail

SRC="${SRC:?SRC=path to the generated marker file}"
MANIFEST="${MANIFEST:?MANIFEST=path to the marker manifest tsv}"
OUTDIR="${OUTDIR:?OUTDIR=where to write raw evidence}"
REP="${REP:-1}"
STYLE="${STYLE:-weak}"

mkdir -p "$OUTDIR"
OUTDIR="$(cd "$OUTDIR" && pwd)"
MANIFEST="$(cd "$(dirname "$MANIFEST")" && pwd)/$(basename "$MANIFEST")"

ws="$(mktemp -d "${TMPDIR:-/tmp}/skill-loading-codex-XXXXXX")"
target="$ws/SKILL.md"
cp "$SRC" "$target"
tag="rep${REP}-${STYLE}"

if [ "$STYLE" = strong ]; then
    phase1="Load the file $target into your context now, in full, from its first line
to its last line. Do not summarise and do not stop early. Reply LOADED."
else
    phase1="Read the skill at $target and follow it. Reply LOADED when ready."
fi

cd "$ws"
timeout 900 codex exec --skip-git-repo-check "$phase1" \
    >"$OUTDIR/${tag}-phase1.txt" 2>&1 || true

moved="${ws}-moved-away.md"
mv "$target" "$moved"

lines="$(awk 'NR>1 {printf "%s ", $1}' "$MANIFEST")"
phase2="From the skill file you loaded earlier, report the VERIFICATION TOKEN on each
of these line numbers: $lines
One line per line number, formatted exactly as: <line> <token>
If a line's token is not in your context, write '<line> ABSENT'. Do not guess
and do not open any file."

timeout 900 codex exec resume --last --skip-git-repo-check "$phase2" \
    >"$OUTDIR/${tag}-phase2.txt" 2>&1 || true

python3 - "$MANIFEST" "$OUTDIR/${tag}-phase2.txt" "$REP" "$STYLE" <<'PY'
import sys
manifest, answer, rep, style = sys.argv[1:5]
text = open(answer, encoding="utf-8", errors="replace").read()
with open(manifest, encoding="utf-8") as handle:
    next(handle)
    for row in handle:
        line, token = row.rstrip("\n").split("\t")
        print(f"{rep}\tcodex\t{style}\t{line}\t{token}\t{'HIT' if token in text else 'MISS'}")
PY

cd /
rm -rf "$ws" "$moved" 2>/dev/null || true
