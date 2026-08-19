#!/usr/bin/env bash
# blast-radius.sh — report what a change set touches beyond the files it edits.
#
# This is an integration-safety report, not a correctness check. It catches the
# mistakes that come from a file's couplings: a generated artifact left stale, a
# new registry that never ships, a branch whose base moved under it. It will not
# find a logic bug, and it is not a substitute for exercising the changed path.
#
# Four passes:
#   1. freshness  — generated artifacts whose sources moved (fails)
#   2. registry   — a new file under planning/ with no manifest row (fails for a
#                   runtime registry, warns for anything else)
#   3. drift      — commits that touched these files since the base ref, which
#                   is how a stale branch silently reverts someone else's fix
#   4. contracts  — couplings recorded in coupling.tsv that a human must honour
#
# Usage:
#   blast-radius.sh [--base <ref>] [<path> ...]
#   blast-radius.sh --help
#
# With no paths, the working tree's own changes are used (staged, unstaged and
# untracked). --base defaults to master and only affects the drift pass.
#
# Exit codes: 0 clean, 1 findings, 64 bad invocation, 69 not a git work tree.

set -euo pipefail
export LC_ALL=C

case "${1:-}" in
    -h|--help)
        awk 'NR == 1 { next }
             /^#/ {
                 sub(/^#[[:space:]]?/, "")
                 if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
                 print; next
             }
             { exit }' "$0"
        exit 0
        ;;
esac

base=master
paths=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --base) [ "$#" -ge 2 ] || { printf '%s: --base needs a ref\n' "${0##*/}" >&2; exit 64; }
                base="$2"; shift 2 ;;
        --base=*) base="${1#--base=}"; shift ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
        *) paths+=("$1"); shift ;;
    esac
done

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
    printf '%s: not a git work tree\n' "${0##*/}" >&2; exit 69
}
cd "$repo_root"
registry="$repo_root/coupling.tsv"
[ -f "$registry" ] || { printf '%s: coupling.tsv not found\n' "${0##*/}" >&2; exit 69; }

failures=0
warnings=0
report_fail() { printf 'FAIL: %s\n' "$1" >&2; failures=$((failures + 1)); }
report_warn() { printf 'WARN: %s\n' "$1" >&2; warnings=$((warnings + 1)); }

# ---- the change set ----------------------------------------------------------
changed_file="$(mktemp "${TMPDIR:-/tmp}/blast-changed.XXXXXX")"
trap 'rm -f "$changed_file"' EXIT
# PORTABILITY(empty-array-setu)
if [ "${#paths[@]}" -gt 0 ]; then
    printf '%s\n' ${paths[@]+"${paths[@]}"} > "$changed_file"
else
    # Porcelain rather than diff, so untracked files count: a new registry that
    # was never added is exactly the case this pass exists to catch.
    git status --porcelain | awk '{ $1=""; sub(/^ +/, ""); if ($0 ~ / -> /) sub(/^.* -> /, ""); print }' \
        > "$changed_file"
fi
changed_count="$(grep -c . "$changed_file" || true)"
if [ "${changed_count:-0}" -eq 0 ]; then
    printf 'blast-radius: no changes to analyse\n'
    exit 0
fi
printf 'blast-radius: %s changed path(s), base %s\n' "$changed_count" "$base"

matches_any() { # <path> <glob>...
    local path="$1"; shift
    local glob
    for glob in "$@"; do
        # shellcheck disable=SC2254  # the glob is data and must stay unquoted.
        case "$path" in $glob) return 0 ;; esac
    done
    return 1
}

# ---- 1 + 4. the coupling registry -------------------------------------------
# One pass over the registry; a matched row either runs its check (freshness) or
# reports the consequence (contract).
ran_checks=""
while IFS="$(printf '\t')" read -r glob level consequence check; do
    case "${glob:-}" in ''|'#'*) continue ;; esac
    hit=""
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        if matches_any "$path" "$glob"; then hit="$path"; break; fi
    done < "$changed_file"
    [ -n "$hit" ] || continue
    if [ -n "${check:-}" ]; then
        case " $ran_checks " in *" $check "*) continue ;; esac
        ran_checks="$ran_checks $check"
        if out="$(eval "$check" 2>&1)"; then
            printf 'ok:   %s\n' "$consequence"
        else
            # Every script here reports as "name.sh: message"; prefer that line
            # over whatever a diff or a trace happened to print first.
            reason="$(printf '%s\n' "$out" | { grep -E '^[A-Za-z0-9._/-]+\.sh: ' || true; } | head -1)"
            [ -n "$reason" ] || reason="$(printf '%s\n' "$out" | { grep -E '[^[:space:]]' || true; } | tail -1)"
            [ -n "$reason" ] || reason="check failed with no output"
            report_fail "$consequence — \`$check\`: $reason"
        fi
    elif [ "$level" = fail ]; then
        report_fail "$hit: $consequence"
    else
        report_warn "$hit: $consequence"
    fi
done < "$registry"

# ---- 2. registry rows for new files -----------------------------------------
while IFS= read -r path; do
    [ -n "$path" ] || continue
    case "$path" in planning/*) ;; *) continue ;; esac
    # Only files that are new to the tree can be missing a row.
    git ls-files --error-unmatch "$path" >/dev/null 2>&1 && continue
    rows="$(grep -Fc "$path" planning/PACKAGE-MANIFEST.txt 2>/dev/null || true)"
    [ "${rows:-0}" -eq 0 ] || continue
    case "$path" in
        *.json)
            report_fail "$path: a new registry under planning/ with no PACKAGE-MANIFEST row will not ship, and a gate reading it through skill_root will die looking for it" ;;
        *)
            report_warn "$path: new under planning/ with no PACKAGE-MANIFEST row — ship it, or record why it is dev-only" ;;
    esac
done < "$changed_file"

# ---- 3. base drift ----------------------------------------------------------
if merge_base="$(git merge-base HEAD "$base" 2>/dev/null)"; then
    drifted=0
    while IFS= read -r path; do
        [ -n "$path" ] || continue
        [ -e "$path" ] || continue
        commits="$(git log --oneline "$merge_base..HEAD" -- "$path" 2>/dev/null | grep -c . || true)"
        [ "${commits:-0}" -gt 0 ] || continue
        drifted=$((drifted + 1))
        printf 'note: %s changed in %s commit(s) since %s\n' "$path" "$commits" "$base"
    done < "$changed_file"
    [ "$drifted" -eq 0 ] || printf 'note: rebase or verify those commits survive before merging\n'
else
    report_warn "cannot resolve a merge base with $base; drift not checked"
fi

printf 'blast-radius: %s failure(s), %s warning(s)\n' "$failures" "$warnings"
[ "$failures" -eq 0 ] || exit 1
