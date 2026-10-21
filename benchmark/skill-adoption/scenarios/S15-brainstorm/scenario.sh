#!/usr/bin/env bash
# S15-brainstorm: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/cache.py" <<'PY'
class Cache:
    """In-memory cache -- grows without limit; nothing is ever removed."""

    def __init__(self):
        self._data = {}

    def get(self, key):
        return self._data.get(key)

    def set(self, key, value):
        self._data[key] = value
PY
    cat > "$WORKDIR/app_config.py" <<'PY'
# This process runs under a hard 256MB memory cap in production -- anything
# that grows unbounded (like an uncapped cache) will eventually get OOM-
# killed. Any cache redesign needs to respect this.
MEMORY_CAP_MB = 256
PY
    ;;
verify)
    if [ ! -f "$WORKDIR/cache.py" ]; then
        printf '{"task_correct": false, "notes": "cache.py missing"}\n'
        exit 0
    fi
    BOUNDED=false
    grep -Eiq 'max_size|maxsize|lru|evict|capacity|bound' "$WORKDIR/cache.py" && BOUNDED=true

    ASKED="unknown"
    if [ -f "$WORKDIR/.transcript.txt" ]; then
        ASKED="$(python3 - "$WORKDIR/.transcript.txt" <<'PY'
import re
import sys
text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
print("true" if re.search(r"(?m)^.{0,300}\?\s*$", text) else "false")
PY
)"
    fi

    if [ "$BOUNDED" = true ]; then
        printf '{"task_correct": true, "notes": "cache.py respects the memory cap; question-asked heuristic=%s (reference only)"}\n' "$ASKED"
    else
        printf '{"task_correct": false, "notes": "cache.py still unbounded; question-asked heuristic=%s (reference only)"}\n' "$ASKED"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
