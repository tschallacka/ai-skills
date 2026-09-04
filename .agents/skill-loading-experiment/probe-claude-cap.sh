#!/usr/bin/env bash
# MODE: DEV
# Measures what claude's Read tool actually hands back for one file.
#
# Single phase and no quiz: the answer comes from the session transcript, so
# this is direct evidence of what entered context and costs one turn. Use it
# to bisect the cap; use run-claude.sh when the question is retention rather
# than delivery.
#
# Prints: file, source lines, source bytes, tool_result lines, tool_result
# chars, and how many Read calls were made.
set -euo pipefail

SRC="${SRC:?SRC=path to the file to probe}"
MODEL="${MODEL:-sonnet}"
LABEL="${LABEL:-probe}"
OUTDIR="${OUTDIR:?OUTDIR=where to write raw evidence}"

mkdir -p "$OUTDIR"
OUTDIR="$(cd "$OUTDIR" && pwd)"
here="$(cd "$(dirname "$0")" && pwd)"
src_lines="$(wc -l <"$SRC" | tr -d ' ')"
src_bytes="$(wc -c <"$SRC" | tr -d ' ')"

ws="$(mktemp -d "${TMPDIR:-/tmp}/skill-loading-probe-XXXXXX")"
cp "$SRC" "$ws/SKILL.md"
cd "$ws"

# STYLE=strong is an emphatic read-it-all directive; STYLE=weak is how a real
# skill is actually loaded, which is the condition that matters in practice.
case "${STYLE:-strong}" in
    strong)
        prompt="Load the file $ws/SKILL.md into your context now, in full, from its first
line to its last line. Do not summarise and do not stop early. When the whole
file is loaded, reply with the single word LOADED."
        ;;
    weak)
        prompt="Read the skill at $ws/SKILL.md and follow it. Reply LOADED when ready."
        ;;
    *) printf 'unknown STYLE: %s\n' "${STYLE:-}" >&2; exit 64 ;;
esac

out="$OUTDIR/${LABEL}-phase1.json"
claude -p --output-format=json \
    --add-dir "$ws" \
    --permission-mode acceptEdits \
    --model "$MODEL" \
    "$prompt" \
    >"$out" 2>"$OUTDIR/${LABEL}-phase1.err" || true

sid="$(sed -nE 's/.*"session_id"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$out" | sed -n '1p')"
transcript="$(find "${CLAUDE_PROJECTS_DIR:-$HOME/.claude/projects}" -type f -name "${sid}.jsonl" -print -quit 2>/dev/null || true)"
if [ -n "$transcript" ]; then
    cp "$transcript" "$OUTDIR/${LABEL}-transcript.jsonl"
    python3 "$here/summarise-read-calls.py" \
        "$OUTDIR/${LABEL}-transcript.jsonl" "$LABEL" "$MODEL" \
        "$src_lines" "$src_bytes"
else
    printf '%s\t%s\t%s\t%s\tNO_TRANSCRIPT\n' "$LABEL" "$MODEL" "$src_lines" "$src_bytes"
fi

cd /
rm -rf "$ws"
