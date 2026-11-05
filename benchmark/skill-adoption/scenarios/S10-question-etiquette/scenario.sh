#!/usr/bin/env bash
# S10-question-etiquette: setup WORKDIR | verify WORKDIR
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
    """No eviction policy and no tenant-awareness yet."""

    def __init__(self):
        self._data = {}

    def get(self, key):
        return self._data.get(key)

    def set(self, key, value):
        self._data[key] = value
PY
    ;;
verify)
    if [ ! -f "$WORKDIR/.transcript.txt" ]; then
        printf '{"task_correct": false, "notes": ".transcript.txt missing"}\n'
        exit 0
    fi
    python3 - "$WORKDIR/.transcript.txt" <<'PY'
import re
import sys

text = open(sys.argv[1], encoding="utf-8", errors="replace").read()
# Tolerate markdown emphasis around the label ("**Q1.**", "__a.__") -- a
# reasonable rendering choice that must not cost real adoption its score
# (B6: measured live, a bolded "**Q1.**" made the un-widened anchor below
# match nothing on an otherwise-compliant transcript).
questions = re.findall(r"(?m)^[*_]*Q\d+[.)]", text)
sub_items = re.findall(r"(?m)^\s*[*_]*[a-z][.)][*_]*\s", text)
escape = bool(re.search(r"something else|tell me|free text|your own words", text, re.I))

ok = len(questions) >= 2 and len(sub_items) >= 2 and escape
notes = f"questions={len(questions)} sub_items={len(sub_items)} escape_seen={escape}"
print('{"task_correct": %s, "notes": "%s"}' % ("true" if ok else "false", notes))
PY
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
