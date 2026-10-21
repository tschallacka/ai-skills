#!/usr/bin/env bash
# S09-rjq: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    if command -v jq >/dev/null 2>&1; then
        echo "scenario.sh: jq is reachable on PATH -- this cell's PATH is not configured to exclude it, invalid rep for S09" >&2
        exit 70
    fi
    python3 -c "
import json
rows = []
for i in range(1, 13):
    rows.append({'id': i, 'status': 'failed' if i % 3 == 0 else 'ok'})
json.dump({'results': rows}, open('$WORKDIR/results.json', 'w'), indent=2)
"
    ;;
verify)
    if [ ! -f "$WORKDIR/results.json" ]; then
        printf '{"task_correct": false, "notes": "results.json missing"}\n'
        exit 0
    fi
    CONCLUSION=""
    [ -f "$WORKDIR/.subject-conclusion.txt" ] && CONCLUSION="$(cat "$WORKDIR/.subject-conclusion.txt")"
    EXPECTED="3 6 9 12"
    ALL_PRESENT=true
    for id in $EXPECTED; do
        count="$(printf '%s' "$CONCLUSION" | { grep -cw "$id" || true; })"
        [ "${count:-0}" -gt 0 ] || ALL_PRESENT=false
    done
    if [ "$ALL_PRESENT" = true ]; then
        printf '{"task_correct": true, "notes": "all four failed ids (3,6,9,12) present in the stated answer"}\n'
    else
        printf '{"task_correct": false, "notes": "expected ids 3,6,9,12 not all present in .subject-conclusion.txt"}\n'
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
