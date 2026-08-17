#!/usr/bin/env bash
# Persona drift guard.
#
# Asserts the persona system stays in sync across three sources of truth:
#   - the registry (planning/scripts/role-context.sh ROLES=()),
#   - the scope docs each persona reads (planning/ROLES.md + role-context.sh
#     role_docs()),
#   - the shipped install set (planning/PACKAGE-MANIFEST.txt + the
#     install.sh skill_files() list).
#
# It fails (drift, no backwards compatibility) when:
#   - a registered persona has no voice in roles/VOICES.md,
#   - a voice key is not a registered persona,
#   - a scope doc a persona reads is not shipped,
#   - the installed reader cannot resolve a registered role's scope docs,
#   - a shipped file on the manifest is missing from disk.
#
# This is the consolidated drift guard (goal 05, W16); the narrower pre-shipping
# voice check lives in planning/tests/test-voice-artifact-drift.sh.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
registry="$root/scripts/role-context.sh"
voices="$root/roles/VOICES.md"
manifest="$root/PACKAGE-MANIFEST.txt"

fail=0
note_fail() { echo "persona drift: $1" >&2; fail=1; }

# 1. Registry ids — derive from role-context.sh --list (identity-free) so
#    adding a persona to ROLES=() is auto-checked rather than hand-mirrored.
registry_ids="$(bash "$registry" --list 2>/dev/null | awk '{print $1}' | tr '\n' ' ')"
[ -n "$registry_ids" ] || { echo "persona drift: could not read registry from $registry --list" >&2; exit 1; }
registry_ids="$(printf '%s\n' "$registry_ids" | sed 's/[[:space:]]*$//')"

# 2. Every registered persona must have a voice keyed by its id.
voices_ids="$(awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v} /^\|/ { rid=trim($2); gsub(/^`|`$/,"",rid); if (rid ~ /^[a-z]+$/) print rid }' "$voices")"
for id in $registry_ids; do
    echo "$voices_ids" | grep -qx "$id" || note_fail "registered persona $id has no voice in roles/VOICES.md"
done
for id in $voices_ids; do
    case " $registry_ids " in
        *" $id "*) ;;
        *) note_fail "unregistered voice key $id in roles/VOICES.md" ;;
    esac
done

# 3. Every scope doc a persona reads must exist on disk and be shipped.
#    Pull the authoritative per-role scope list from role-context.sh.
scope_errors=0
for id in $registry_ids; do
    docs="$(ROLE_ID=maintainer bash "$registry" --paths "$id" 2>/dev/null || true)"
    [ -n "$docs" ] || continue
    while IFS= read -r rel; do
        [ -n "$rel" ] || continue
        # on-disk existence
        [ -f "$root/$rel" ] || { note_fail "scope doc missing on disk: planning/$rel (persona $id)"; continue; }
        # shipped? (manifest lists planning/<rel> in the source column)
        grep -q "$(printf '%s' "planning/$rel	")" "$manifest" || note_fail "scope doc not shipped: planning/$rel (persona $id)"
    done <<< "$docs"
done

# 4. Every manifest entry must resolve on disk (no stale shipped path).
if [ -f "$manifest" ]; then
    while IFS=$'\t' read -r source _; do
        [ -n "$source" ] || continue
        [ -f "$root/${source#planning/}" ] || note_fail "manifest references missing file: $source"
    done < "$manifest"
fi

# 5. The new persona scripts and tests must be shipped.
for rel in scripts/role-context.sh scripts/monitor-read.sh scripts/supervision-frame.sh \
           roles/VOICES.md ROLES.md MAINTAINER-STYLE-CONTRACT.md \
           tests/test-supervision-frame.sh tests/test-voice-artifact-drift.sh; do
    grep -q "planning/$rel	" "$manifest" || note_fail "persona artifact not shipped: planning/$rel"
done

if [ "$fail" -eq 0 ]; then
    echo 'persona drift: PASS'
else
    echo 'persona drift: FAIL' >&2
    exit 1
fi
