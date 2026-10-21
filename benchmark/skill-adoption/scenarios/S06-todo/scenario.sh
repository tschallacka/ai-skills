#!/usr/bin/env bash
# S06-todo: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/utils.py" <<'PY'
def slugify(text):
    return text.lower().replace(" ", "-")
PY
    cat > "$WORKDIR/parser.py" <<'PY'
import utils


def parse_line(line):
    return utils.slugify(line.strip())
PY
    cat > "$WORKDIR/main.py" <<'PY'
import utils

GREETING = utils.slugify("Hello World")
PY
    cat > "$WORKDIR/README.md" <<'MD'
# Project

A small CLI. The `--dry-run` flag is new and not yet documented here.
MD
    ;;
verify)
    RENAMED=false
    if [ ! -f "$WORKDIR/utils.py" ] && [ -f "$WORKDIR/helpers.py" ] \
        && ! grep -rq '^import utils\|from utils' "$WORKDIR" --include='*.py' 2>/dev/null; then
        RENAMED=true
    fi

    TYPED=false
    if [ -f "$WORKDIR/parser.py" ] && grep -Eq 'def [a-zA-Z_]+\([a-zA-Z_]+: ' "$WORKDIR/parser.py"; then
        TYPED=true
    fi

    DOCUMENTED=false
    if [ -f "$WORKDIR/README.md" ] && grep -q -- '--dry-run' "$WORKDIR/README.md" \
        && [ "$(grep -c -- '--dry-run' "$WORKDIR/README.md")" -ge 1 ] \
        && ! grep -q 'not yet documented' "$WORKDIR/README.md"; then
        DOCUMENTED=true
    fi

    if [ "$RENAMED" = true ] && [ "$TYPED" = true ] && [ "$DOCUMENTED" = true ]; then
        printf '{"task_correct": true, "notes": "all three items landed"}\n'
    else
        printf '{"task_correct": false, "notes": "renamed=%s typed=%s documented=%s"}\n' "$RENAMED" "$TYPED" "$DOCUMENTED"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
