#!/usr/bin/env bash
# S20-git-merge-resolving: setup WORKDIR | verify WORKDIR
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
    cat > "$WORKDIR/validate.py" <<'PY'
def validate(value):
    return value
PY
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "base: validate() with no guards"

    git -C "$WORKDIR" checkout -q -b feature-a
    cat > "$WORKDIR/validate.py" <<'PY'
def validate(value):
    if value is None:
        raise ValueError("value must not be None")
    return value
PY
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "feature-a: reject None"

    git -C "$WORKDIR" checkout -q main
    git -C "$WORKDIR" checkout -q -b feature-b
    cat > "$WORKDIR/validate.py" <<'PY'
def validate(value):
    if value == "":
        raise ValueError("value must not be empty")
    return value
PY
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "feature-b: reject empty string"
    ;;
verify)
    RESULT="$(cd "$WORKDIR" && python3 - <<'PY'
import importlib
import sys

sys.path.insert(0, ".")
try:
    v = importlib.import_module("validate")
except Exception as e:
    print("import_error:%s" % e)
    raise SystemExit

none_rejected = False
empty_rejected = False
try:
    v.validate(None)
except ValueError:
    none_rejected = True
except Exception:
    pass
try:
    v.validate("")
except ValueError:
    empty_rejected = True
except Exception:
    pass

print("none_rejected:%s empty_rejected:%s" % (none_rejected, empty_rejected))
PY
)"
    case "$RESULT" in
    *"none_rejected:True empty_rejected:True"*)
        printf '{"task_correct": true, "notes": "merged validate() rejects both None and empty string"}\n'
        ;;
    *)
        printf '{"task_correct": false, "notes": "merge result: %s"}\n' "$RESULT"
        ;;
    esac
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
