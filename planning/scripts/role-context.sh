#!/usr/bin/env bash
# role-context.sh — role-gated context reader.
#
# Given a role id or canonical name, print (or list) the .md documents that
# single role needs, concatenated with provenance headers, so an agent gets a
# scoped payload instead of doing many file lookups or loading unrelated
# ("future-phase") knowledge.
#
# Output is BYTE-budgeted and paginated for non-interactive agents: run
#   role-context.sh <role-id|name> -p1     # first page  (-p|--page N)
#   role-context.sh <role-id|name> -p2     # next page (if "more: ..." is shown)
# Each page is a deterministic slice; agents never need an interactive TTY.
#   -l|--list  list roles (identity-free); --paths (maintainer) prints doc paths;
#   --page-size BYTES sets the per-page byte budget (default 12000).
#
# GATING: the script is identity-aware and FAILS CLOSED. Any content read
# requires a valid ROLE_ID resolving to a registered persona; an unset or
# unknown ROLE_ID is a hard refusal with a "FAIL-CLOSED identity" message, and
# the worker is denied a persona. Reads are further restricted to the caller's
# own role (reviewer family or the maintainer may read more). Only `--list` is
# identity-free (safe mode: reveals nothing but ids/names). `--paths` exposes
# the on-disk layout and is maintainer-only:
#   ROLE_ID=maintainer role-context.sh --paths <role-id|name>
# Shell gates are advisory (not a security boundary); the agent framework is
# what actually confines the process.
#
# Alongside the docs, each payload prefixes the role's voice/stance preamble
# from roles/VOICES.md (identity preamble; see voice_for). This script is the
# machine source of the persona registry (ROLES=()) and per-role scope
# (role_docs()); the ROLES.md persona matrix is a maintained mirror of that
# scope, and scope-doc shipping is enforced by test-persona-drift.sh. Sourcing
# this file (not executing it) defines only the registry + resolvers (sourcing
# guard), so callers reuse resolve_id without running the CLI main flow.
#
# Usage:
#   role-context.sh <role-id|canonical-name> [-p N] [--page-size BYTES]   # needs ROLE_ID
#   role-context.sh --list                                                 # open (safe mode)
#   ROLE_ID=maintainer role-context.sh --paths <role-id|name>              # maintainer-only
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

# Return the persona's voice/stance line from roles/VOICES.md, keyed by the
# canonical role id, for injection into the identity preamble. Emits nothing if
# the voice document is missing (the reader still fails closed on missing docs
# elsewhere; a missing voice is surfaced by the voice-artifact drift test).
voice_for() {
    local id="$1" file="$SKILL_DIR/roles/VOICES.md"
    [ -f "$file" ] || return 0
    awk -F'|' -v wanted="$id" '
        function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v}
        /^\|/ {
            rid=trim($2); gsub(/^`|`$/,"",rid)
            if (rid==wanted) { print trim($3); exit }
        }' "$file"
}

# Identity gate. True when `caller` may read context for `target`:
# its own role, the reviewer family, or any role if the caller is the
# maintainer (supervision). Names resolve to ids first.
can_access() {
    local caller="$1" target="$2"
    [ "$caller" = "$target" ] && return 0
    [ "$caller" = maintainer ] && return 0
    case "$caller" in
        chris|christian|christoph)
            case "$target" in
                chris|christian|christoph) return 0 ;;
            esac
            ;;
    esac
    return 1
}

# Sourcing guard: when this file is sourced (not executed), only the registry
# (ROLES=()) and the resolver functions are defined; the CLI main flow that
# parses args / renders context is skipped so the caller can reuse resolve_id
# without this script's main body running (and its exit / usage firing).
if [ "${BASH_SOURCE[0]}" != "$0" ]; then
    return 0
fi

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
        -h|--help) sed -n '1,43p' "$0"; exit 0 ;;
        -*) usage ;;
        *) [ -z "$ROLE" ] && ROLE="$1" || usage; shift ;;
    esac
done
# --list is identity-free and needs no role token; all other modes do.
if [ "$MODE" != list ]; then
    [ -n "$ROLE" ] || usage
    id="$(resolve_id "$ROLE")"
    [ "$id" = UNKNOWN ] && {
        printf 'role-context: unknown role or name: %s\n' "$ROLE" >&2
        printf '  valid: ' >&2; list_roles >&2
        exit 64
    }
else
    id=""
fi

# Identity gating. --list is open (id/name meta only). Any content read
# requires a valid ROLE_ID; unauthenticated callers get a list-only safe mode
# that reveals nothing else. --paths additionally requires the maintainer.
caller_id=""
if [ -n "${ROLE_ID:-}" ]; then
    caller_id="$(resolve_id "$ROLE_ID")"
    [ "$caller_id" = UNKNOWN ] && {
        printf 'role-context: FAIL-CLOSED identity: unknown ROLE_ID "%s". This worker has no persona and cannot continue; the coordinator must respawn it with a valid ROLE_ID.\n' "$ROLE_ID" >&2
        exit 64
    }
fi

case "$MODE" in
    list) list_roles; exit 0 ;;
    paths)
        [ "$caller_id" = maintainer ] || {
            printf 'role-context: --paths is maintainer-only; run with ROLE_ID=maintainer\n' >&2
            exit 64
        }
        role_docs "$id"
        exit 0
        ;;
    print) ;;
esac

[ -n "$caller_id" ] || {
    printf 'role-context: FAIL-CLOSED identity: no ROLE_ID set. This worker lacks a persona and cannot read scoped context; the coordinator must respawn it with a valid ROLE_ID (or use --list).\n' >&2
    exit 64
}
can_access "$caller_id" "$id" || {
    printf 'role-context: role %s may not read context for %s (own-role gate)\n' "$caller_id" "$id" >&2
    printf '  your role: %s; valid ids: ' "$caller_id" >&2; list_roles >&2
    exit 64
}

# Render the full payload once, then slice it into byte-budgeted pages.
name="$(canonical_name "$id")"
voice="$(voice_for "$id")"
tmp="$(mktemp)"; trap 'rm -f "$tmp"' EXIT
{
    printf '# Role context: %s (%s)\n\n' "$id" "$name"
    if [ -n "$voice" ]; then
        printf '# Voice (%s): %s\n\n' "$id" "$voice"
    fi
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
