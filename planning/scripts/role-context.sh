#!/usr/bin/env bash
# role-context.sh — role-gated context reader.
#
# Given a role id or canonical name, print (or list) the .md documents that
# single role needs, concatenated with provenance headers, so an agent gets a
# scoped payload instead of doing many file lookups or loading unrelated
# ("future-phase") knowledge.
#
# Output is BYTE-budgeted and paginated for non-interactive agents: run
#   role-context.sh <role-id|name> -p1     # first page
#   role-context.sh <role-id|name> -p2     # next page (if "more: ..." is shown)
# Each page is a deterministic slice; agents never need an interactive TTY.
#
# Usage:
#   role-context.sh <role-id|canonical-name> [-p N] [--page-size BYTES]
#   role-context.sh --paths <role-id|name>
#   role-context.sh --list
#   role-context.sh --help
#
# Accepts both the canonical id and the canonical name (case-insensitive),
# plus id/name aliases (e.g. "willie" or "maintainer", "pythia" or "oracle",
# "benny-02" -> "benny").

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILL_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Canonical registry: "id:name". Source of truth is the maintainer contract.
ROLES=(
    alex:Alex
    benny:Benny
    chris:Chris
    christian:Christian
    christoph:Christoph
    dana:Dana
    frank:Frank
    maintainer:Willie
    installer:Felix
    oracle:Pythia
    eve:Eve
)

usage() {
    sed -n '1,26p' "$0" >&2
    exit 64
}

resolve_id() {
    local token="$1" id name pair
    token="$(printf '%s' "$token" | tr '[:upper:]' '[:lower:]')"
    [[ "$token" =~ ^benny-?[0-9]*$ ]] && { printf 'benny\n'; return; }
    for pair in "${ROLES[@]}"; do
        id="${pair%%:*}"; name="${pair##*:}"
        if [ "$token" = "$id" ] || [ "$token" = "$(printf '%s' "$name" | tr '[:upper:]' '[:lower:]')" ]; then
            printf '%s\n' "$id"; return
        fi
    done
    printf 'UNKNOWN\n'
}

canonical_name() {
    local id="$1" pair
    for pair in "${ROLES[@]}"; do
        [ "${pair%%:*}" = "$id" ] && { printf '%s\n' "${pair##*:}"; return; }
    done
    printf '%s\n' "$id"
}

role_docs() {
    local id="$1"
    case "$id" in
        alex)       printf '%s\n' SKILL.md ROLES.md roles/planning.md roles/execution.md ;;
        benny)      printf '%s\n' ROLES.md roles/planning.md roles/execution.md ;;
        chris|christian|christoph)  printf '%s\n' ROLES.md roles/planning.md REVIEWER.md ;;
        dana)       printf '%s\n' ROLES.md roles/execution.md ;;
        frank)      printf '%s\n' ROLES.md roles/cleanup.md ;;
        maintainer) printf '%s\n' MAINTAINER-STYLE-CONTRACT.md ROLES.md roles/planning.md roles/execution.md roles/cleanup.md ;;
        installer)  printf '%s\n' ROLES.md MAINTAINER-STYLE-CONTRACT.md ;;
        oracle|eve) printf '%s\n' MAINTAINER-STYLE-CONTRACT.md ;;
        *)          printf '%s\n' ROLES.md ;;
    esac
}

list_roles() {
    local pair id name
    for pair in "${ROLES[@]}"; do
        id="${pair%%:*}"; name="${pair##*:}"
        printf '%-11s %s\n' "$id" "$name"
    done
}

PAGE=1
PAGE_BUDGET="${PAGE_BUDGET:-12000}"     # bytes per page
MODE=print
ROLE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -p|--page) [ "$#" -ge 2 ] || usage; PAGE="$2"; shift 2 ;;
        -p[0-9]*) PAGE="${1#-p}"; shift ;;
        --page-size) [ "$#" -ge 2 ] || usage; PAGE_BUDGET="$2"; shift 2 ;;
        --paths) MODE=paths; shift ;;
        --list|-l) MODE=list; shift ;;
        -h|--help) usage ;;
        -*) usage ;;
        *) [ -z "$ROLE" ] && ROLE="$1" || usage; shift ;;
    esac
done
[ -n "$ROLE" ] || usage

id="$(resolve_id "$ROLE")"
[ "$id" = UNKNOWN ] && {
    printf 'role-context: unknown role or name: %s\n' "$ROLE" >&2
    printf '  valid: ' >&2; list_roles >&2
    exit 64
}

case "$MODE" in
    list) list_roles; exit 0 ;;
    paths) role_docs "$id"; exit 0 ;;
esac

# Render the full payload once, then slice it into byte-budgeted pages.
name="$(canonical_name "$id")"
tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
{
    printf '# Role context: %s (%s)\n\n' "$id" "$name"
    for rel in $(role_docs "$id"); do
        file="$SKILL_DIR/$rel"
        if [ -f "$file" ]; then
            printf '\n===== %s =====\n' "$rel"
            cat "$file"
        else
            printf '\n# (missing: %s)\n' "$rel"
        fi
    done
} > "$tmp"

# Build page boundaries by cumulative byte budget.
declare -a start_line end_line
line=0; page=1; bytes=0; start_line[1]=1
while IFS= read -r l; do
    line=$((line + 1))
    len=$(( ${#l} + 1 ))
    if [ "$line" -gt 1 ] && [ $((bytes + len)) -gt "$PAGE_BUDGET" ]; then
        end_line[$page]=$((line - 1))
        page=$((page + 1))
        start_line[$page]="$line"
        bytes=0
    fi
    bytes=$((bytes + len))
done < "$tmp"
end_line[$page]="$line"
total_page="$page"

PAGE="${PAGE:-1}"
printf '# role-context %s (%s) — page %s/%s\n' "$id" "$name" "$PAGE" "$total_page"
if [ "$PAGE" -lt 1 ] || [ "$PAGE" -gt "$total_page" ]; then
    printf '# (page out of range; expected 1..%s)\n' "$total_page"
    exit 0
fi
sed -n "${start_line[$PAGE]},${end_line[$PAGE]}p" "$tmp"
if [ "$PAGE" -lt "$total_page" ]; then
    printf '\n# more: role-context.sh <%s|name> -p %s\n' "$id" "$((PAGE + 1))"
fi
