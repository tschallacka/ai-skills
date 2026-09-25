#!/usr/bin/env bash
# MODE: DEV
# Persona drift guard.
#
# Asserts the persona system stays in sync across three sources of truth:
#   - the registry (planning/scripts/role-context.sh ROLES=()),
#   - the scope docs each persona reads (planning/ROLES.md + role-context.sh
#     role_docs()),
#   - the shipped install set (planning/PACKAGE-MANIFEST.tsv + the
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
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin


root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
registry="$root/scripts/role-context.sh"
voices="$root/roles/VOICES.md"
manifest="$root/PACKAGE-MANIFEST.tsv"

# The dev arm of skill_files(), for artifacts that are registered for delivery to
# a maintainer rather than to an end user.
dev_arm="$(mktemp "${TMPDIR:-/tmp}/persona-dev-arm.XXXXXX")"
trap 'rm -f "$dev_arm"' EXIT
(
    # shellcheck disable=SC1090
    source "$root/../installer/src/05-config.sh"
    # shellcheck disable=SC1090
    source "$root/../installer/src/50-manifest.sh"
    SOURCE_ROOT="$(cd "$root/.." && pwd)"
    skill_files planning dev
) > "$dev_arm"

note_fail() { echo "persona drift: $1" >&2; t_record "$1"; }

# 1. Registry ids — derive from role-context.sh --list (identity-free) so
#    adding a persona to ROLES=() is auto-checked rather than hand-mirrored.
registry_ids="$("$BASH" "$registry" --list 2>/dev/null | awk '{print $1}' | tr '\n' ' ')"
[ -n "$registry_ids" ] || { echo "persona drift: could not read registry from $registry --list" >&2; exit 1; }
registry_ids="$(printf '%s\n' "$registry_ids" | sed 's/[[:space:]]*$//')"

# 2. Every registered persona must have a voice keyed by its id.
voices_ids="$(awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v} /^\|/ { rid=trim($2); gsub(/^`|`$/,"",rid); if (rid ~ /^[a-z]+$/) print rid }' "$voices")"
voices_list=" $(printf '%s' "$voices_ids" | tr '\n' ' ') "
for id in $registry_ids; do
    case "$voices_list" in
        *" $id "*) ;;
        *) note_fail "registered persona $id has no voice in roles/VOICES.md" ;;
    esac
done
for id in $voices_ids; do
    case " $registry_ids " in
        *" $id "*) ;;
        *) note_fail "unregistered voice key $id in roles/VOICES.md" ;;
    esac
done

# 3. Every shipped .agents/profiles/<id>.json's own instructions field must
#    match a fresh role-context read (T148 goal 4). This replaces the old
#    live-scope-doc-resolution check: dispatch no longer resolves scope docs
#    at read time, it ships a build-time snapshot (generate-profile-content),
#    so what can now drift is that snapshot vs. its live sources, not the
#    resolution machinery itself.
generator=""
for candidate in "$root/../target/release/generate-profile-content" "$root/../target/debug/generate-profile-content"; do
    if [ -x "$candidate" ]; then
        generator="$candidate"
        break
    fi
done
if [ -z "$generator" ]; then
    echo "persona drift: SKIP the profile-freshness check -- no generate-profile-content binary found under target/{release,debug}/. Build one with:" >&2
    echo "    cargo build --release -p generate-profile-content" >&2
else
    profile_personas="benny chris christian christoph dana frank maintainer installer oracle eve"
    for id in $profile_personas; do
        case " $registry_ids " in
            *" $id "*) ;;
            *) note_fail "profile persona $id is not in the role-context registry"; continue ;;
        esac
        drift="$("$generator" "$id" --check 2>&1)" && continue
        note_fail "profile drifted from a fresh role-context read: $id ($drift)"
    done
fi

# 4. Every manifest entry must resolve on disk (no stale shipped path).
if [ -f "$manifest" ]; then
    while IFS=$'\t' read -r source _; do
        [ -n "$source" ] || continue
        if [ ! -f "$root/${source#planning/}" ]; then
            case "$source" in
                planning/bin/*) continue ;; # cross-target artifacts are CI outputs
                *) note_fail "manifest references missing file: $source" ;;
            esac
        fi
    done < "$manifest"
fi

# 5. Every persona artifact must be registered for delivery. The runtime ones go
# to the end user; the tests that pin them are a maintainer's, so they belong to
# the dev arm of skill_files() and are deliberately not in the ship manifest --
# what matters is that neither is left out of both.
for rel in scripts/role-context.sh scripts/monitor-read.sh scripts/supervision-frame.sh \
           roles/VOICES.md ROLES.md MAINTAINER-STYLE-CONTRACT.md; do
    grep -q "planning/$rel	" "$manifest" || note_fail "persona artifact not shipped: planning/$rel"
done
for rel in tests/test-supervision-frame.sh tests/test-voice-artifact-drift.sh; do
    [ -f "$root/$rel" ] \
        || note_fail "persona test is missing from the tree: planning/$rel"
    grep -qx "$rel" "$dev_arm" \
        || note_fail "persona test is in neither arm of skill_files(): planning/$rel"
done

if [ "$(t_failures)" -eq 0 ]; then
    echo 'persona drift: PASS'
else
    echo 'persona drift: FAIL' >&2
    exit 1
fi
