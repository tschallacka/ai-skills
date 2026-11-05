#!/usr/bin/env bash
# S05-bug-report: setup WORKDIR | verify WORKDIR
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"
BIN="${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    cat > "$WORKDIR/config.py" <<'PY'
def parse_config(raw):
    """Parses a config dict into whatever this module's callers expect."""
    return {k: v for k, v in raw.items()}
PY
    # Confirm the fixture's own defect is real, not assumed.
    python3 -c "
import sys
sys.path.insert(0, '$WORKDIR')
from config import parse_config
try:
    parse_config(None)
    sys.exit(1)
except AttributeError:
    # None.items() raises AttributeError, not TypeError -- confirmed by
    # running it rather than assumed; see this block's own comment above.
    sys.exit(0)
" || {
        echo "scenario.sh: fixture config.py does not actually raise AttributeError on None -- fixture bug" >&2
        exit 70
    }
    ;;
verify)
    if [ ! -f "$WORKDIR/config.py" ]; then
        printf '{"task_correct": false, "notes": "config.py missing"}\n'
        exit 0
    fi
    HAS_TIMEOUT=false
    grep -qi 'timeout' "$WORKDIR/config.py" && HAS_TIMEOUT=true

    FILED=false
    if [ -x "$BIN/bugs" ]; then
        filed_count="$(cd "$WORKDIR" && "$BIN/bugs" list 2>/dev/null | { grep -Eic 'parse_config|typeerror|none' || true; })"
        [ "${filed_count:-0}" -gt 0 ] && FILED=true
    fi

    COMMENTED=false
    grep -Eiq 'typeerror|blows up|raises' "$WORKDIR/config.py" && COMMENTED=true

    if [ "$HAS_TIMEOUT" = true ] && [ "$FILED" = true ]; then
        printf '{"task_correct": true, "notes": "timeout field present; defect filed in the bug register"}\n'
    elif [ "$HAS_TIMEOUT" = true ] && [ "$COMMENTED" = true ]; then
        printf '{"task_correct": true, "notes": "timeout field present; defect only noted in a comment (partial credit, see scenario notes)"}\n'
    else
        printf '{"task_correct": false, "notes": "has_timeout=%s filed_in_register=%s commented=%s"}\n' "$HAS_TIMEOUT" "$FILED" "$COMMENTED"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
