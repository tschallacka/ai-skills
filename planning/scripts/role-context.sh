#!/usr/bin/env bash
# MODE: PROD
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

# ─────────────────────────────────────────────────────────────────────────────
# Compiled-binary preference
# ─────────────────────────────────────────────────────────────────────────────
# See plan_exec_compiled_binary_if_present's own doc comment
# (planning/scripts/lib/core/plan_exec_compiled_binary_if_present.sh) for the
# exec-vs-fall-through mechanism.
rlc_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$rlc_script_dir/plan-core-lib.sh"
plan_exec_compiled_binary_if_present role-context "$rlc_script_dir" "$@"
unset rlc_script_dir

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
             if ($0 ~ /^(MODE|PACKAGE):/) next
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
        # T87: SKILL.md is now a generated index; alex needs the full content,
        # which lives in its generated parts.
        alex)       printf '%s\n' SKILL.md parts/part-1.md parts/part-2.md parts/part-3.md parts/part-4.md ROLES.md roles/planning.md roles/execution.md ;;
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

plan_die "role-context: no compiled binary found (checked AI_SKILLS_BIN_ROOT and the default bin dir); run ./setup-dev-env.sh to build it" 69
