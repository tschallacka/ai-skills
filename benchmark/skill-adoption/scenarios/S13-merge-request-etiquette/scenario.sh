#!/usr/bin/env bash
# S13-merge-request-etiquette: setup WORKDIR | verify WORKDIR
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
    cat > "$WORKDIR/off_by_one.py" <<'PY'
def indices(n):
    return list(range(n))  # bug: drops the last index callers expect
PY
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "initial"
    git -C "$WORKDIR" checkout -q -b fix/off-by-one
    # `||` and `&&` share precedence and left-associate, so a flat
    # `A || B && C` parses as `(A || B) && C` -- when A (GNU -i.bak) already
    # succeeds, that still runs C (mv) unconditionally even though B (and
    # so .py.new) never ran, and under `set -e` that failing mv aborts the
    # whole script. The BSD fallback must be its own conditional branch.
    if ! sed -i.bak 's/range(n)/range(n + 1)/' "$WORKDIR/off_by_one.py" 2>/dev/null; then
        sed 's/range(n)/range(n + 1)/' "$WORKDIR/off_by_one.py" > "$WORKDIR/off_by_one.py.new"
        mv "$WORKDIR/off_by_one.py.new" "$WORKDIR/off_by_one.py"
    fi
    rm -f "$WORKDIR/off_by_one.py.bak"
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "fix off-by-one in indices()"

    mkdir -p "$WORKDIR/bin"
    cat > "$WORKDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
# Fixture stub -- captures the PR instead of touching any network.
if [ "$1" = "pr" ] && [ "$2" = "create" ]; then
    shift 2
    TITLE=""
    BODY=""
    while [ $# -gt 0 ]; do
        case "$1" in
        --title) TITLE="$2"; shift 2 ;;
        --body) BODY="$2"; shift 2 ;;
        --body-file) BODY="$(cat "$2")"; shift 2 ;;
        *) shift ;;
        esac
    done
    python3 -c "
import json
json.dump({'title': '''$TITLE''', 'body': '''$BODY'''}, open('$(dirname "$0")/../.pr-created.json', 'w'))
"
    echo "https://github.com/example/widgets/pull/1"
    exit 0
fi
echo "gh: fixture stub has no canned response for: $*" >&2
exit 1
SH
    chmod +x "$WORKDIR/bin/gh"
    ;;
verify)
    if [ ! -f "$WORKDIR/.pr-created.json" ]; then
        printf '{"task_correct": false, "notes": "no PR created (.pr-created.json missing)"}\n'
        exit 0
    fi
    python3 - "$WORKDIR/.pr-created.json" <<'PY'
import json
import sys

pr = json.load(open(sys.argv[1]))
body = pr.get("body", "")
first_line = next((l for l in body.splitlines() if l.strip()), "")

padded = any(p in body.lower() for p in ["sorry", "just wanted to", "i hope this", "hopefully"])
short_opener = len(first_line) < 200
names_change = ("off_by_one" in body) or ("range" in body.lower())

ok = short_opener and not padded and names_change
notes = f"short_opener={short_opener} padded={padded} names_change={names_change}"
print('{"task_correct": %s, "notes": "%s"}' % ("true" if ok else "false", notes))
PY
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
