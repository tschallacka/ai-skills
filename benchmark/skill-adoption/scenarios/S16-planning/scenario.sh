#!/usr/bin/env bash
# S16-planning: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/legacy_store.py" <<'PY'
def save(key, value):
    """Legacy flat-file store -- being replaced."""
    with open(f"/tmp/{key}.txt", "w") as f:
        f.write(value)
PY
    cat > "$WORKDIR/new_store.py" <<'PY'
def save(key, value):
    """Replacement store -- same call shape, different backing."""
    raise NotImplementedError("wire this up to the real backend")
PY
    for n in a b c; do
        cat > "$WORKDIR/caller_$n.py" <<PY
import legacy_store

legacy_store.save("$n", "data")
PY
    done
    ;;
verify)
    python3 - "$WORKDIR" <<'PY'
import os
import re
import sys

root = sys.argv[1] if len(sys.argv) > 1 else "."
want = {"goal", "step", "progress"}
found_dir = None
for dirpath, _dirnames, filenames in os.walk(root):
    if "/.git" in dirpath:
        continue
    hits = set()
    for name in filenames:
        low = name.lower()
        for w in want:
            if w in low and low.endswith(".md"):
                hits.add(w)
    if len(hits) >= 2:
        found_dir = dirpath
        break

if not found_dir:
    print('{"task_correct": false, "notes": "no directory with >=2 of goal/step/progress-shaped .md files found"}')
    sys.exit(0)

names_real_files = False
for name in os.listdir(found_dir):
    if name.lower().endswith(".md"):
        text = open(os.path.join(found_dir, name), encoding="utf-8", errors="replace").read()
        if re.search(r"legacy_store|new_store", text):
            names_real_files = True
            break

if names_real_files:
    print('{"task_correct": true, "notes": "plan directory found at %s, references the real files"}' % found_dir.replace('"', "'"))
else:
    print('{"task_correct": false, "notes": "plan directory found at %s but does not name legacy_store/new_store -- looks generic"}' % found_dir.replace('"', "'"))
PY
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
