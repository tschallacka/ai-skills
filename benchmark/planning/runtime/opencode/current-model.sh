#!/usr/bin/env bash
# Resolve "the model the user is currently using" for benchmark runs.
#
# The opencode driver (agent.sh) falls back to its own default
# (opencode/big-pickle) when OPENCODE_MODEL is unset; this resolver instead
# asks opencode's session store for the model of the user's most recent
# interactive session. Benchmark worker/reviewer/analyzer sessions are
# excluded so a just-finished run cannot pollute the default for the next one.
#
# Precedence:
#   1. OPENCODE_MODEL, when already set (explicit override wins).
#   2. The model of the newest opencode.db session whose directory is not a
#      benchmark workspace or result dir (model JSON -> providerID/id).
#   3. The harness default opencode/big-pickle.
set -euo pipefail

if [ -n "${OPENCODE_MODEL:-}" ]; then
    printf '%s\n' "$OPENCODE_MODEL"
    exit 0
fi

db=""
for base in "${XDG_DATA_HOME:-$HOME/.local/share}/opencode" "$HOME/.local/share/opencode"; do
    if [ -f "$base/opencode.db" ]; then
        db="$base/opencode.db"
        break
    fi
done

if [ -z "$db" ] || ! command -v python3 >/dev/null 2>&1; then
    printf '%s\n' "opencode/big-pickle"
    exit 0
fi

model="$(python3 - "$db" <<'PY'
import json
import sqlite3
import sys

db = sys.argv[1]
try:
    conn = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = conn.execute(
        "select model, directory from session order by time_created desc"
    ).fetchall()
    conn.close()
except sqlite3.Error:
    sys.exit(1)
for raw, directory in rows:
    if not raw or "/ai-skills-benchmark/" in directory or "/benchmark/results/" in directory:
        continue
    try:
        parsed = json.loads(raw)
        provider, model_id = parsed.get("providerID", ""), parsed.get("id", "")
    except (ValueError, TypeError):
        provider, model_id = "", raw
    if provider and model_id:
        print(f"{provider}/{model_id}")
        sys.exit(0)
sys.exit(1)
PY
)" || model="opencode/big-pickle"

printf '%s\n' "$model"