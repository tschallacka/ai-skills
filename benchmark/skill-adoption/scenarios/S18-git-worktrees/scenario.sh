#!/usr/bin/env bash
# S18-git-worktrees: setup WORKDIR | verify WORKDIR
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
    cat > "$WORKDIR/transform.py" <<'PY'
def normalize(rows):
    """Baseline implementation -- the subject is asked to try two different
    refactors of this at once, side by side."""
    return [r.strip().lower() for r in rows]
PY
    git -C "$WORKDIR" add -A
    git -C "$WORKDIR" -c user.email=a@b.c -c user.name=setup commit -q -m "initial"
    ;;
verify)
    LISTED_WORKTREES=0
    WT_OUTPUT="$(git -C "$WORKDIR" worktree list 2>/dev/null || true)"
    if [ -n "$WT_OUTPUT" ]; then
        LISTED_WORKTREES="$(printf '%s\n' "$WT_OUTPUT" | wc -l | tr -d ' ')"
    fi

    # Fallback: the subject may have made separate plain clones instead of
    # using `git worktree` directly -- still satisfies the actual goal
    # (both approaches simultaneously inspectable), so count those too.
    SIBLING_DIRS=0
    PARENT="$(dirname "$WORKDIR")"
    if [ -d "$PARENT" ]; then
        SIBLING_DIRS="$(find "$PARENT" -maxdepth 1 -type d -name '*transform*' -o -maxdepth 1 -type d -name '*approach*' 2>/dev/null | wc -l | tr -d ' ')"
    fi

    TOTAL=$((LISTED_WORKTREES + SIBLING_DIRS))
    if [ "$TOTAL" -ge 2 ]; then
        printf '{"task_correct": true, "notes": "worktrees=%s sibling_clone_dirs=%s -- at least two separate working copies exist"}\n' "$LISTED_WORKTREES" "$SIBLING_DIRS"
    else
        printf '{"task_correct": false, "notes": "worktrees=%s sibling_clone_dirs=%s -- no evidence of two simultaneously-inspectable copies"}\n' "$LISTED_WORKTREES" "$SIBLING_DIRS"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
