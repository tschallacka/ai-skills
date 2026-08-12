#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
map_file="$repo_dir/planning/V27-PACKAGE-MAP.tsv"
manifest_file="$repo_dir/planning/V27-PACKAGE-MANIFEST.txt"

test_v27_manifest_emission() {
    local emitted map_installable source destination owner gate collision source_only resolved
    emitted=$(mktemp)
    map_installable=$(mktemp)
    trap 'rm -f "$emitted" "$map_installable"' RETURN

    bash "$repo_dir/install.sh" --print-skill-files planning --format=tsv >"$emitted"
    cmp -s "$manifest_file" "$emitted"
    awk -F '\t' 'NR == 1 { next } $6 == "false" { print }' "$map_file" >"$map_installable"
    cmp -s "$manifest_file" "$map_installable"
    [ "$(wc -l < "$manifest_file")" -eq 52 ]

    while IFS=$'\t' read -r source destination owner gate collision source_only; do
        [ -n "$source" ] || continue
        [ "$source_only" = false ] || { printf 'source-only row in manifest: %s\n' "$source" >&2; return 1; }
        [ -n "$owner" ] && [ -n "$gate" ] && [ -n "$collision" ]
        resolved=$(realpath -m "$(bash "$repo_dir/install.sh" --resolve-source planning "$destination")")
        [ "$resolved" = "$(realpath -m "$repo_dir/$source")" ] || {
            printf 'source mismatch: %s -> %s (got %s)\n' "$source" "$destination" "$resolved" >&2
            return 1
        }
    done < "$manifest_file"

    ! grep -q 'brainstorm-limiting-context.v27.md' "$emitted"
    ! grep -q 'brainstorm-limiting-context-report.v27.md' "$emitted"
    printf '%s\n' 'test_v27_manifest_emission: PASS'
}

test_v27_manifest_emission "$@"
