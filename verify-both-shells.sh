#!/usr/bin/env bash
# MODE: DEV
# verify-both-shells.sh — run the suite on the working tree under both shells.
#
# Verifies the WORKING TREE, not HEAD, in a linked worktree away from the repo, so
# editing can continue here while it runs. Both legs matter: the local bash, and
# the bash 3.2 floor CODE-STYLE.md section 1 declares, because stock macOS ships
# 3.2 and a bash-4 construct is invisible under a newer shell.
#
# Usage:
#   ./verify-both-shells.sh            # both legs
#   ./verify-both-shells.sh --keep     # keep the logs and the worktree on failure
#   ./verify-both-shells.sh --help
#
# A linked worktree rather than a clone: it shares the object store, so every ref
# is present. blast-radius.sh resolves a merge base against master, and a clone
# has only origin/master -- which made an earlier version of this report a failure
# of itself as a failure of the code. --detach because the branch is checked out
# in the main repo, and it lives in TMPDIR, never under the repo: a worktree
# inside it gets picked up by the filesystem scans and lands machine-specific
# paths in generated artifacts, which is how PORTABILITY.md was once polluted.
set -uo pipefail

src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
keep=false
case "${1:-}" in
    --keep) keep=true ;;
    -h|--help) sed -n '3,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    '') ;;
    *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
esac

wt="$(mktemp -d "${TMPDIR:-/tmp}/verify-wt.XXXXXX")/tree"
log5="$(mktemp "${TMPDIR:-/tmp}/verify-log5.XXXXXX")"
log3="$(mktemp "${TMPDIR:-/tmp}/verify-log3.XXXXXX")"
status=0

cleanup() {
    git -C "$src" worktree remove --force "$wt" 2>/dev/null
    rm -rf "$(dirname "$wt")"
    if [ "$keep" = true ] && [ "$status" -ne 0 ]; then
        printf 'logs kept: %s %s\n' "$log5" "$log3" >&2
    else
        rm -f "$log5" "$log3"
    fi
}
# INT/TERM as well as EXIT: an interrupted run must not leave a registration
# behind. A SIGKILL still can, which is what the sweep below is for.
trap cleanup EXIT INT TERM HUP

# Self-cleaning is not a trap alone. Sweep this harness's own leftovers first: a
# killed run leaves a registered worktree, and a registration under the repo is
# what puts machine-specific paths into generated artifacts.
git -C "$src" worktree list --porcelain \
    | sed -n 's|^worktree \(.*/verify-wt\..*\)$|\1|p' \
    | while IFS= read -r stale; do
        printf 'sweeping leftover worktree %s\n' "$stale"
        git -C "$src" worktree remove --force "$stale" 2>/dev/null
        rm -rf "$(dirname "$stale")"
    done

git -C "$src" worktree add --detach --quiet "$wt" HEAD || exit 70
# The working tree, including uncommitted edits, so what is verified is what is
# in front of you. Untracked files are deliberately not carried: a stray file is
# not part of the change.
overlaid=0
while IFS= read -r path; do
    [ -n "$path" ] || continue
    if [ -f "$src/$path" ]; then
        mkdir -p "$wt/$(dirname "$path")"
        cp "$src/$path" "$wt/$path"
    else
        rm -f "$wt/$path"
    fi
    overlaid=$((overlaid + 1))
done < <(git -C "$src" diff HEAD --name-only)

printf 'worktree %s (base %s, %s file(s) overlaid)\n' "$wt" \
    "$(git -C "$src" rev-parse --short HEAD)" "$overlaid"

# Report the summary, and on a failure the failing tests' own output. Printing
# only the summary is what turned a real bash 3.2 failure into a test name with
# no diagnosis, after the log had already been deleted.
report() {
    local label="$1" log="$2"
    if ! grep -qE 'Total ran' "$log"; then
        printf '=== %s -- NO SUMMARY, the leg did not run ===\n' "$label"
        tail -20 "$log"
        status=1
        return
    fi
    printf '=== %s ===\n' "$label"
    grep -E 'Total ran|^Failed:' "$log"
    grep -qE '^Failed:' "$log" || return 0
    status=1
    printf '%s\n' "--- $label: what each failing test said ---"
    # run-tests.sh prints a failing test's whole output, indented, after its
    # FAIL line. Take each such block up to the next test's result line.
    awk '
        /^  [^ ].* (PASS|FAIL|UNCONFIGURED)/ { inblock = ($0 ~ /FAIL/) }
        inblock { print }
    ' "$log"
}

( cd "$wt" && ./run-tests.sh ) >"$log5" 2>&1
( cd "$wt" && nix develop "$src" --command bash32-run-tests ) >"$log3" 2>&1
report 'bash 5.3' "$log5"
report 'bash 3.2' "$log3"
exit "$status"
