#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
map_file="$repo_dir/planning/PACKAGE-MAP.tsv"
manifest_file="$repo_dir/planning/PACKAGE-MANIFEST.txt"

test_manifest_emission() {
    local emitted map_installable source destination owner gate collision source_only resolved manifest_count
    emitted=$(mktemp)
    map_installable=$(mktemp)
    trap 'rm -f "$emitted" "$map_installable"' RETURN

    # NOTE: --print-skill-files emits the manifest itself (install.sh cats
    # PACKAGE-MANIFEST.txt), so comparing emitted to the manifest would be a
    # tautology. The real contracts are (a) manifest == map installable rows
    # below, and (b) skill_files() == manifest destinations
    # (test_skill_files_matches_manifest).
    bash "$repo_dir/install.sh" --print-skill-files planning --format=tsv >"$emitted"
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
        resolved=$(realpath -m "$(bash "$repo_dir/install.sh" --resolve-source planning "$destination")")
        [ "$resolved" = "$(realpath -m "$repo_dir/$source")" ] || {
            printf 'source mismatch: %s -> %s (got %s)\n' "$source" "$destination" "$resolved" >&2
            return 1
        }
    done < "$manifest_file"

    # The source-only brainstorming docs are legitimate context content and
    # appear in the manifest; the installer emission must NOT carry them as
    # installable rows. The real guard is the manifest == map-installable cmp
    # above (map treats these as source_only=true and excludes them).
    printf '%s\n' 'test_manifest_emission: PASS'
}

# The install set (install.sh skill_files planning heredoc) must equal the
# manifest's destination list. skill_files() drives install_skill/cli_install;
# the manifest drives --print-skill-files and the map. If they diverge, an
# install ships an incomplete/broken skill while the manifest claims otherwise,
# so reconcile them here (durable guard; add the file to BOTH on change).
test_skill_files_matches_manifest() {
    local manifest_dests skill_files
    skill_files=$(mktemp)
    manifest_dests=$(mktemp)
    trap 'rm -f "$manifest_dests" "$skill_files"' RETURN

    # Extract the planning skill_files() heredoc destinations from install.sh.
    python3 - "$repo_dir/install.sh" "$skill_files" <<'PY'
import re, sys
src = open(sys.argv[1]).read()
m = re.search(r"skill_files\(\)\s*\{\s*case \"\$1\" in\s*planning\)\s*cat <<'EOF'\n(.*?)\nEOF", src, re.S)
if not m:
    sys.exit(1)
with open(sys.argv[2], "w") as out:
    for line in m.group(1).splitlines():
        line = line.strip()
        if line:
            out.write(line + "\n")
PY
    # Manifest destinations (column 2).
    awk -F '\t' 'NR >= 1 && $2 != "" && $2 != "destination" { print $2 }' "$manifest_file" | sort > "$manifest_dests"
    sort "$skill_files" -o "$skill_files"

    if ! cmp -s "$skill_files" "$manifest_dests"; then
        echo "skill_files() planning set does not match PACKAGE-MANIFEST.txt destinations:" >&2
        diff "$skill_files" "$manifest_dests" >&2 || true
        return 1
    fi
    printf '%s\n' 'test_skill_files_matches_manifest: PASS'
}

test_manifest_emission "$@"
test_skill_files_matches_manifest "$@"
