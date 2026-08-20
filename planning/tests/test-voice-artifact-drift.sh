#!/usr/bin/env bash
# MODE: DEV
# Voice-artifact drift test.
#
# Asserts the per-role voice document (planning/roles/VOICES.md) stays aligned
# with the canonical persona registry and is actually injected by
# role-context.sh into each persona's identity preamble. This is the
# pre-shipping voice drift guard: it fails when VOICES.md misses a registered
# ROLE_ID, has an unregistered key, is empty/over-budget for a persona, or is
# not served by role-context. The consolidated registry : shipped scope docs :
# shipped voice docs guard (including the installer manifest) lives in
# planning/tests/test-persona-drift.sh (goal 05, W16).

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
voices="$root/roles/VOICES.md"
reader="$root/scripts/role-context.sh"

[ -f "$voices" ] || { echo "voice artifact missing: $voices" >&2; exit 1; }

# Canonical registry ids — derive from role-context.sh --list (identity-free)
# so adding a persona to ROLES=() is auto-checked rather than hand-mirrored.
registry="$("$BASH" "$reader" --list 2>/dev/null | awk '{print $1}' | tr '\n' ' ')"
[ -n "$registry" ] || { echo "voice drift: could not read registry from $reader --list" >&2; exit 1; }

# IDs present in VOICES.md (table rows `| \`id\` | text |`).
present="$(awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v} /^\|/ { rid=trim($2); gsub(/^`|`$/,"",rid); if (rid ~ /^[a-z]+$/) print rid }' "$voices")"
[ -n "$present" ] || { echo "voice artifact has no keyed rows" >&2; exit 1; }

present_list=" $(printf '%s' "$present" | tr '\n' ' ') "
for id in $registry; do
    case "$present_list" in
        *" $id "*) ;;
        *)
            echo "voice drift: registered persona $id has no voice in $voices" >&2
            exit 1
            ;;
    esac
done

# Every voice key must be a registered persona (no orphan keys).
for id in $present; do
    case " $registry " in
        *" $id "*) ;;
        *) echo "voice drift: unregistered voice key $id in $voices" >&2; exit 1 ;;
    esac
done

# Each voice must be non-empty and byte-budgeted (<= 512 bytes), and never a
# biography (no past-tense bio markers is a weak heuristic; the budget is the
# hard gate).
awk -F'|' '
    function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v}
    /^\|/ {
        rid=trim($2); gsub(/^`|`$/,"",rid)
        if (rid ~ /^[a-z]+$/) {
            text=trim($3)
            if (length(text) == 0) { print "voice drift: empty voice for " rid > "/dev/stderr"; exit 1 }
            if (length(text) > 512) { print "voice drift: voice over budget for " rid " (" length(text) " bytes)" > "/dev/stderr"; exit 1 }
        }
    }' "$voices" || exit 1

# Every persona payload includes its voice via role-context (injection proof).
for id in $registry; do
    payload="$(ROLE_ID="$id" "$BASH" "$reader" "$id" -p1 2>/dev/null || true)"
    case "$payload" in
        *"# Voice ($id):"*) ;;
        *)
            echo "voice drift: role-context does not inject voice for $id" >&2
            exit 1
            ;;
    esac
done

echo 'voice-artifact drift: PASS'
