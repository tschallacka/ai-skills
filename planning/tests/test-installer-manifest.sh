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
# Load the generated installer helper so the platform-path contract is tested
# at the same implementation boundary used by install.sh.
# shellcheck disable=SC1090
source "$repo_dir/installer/src/50-manifest.sh"

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
        bin/*) return 0 ;; # cross-target artifacts are CI outputs
        scripts/*)
            case "$1" in *.sh) return 1 ;; esac
            return 0 ;; # extensionless Rust commands are build outputs
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
    cat >> "$skill_files" <<'EOF'
bin/x86_64-unknown-linux-musl/rjq
bin/aarch64-unknown-linux-musl/rjq
bin/x86_64-apple-darwin/rjq
bin/aarch64-apple-darwin/rjq
bin/x86_64-pc-windows-msvc/rjq.exe
EOF
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

test_windows_command_paths() {
    # The manifest deliberately names Rust commands without a suffix. The
    # installer must add the Windows suffix only to the physical path, while
    # leaving shell helpers and ordinary files untouched.
    uname() { printf '%s\n' Windows_NT; }
    [ "$(platform_relative_path planning scripts/plan-content)" = \
        scripts/plan-content.exe ]
    [ "$(platform_relative_path planning scripts/plan-content.sh)" = \
        scripts/plan-content.sh ]
    [ "$(platform_relative_path planning SKILL.md)" = SKILL.md ]
    unset -f uname
    printf '%s\n' 'test_windows_command_paths: PASS'
}

# Every assertion here reports what failed before exiting.
#
# It used to assert with bare `[ ... ]` under `set -e`, so a failure exited 1
# having printed nothing at all -- the run showed three sub-tests passing, then
# `FAIL (exit 1)` with no reason, on every platform. That is the whole of B170:
# four rows named shell helpers deleted when the register binaries landed, and
# the test could not say so. A test that cannot name its own failure costs more
# than the failure.
registry_fail() {
    printf 'installer-manifest: %s\n' "$1" >&2
    exit 1
}

registry_count_is() {
    local label="$1" actual="$2" want="$3"
    [ "$actual" -eq "$want" ] \
        || registry_fail "rust-migration.tsv has $actual $label rows, expected $want"
}

test_rust_migration_registry() {
    local runtime generator dev excluded path kind candidate crate
    runtime="$(awk -F '\t' '$2 == "runtime-binary" { n++ } END { print n + 0 }' \
        "$repo_dir/planning/rust-migration.tsv")"
    generator="$(awk -F '\t' '$2 == "build-generator" { n++ } END { print n + 0 }' \
        "$repo_dir/planning/rust-migration.tsv")"
    dev="$(awk -F '\t' '$2 == "dev-binary" { n++ } END { print n + 0 }' \
        "$repo_dir/planning/rust-migration.tsv")"
    excluded="$(awk -F '\t' '$2 == "runtime-binary" && $3 == "render-plans-board" { n++ } END { print n + 0 }' \
        "$repo_dir/planning/rust-migration.tsv")"
    registry_count_is runtime-binary "$runtime" 48
    registry_count_is build-generator "$generator" 1
    # One: register-rebuild.sh. The four register helpers that were here went
    # with their shell originals when the compiled `bugs` and `todo` replaced
    # them -- every other row's path is a live file, which is the invariant the
    # loop below enforces, and a completed migration cannot satisfy it.
    registry_count_is dev-binary "$dev" 1
    registry_count_is 'excluded render-plans-board' "$excluded" 1

    while IFS=$'\t' read -r path kind candidate _; do
        [ -n "$candidate" ] || continue
        case "$kind" in
            runtime-binary|build-generator|dev-binary)
                [ "$candidate" = render-plans-board ] && continue
                [ -f "$repo_dir/$path" ] \
                    || registry_fail "rust-migration.tsv names $path, which does not exist (candidate $candidate)"
                crate="$candidate"
                [ "$candidate" = overview-state ] && crate=plan-overview
                [ -f "$repo_dir/src/$crate/Cargo.toml" ] \
                    || registry_fail "row $path names crate $crate, but src/$crate/Cargo.toml does not exist"
                ;;
        esac
    done < "$repo_dir/planning/rust-migration.tsv"
    printf '%s\n' 'test_rust_migration_registry: PASS'
}

test_manifest_emission "$@"
test_skill_files_matches_manifest "$@"
test_windows_command_paths "$@"
test_rust_migration_registry "$@"
