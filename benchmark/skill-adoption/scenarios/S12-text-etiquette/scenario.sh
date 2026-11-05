#!/usr/bin/env bash
# S12-text-etiquette: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    git -C "$WORKDIR" init -q -b main 2>/dev/null || git -C "$WORKDIR" init -q
    touch "$WORKDIR/.gitignore"
    ;;
verify)
    GITIGNORE_OK=false
    [ -f "$WORKDIR/.gitignore" ] && grep -q '\*\.pyc' "$WORKDIR/.gitignore" && GITIGNORE_OK=true

    CONCISE="unknown"
    if [ -f "$WORKDIR/.transcript.txt" ]; then
        CONCISE="$(python3 - "$WORKDIR/.transcript.txt" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
m = re.search(r"\b(main|master)\b", text)
if not m:
    print("unknown")
else:
    # Rough heuristic: the reply segment is the ~400 chars around the first
    # mention of main/master, up to the next blank-line-delimited paragraph
    # break. This is approximate -- CLI chrome (box borders, prompts) is not
    # stripped out -- tune against real transcripts per PROTOCOL.md's own
    # "unverified assumptions" stance rather than trusting this blindly.
    segment = text[max(0, m.start() - 100):m.start() + 300]
    para = segment.split("\n\n")[0]
    words = len(para.split())
    print("concise" if words <= 40 else "verbose")
PY
)"
    fi

    if [ "$GITIGNORE_OK" = true ]; then
        printf '{"task_correct": true, "notes": "gitignore entry landed; style heuristic: %s"}\n' "$CONCISE"
    else
        printf '{"task_correct": false, "notes": "gitignore entry missing; style heuristic: %s"}\n' "$CONCISE"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
