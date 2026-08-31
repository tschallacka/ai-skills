#!/usr/bin/env bash
# MODE: DEV
# test-installer-manifest — the planning ship manifest, the package map, and
# install.sh's install set must describe the same file list.
#
# Usage: test-installer-manifest.sh
#
# This file is itself shipped (it is registered in PACKAGE-MANIFEST.tsv), so it
# holds to the shipped-runtime dependency rule in CODE-STYLE.md §1: bash, POSIX
# coreutils, awk, sed, grep, git only. No python3.
set -euo pipefail
export LC_ALL=C
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
map_file="$repo_dir/planning/PACKAGE-MAP.tsv"
manifest_file="$repo_dir/planning/PACKAGE-MANIFEST.tsv"

# Normalise a path without requiring GNU `realpath -m` (absent on macOS).
# Both callers pass paths that exist, so resolving the parent is sufficient.
abs_path() {
    local path="$1" dir base
    dir="$(dirname "$path")"
    base="$(basename "$path")"
    printf '%s/%s\n' "$(cd "$dir" && pwd -P)" "$base"
}

deferred_artifact() {
    case "$1" in
        bin/*/plan-overview|bin/*/plan-overview.exe) return 0 ;;
        *) return 1 ;;
    esac
}

test_manifest_emission() {
    local emitted map_installable source destination owner gate collision source_only resolved manifest_count
    emitted=$(mktemp)
    map_installable=$(mktemp)
    trap 'rm -f "$emitted" "$map_installable"' RETURN

    # --print-skill-files cats PACKAGE-MANIFEST.tsv, so emitted-vs-manifest is a
    # tautology; the real contract is manifest == the map's installable rows.
    "$BASH" "$repo_dir/install.sh" --print-skill-files planning --format=tsv >"$emitted"
    awk -F '\t' 'NR == 1 { next } $6 == "false" { print }' "$map_file" >"$map_installable"
    cmp -s "$manifest_file" "$map_installable"
    # Derive the expected manifest row count from the map (it must equal the
    # installable set), so the count cannot drift independently of the package.
    manifest_count="$(wc -l < "$map_installable")"
    [ "$(wc -l < "$manifest_file")" -eq "$manifest_count" ]

    while IFS=$'\t' read -r source destination owner gate collision source_only; do
        [ -n "$source" ] || continue
        [ "$source_only" = false ] || { printf 'source-only row in manifest: %s\n' "$source" >&2; return 1; }
        [ -n "$owner" ] && [ -n "$gate" ] && [ -n "$collision" ]
        if [ ! -e "$repo_dir/$source" ]; then
            deferred_artifact "$destination" || {
                printf 'missing manifest source: %s\n' "$source" >&2
                return 1
            }
            continue
        fi
        resolved=$(abs_path "$("$BASH" "$repo_dir/install.sh" --resolve-source planning "$destination")")
        [ "$resolved" = "$(abs_path "$repo_dir/$source")" ] || {
            printf 'source mismatch: %s -> %s (got %s)\n' "$source" "$destination" "$resolved" >&2
            return 1
        }
    done < "$manifest_file"

    # Source-only rows must never reach the installable emission; the cmp above
    # is the guard, since the map excludes source_only=true.
    printf '%s\n' 'test_manifest_emission: PASS'
}

# skill_files() drives the actual install, the manifest drives --print-skill-files
# and the map; divergence ships a broken skill, so a new file goes in both.
test_skill_files_matches_manifest() {
    local manifest_dests skill_files
    skill_files=$(mktemp)
    manifest_dests=$(mktemp)
    trap 'rm -f "$manifest_dests" "$skill_files"' RETURN

    # Extract the planning skill_files() heredoc destinations from install.sh.
    # State machine over the same landmarks the previous python regex matched:
    # `skill_files()` -> `planning)` -> `cat <<'EOF'` -> lines -> `EOF`.
    awk '
        !in_func && /^[[:space:]]*skill_files\(\)/ { in_func = 1; next }
        in_func && !seen_planning && /^[[:space:]]*planning\)/ { seen_planning = 1; next }
        seen_planning && !capture && /cat[[:space:]]*<<'"'"'EOF'"'"'/ { capture = 1; next }
        capture && /^[[:space:]]*EOF[[:space:]]*$/ { exit }
        capture {
            line = $0
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
            if (line != "") print line
            found = 1
        }
        END { if (!found) exit 1 }
    ' "$repo_dir/install.sh" > "$skill_files"
    # Manifest destinations (column 2).
    awk -F '\t' 'NR >= 1 && $2 != "" && $2 != "destination" { print $2 }' "$manifest_file" | sort > "$manifest_dests"
    sort "$skill_files" -o "$skill_files"

    if ! cmp -s "$skill_files" "$manifest_dests"; then
        echo "skill_files() planning set does not match PACKAGE-MANIFEST.tsv destinations:" >&2
        diff "$skill_files" "$manifest_dests" >&2 || true
        return 1
    fi
    printf '%s\n' 'test_skill_files_matches_manifest: PASS'
}

test_manifest_emission "$@"
test_skill_files_matches_manifest "$@"
