#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# role-context.sh — role-gated context reader (persona registry + scope docs).
#
# Given a role id or canonical name, print the .md documents that single role
# needs, concatenated with provenance headers and the role's voice preamble, so
# an agent gets a scoped payload instead of loading unrelated knowledge.
#
# Usage:
#   role-context.sh <role-id|name> [-p N|--page N] [--page-size BYTES]
#   role-context.sh --list                    # -l; identity-free (safe mode)
#   ROLE_ID=maintainer role-context.sh --paths <role-id|name>  # maintainer-only
#   role-context.sh --help
#
# Output is BYTE-budgeted and paginated: -p2 (or -p 2) prints the next page when
# a "more: ..." footer is shown; --page-size sets the per-page byte budget
# (default 12000). Every page is a deterministic slice; no TTY is needed.
#
# GATING: identity-aware and FAILS CLOSED. Any content read requires a ROLE_ID
# resolving to a registered persona; reads are restricted to the caller's own
# role (reviewer family mutual, maintainer may read all). --list is open.

# `usage` prints lines 1-20 verbatim, so the docblock above MUST stay 20 lines.
# `ROLES`, `resolve_id`, `canonical_name`, `role_docs`, `list_roles`, `voice_for`
# and `can_access` are this file's sourced public surface; do not rename them.

set -euo pipefail
# LC_ALL=C also pins the page accounting to BYTES: under a UTF-8 locale ${#str}
# counts characters, which mis-bills the byte budget below for the multi-byte
# glyphs (§ 💤 ⏳ ✅ —) these documents are full of. Bytes everywhere, one unit.
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
skill_dir="$(cd "$script_dir/.." && pwd)"

# Canonical registry: "id:name". Source of truth is the maintainer contract.
# Public when this file is sourced — see the DUAL-NATURED note above.
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
    local rc="${1:-64}"
    awk 'NR == 1 { next }
         /^#/ {
             sub(/^#[[:space:]]?/, "")
             if ($0 ~ /^----[[:space:]]*(quoted:|end quoted)/) next
             print; next
         }
         { exit }' "$0"
    exit "$rc"
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

# Emits nothing when the voice document is missing: this lookup must not be the
# thing that fails closed, because a missing voice is a drift-test finding
# rather than a read refusal.
voice_for() {
    local id="$1" file="$skill_dir/roles/VOICES.md"
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

# Sourcing guard: sourced, this file must define only the registry and the
# resolver functions. Letting the CLI main flow run would fire its arg parsing,
# usage and exit inside the caller.
if [ "${BASH_SOURCE[0]}" != "$0" ]; then
    return 0
fi

req_page=1
page_budget="${PAGE_BUDGET:-12000}"     # bytes per page
mode=print
role=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -p|--page) [ "$#" -ge 2 ] || usage; req_page="$2"; shift 2 ;;
        -p[0-9]*) req_page="${1#-p}"; shift ;;
        --page-size) [ "$#" -ge 2 ] || usage; page_budget="$2"; shift 2 ;;
        --paths) mode=paths; shift ;;
        --list|-l) mode=list; shift ;;
        -h|--help) usage 0 ;;
        --) shift; break ;;
        -*) usage ;;
        *)
            [ -z "$role" ] || usage
            role="$1"
            shift
            ;;
    esac
done
# --list is identity-free and needs no role token; all other modes do.
if [ "$mode" != list ]; then
    [ -n "$role" ] || usage
    id="$(resolve_id "$role")"
    [ "$id" = UNKNOWN ] && {
        printf 'role-context: unknown role or name: %s\n' "$role" >&2
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

case "$mode" in
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
tmp="$(mktemp "${TMPDIR:-/tmp}/role-context.XXXXXX")"; trap 'rm -f "$tmp"' EXIT
{
    printf '# Role context: %s (%s)\n\n' "$id" "$name"
    if [ -n "$voice" ]; then
        printf '# Voice (%s): %s\n\n' "$id" "$voice"
    fi
    for rel in $(role_docs "$id"); do
        file="$skill_dir/$rel"
        if [ -f "$file" ]; then
            printf '\n===== %s =====\n' "$rel"
            cat "$file"
        else
            printf '\n# (missing: %s)\n' "$rel"
        fi
    done
} > "$tmp"

# Build page boundaries by cumulative byte budget. ALL arithmetic here is in
# BYTES: LC_ALL=C above makes ${#l} a byte count, and the +1 is the newline
# `read` stripped, so the total matches what `wc -c` would report.
declare -a start_line end_line
line=0; page=1; bytes=0; start_line[1]=1
while IFS= read -r l; do
    line=$((line + 1))
    len=$(( ${#l} + 1 ))
    if [ "$line" -gt 1 ] && [ $((bytes + len)) -gt "$page_budget" ]; then
        end_line[page]=$((line - 1))
        page=$((page + 1))
        start_line[page]="$line"
        bytes=0
    fi
    bytes=$((bytes + len))
done < "$tmp"
end_line[page]="$line"
total_page="$page"

printf '# role-context %s (%s) — page %s/%s\n' "$id" "$name" "$req_page" "$total_page"
if [ "$req_page" -lt 1 ] || [ "$req_page" -gt "$total_page" ]; then
    printf '# (page out of range; expected 1..%s)\n' "$total_page"
    exit 0
fi
sed -n "${start_line[$req_page]},${end_line[$req_page]}p" "$tmp"
if [ "$req_page" -lt "$total_page" ]; then
    printf '\n# more: role-context.sh <%s|name> -p %s\n' "$id" "$((req_page + 1))"
fi
